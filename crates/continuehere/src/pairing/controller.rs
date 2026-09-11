use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex, MutexGuard, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use uuid::Uuid;

use crate::{
    devices::DeviceIdentityCapability,
    models::{DeviceId, LocalDeviceIdentity, ProtocolVersion},
    security::SecurityCapability,
    transport::{
        PairingConnectionCapability, PairingHello, PairingMessage, PairingTransportCapability,
        TransportError,
    },
};

use super::{
    PairingError, PairingFailure, PairingMode, PairingRole, PairingSession, PairingSessionChange,
    PairingSessionChangedEvent, PairingSessionId, PairingState, PairingVerification, TrustMutation,
    TrustedDevice, TrustedDeviceChange, TrustedDeviceChangedEvent, TrustedDeviceRegistry,
};

mod session;

use session::run_session;

const APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);
const SESSION_TIMEOUT: Duration = Duration::from_secs(120);
const APPROVAL_POLL_INTERVAL: Duration = Duration::from_millis(200);
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(200);
const MAX_ACTIVE_SESSIONS: usize = 8;

pub(crate) struct PairingController {
    state: Mutex<ControllerState>,
    trusted: TrustedDeviceRegistry,
    device_identity: DeviceIdentityCapability,
    security: SecurityCapability,
    transport: PairingTransportCapability,
    connection: PairingConnectionCapability,
    session_changed: PairingSessionChangedEvent,
    trusted_changed: TrustedDeviceChangedEvent,
}

struct ControllerState {
    running: bool,
    sessions: BTreeMap<PairingSessionId, PairingSession>,
    operations: HashMap<String, ActiveOperation>,
    orphan_workers: Vec<JoinHandle<()>>,
}

struct ActiveOperation {
    session_id: PairingSessionId,
    commands: mpsc::Sender<SessionCommand>,
    decision_sent: bool,
    worker: Option<JoinHandle<()>>,
    owns_listener: bool,
}

enum SessionCommand {
    Approve,
    Reject,
    Cancel,
}

struct VerifiedPeer {
    hello: PairingHello,
    fingerprint: [u8; 32],
    connection_endpoint: crate::discovery::DiscoveryEndpoint,
}

impl PairingController {
    pub(crate) fn new(
        trusted: TrustedDeviceRegistry,
        device_identity: DeviceIdentityCapability,
        security: SecurityCapability,
        transport: PairingTransportCapability,
        connection: PairingConnectionCapability,
        session_changed: PairingSessionChangedEvent,
        trusted_changed: TrustedDeviceChangedEvent,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(ControllerState {
                running: false,
                sessions: BTreeMap::new(),
                operations: HashMap::new(),
                orphan_workers: Vec::new(),
            }),
            trusted,
            device_identity,
            security,
            transport,
            connection,
            session_changed,
            trusted_changed,
        })
    }

    pub(crate) fn start(&self) -> Result<(), PairingError> {
        self.trusted.load()?;
        lock(&self.state)?.running = true;
        Ok(())
    }

    pub(crate) fn is_running(&self) -> bool {
        lock(&self.state)
            .map(|state| state.running)
            .unwrap_or(false)
    }

    pub(crate) fn stop(&self) -> Result<(), PairingError> {
        let (removed, workers, listener_count) = {
            let mut state = lock(&self.state)?;
            state.running = false;
            for operation in state.operations.values() {
                let _send_result = operation.commands.send(SessionCommand::Cancel);
            }
            let mut workers = state
                .operations
                .values_mut()
                .filter_map(|operation| operation.worker.take())
                .collect::<Vec<_>>();
            let listener_count = state
                .operations
                .values()
                .filter(|operation| operation.owns_listener)
                .count();
            workers.append(&mut state.orphan_workers);
            state.operations.clear();
            let removed = std::mem::take(&mut state.sessions)
                .into_values()
                .collect::<Vec<_>>();
            (removed, workers, listener_count)
        };
        for _ in 0..listener_count {
            let _release_result = self.transport.release_listener();
        }
        for session in removed {
            self.session_changed
                .publish(PairingSessionChange::Removed(session));
        }
        for worker in workers {
            worker.join().map_err(|_| PairingError::WorkerStopFailed)?;
        }
        Ok(())
    }

    pub(crate) fn begin(
        self: &Arc<Self>,
        handle_identifier: String,
        mode: PairingMode,
    ) -> Result<(), PairingError> {
        let role = match &mode {
            PairingMode::Initiate(_) => PairingRole::Initiator,
            PairingMode::Receive => PairingRole::Receiver,
        };
        {
            let state = lock(&self.state)?;
            if !state.running {
                return Err(PairingError::ManagerUnavailable);
            }
            if state.operations.len() >= MAX_ACTIVE_SESSIONS {
                return Err(PairingError::SessionLimit);
            }
        }
        let owns_listener = matches!(&mode, PairingMode::Receive);
        if owns_listener {
            self.transport
                .acquire_listener()
                .map_err(|_| PairingError::ManagerUnavailable)?;
        }
        let session_id = PairingSessionId::new(Uuid::new_v4().hyphenated().to_string());
        let session = PairingSession::new(session_id.clone(), role, PairingState::Connecting);
        let (sender, receiver) = mpsc::channel();
        let insert_result = {
            let mut state = lock(&self.state)?;
            if !state.running {
                Err(PairingError::ManagerUnavailable)
            } else if state.operations.len() >= MAX_ACTIVE_SESSIONS {
                Err(PairingError::SessionLimit)
            } else {
                state.sessions.insert(session_id.clone(), session.clone());
                state.operations.insert(
                    handle_identifier.clone(),
                    ActiveOperation {
                        session_id: session_id.clone(),
                        commands: sender,
                        decision_sent: false,
                        worker: None,
                        owns_listener,
                    },
                );
                Ok(())
            }
        };
        if let Err(error) = insert_result {
            if owns_listener {
                let _release_result = self.transport.release_listener();
            }
            return Err(error);
        }
        self.session_changed
            .publish(PairingSessionChange::Added(session));

        let controller = Arc::clone(self);
        let spawn_result = thread::Builder::new()
            .name("continuehere-pairing-session".to_owned())
            .spawn(move || {
                run_session(controller, session_id, mode, receiver);
            });
        let worker = match spawn_result {
            Ok(worker) => worker,
            Err(_) => {
                self.remove(&handle_identifier);
                return Err(PairingError::CommandChannelUnavailable);
            }
        };
        let mut state = lock(&self.state)?;
        match state.operations.get_mut(&handle_identifier) {
            Some(operation) => operation.worker = Some(worker),
            None => state.orphan_workers.push(worker),
        }
        Ok(())
    }

    pub(crate) fn approve(&self, handle_identifier: &str) -> Result<(), PairingError> {
        self.send_decision(handle_identifier, SessionCommand::Approve)
    }

    pub(crate) fn reject(&self, handle_identifier: &str) -> Result<(), PairingError> {
        self.send_decision(handle_identifier, SessionCommand::Reject)
    }

    pub(crate) fn cancel(&self, handle_identifier: &str) {
        let sender = lock(&self.state).ok().and_then(|state| {
            state
                .operations
                .get(handle_identifier)
                .map(|value| value.commands.clone())
        });
        if let Some(sender) = sender {
            let _send_result = sender.send(SessionCommand::Cancel);
        }
    }

    pub(crate) fn remove(&self, handle_identifier: &str) {
        let removed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let Some(mut operation) = state.operations.remove(handle_identifier) else {
                return;
            };
            if let Some(worker) = operation.worker.take() {
                state.orphan_workers.push(worker);
            }
            let removed = state.sessions.remove(&operation.session_id);
            (removed, operation.owns_listener)
        };
        if removed.1 {
            let _release_result = self.transport.release_listener();
        }
        if let Some(session) = removed.0 {
            self.session_changed
                .publish(PairingSessionChange::Removed(session));
        }
    }

    pub(crate) fn session_for_handle(&self, handle_identifier: &str) -> Option<PairingSession> {
        let state = lock(&self.state).ok()?;
        let operation = state.operations.get(handle_identifier)?;
        state.sessions.get(&operation.session_id).cloned()
    }

    pub(crate) fn sessions(&self) -> Vec<PairingSession> {
        match lock(&self.state) {
            Ok(state) => state.sessions.values().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    fn has_session(&self, session_id: &PairingSessionId) -> bool {
        lock(&self.state)
            .map(|state| state.sessions.contains_key(session_id))
            .unwrap_or(false)
    }

    pub(crate) fn trusted_devices(&self) -> Vec<TrustedDevice> {
        self.trusted.devices()
    }

    pub(crate) fn remove_trusted_device(&self, device_id: &DeviceId) -> Result<(), PairingError> {
        let removed = self.trusted.remove(device_id)?;
        self.trusted_changed
            .publish(TrustedDeviceChange::Removed(removed));
        Ok(())
    }

    fn send_decision(
        &self,
        handle_identifier: &str,
        command: SessionCommand,
    ) -> Result<(), PairingError> {
        let sender = {
            let mut state = lock(&self.state)?;
            let operation = state
                .operations
                .get(handle_identifier)
                .ok_or(PairingError::SessionNotFound)?;
            let session_id = operation.session_id.clone();
            let decision_sent = operation.decision_sent;
            let sender = operation.commands.clone();
            let session = state
                .sessions
                .get(&session_id)
                .ok_or(PairingError::SessionNotFound)?;
            if session.state() != PairingState::AwaitingVerification || decision_sent {
                return Err(PairingError::InvalidSessionState);
            }
            state
                .operations
                .get_mut(handle_identifier)
                .ok_or(PairingError::SessionNotFound)?
                .decision_sent = true;
            sender
        };
        sender
            .send(command)
            .map_err(|_| PairingError::CommandChannelUnavailable)
    }

    fn local_identity(&self) -> Result<LocalDeviceIdentity, PairingFailure> {
        self.device_identity
            .identity()
            .ok_or(PairingFailure::Internal)
    }

    fn update_state(&self, session_id: &PairingSessionId, new_state: PairingState) {
        self.update_session(session_id, |session| session.set_state(new_state));
    }

    fn update_peer(
        &self,
        session_id: &PairingSessionId,
        peer: &VerifiedPeer,
        verification: [u8; 32],
    ) {
        self.update_session(session_id, |session| {
            session.set_peer(
                peer.hello.device_id().clone(),
                peer.hello.display_name().to_owned(),
                peer.connection_endpoint.clone(),
            );
            session.set_verification(PairingVerification::new(verification));
        });
    }

    fn fail(&self, session_id: &PairingSessionId, failure: PairingFailure) {
        self.update_session(session_id, |session| session.fail(failure));
    }

    fn update_session(
        &self,
        session_id: &PairingSessionId,
        update: impl FnOnce(&mut PairingSession),
    ) {
        let updated = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let Some(session) = state.sessions.get_mut(session_id) else {
                return;
            };
            update(session);
            session.clone()
        };
        self.session_changed
            .publish(PairingSessionChange::Updated(updated));
    }

    fn persist_trust(&self, peer: &VerifiedPeer) -> Result<TrustMutation, PairingError> {
        self.trusted.persist(TrustedDevice::new(
            peer.hello.device_id().clone(),
            peer.fingerprint,
            peer.hello.display_name().to_owned(),
            peer.hello.platform(),
        ))
    }

    fn remember_peer_endpoint(&self, peer: &VerifiedPeer) {
        let _remember_result = self
            .connection
            .remember_verified_endpoint(peer.hello.device_id(), &peer.connection_endpoint);
    }

    fn rollback_unconfirmed_trust(&self, persisted: &TrustMutation) {
        self.trusted.rollback(persisted);
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, PairingError> {
    value
        .lock()
        .map_err(|_| PairingError::HandleSynchronizationFailed)
}

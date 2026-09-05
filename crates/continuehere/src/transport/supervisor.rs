use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    net::IpAddr,
    sync::{Arc, Mutex, MutexGuard, mpsc as standard_mpsc},
    time::{Duration, Instant},
};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    task::{JoinHandle, JoinSet},
    time::{MissedTickBehavior, interval},
};
use uuid::Uuid;

use crate::{
    discovery::DiscoveryEndpoint,
    models::{Capability, DeviceId, LocalDeviceIdentity},
    pairing::{TrustedDevice, TrustedPeerLookup},
    security::{CryptographicIdentity, SecurityCapability},
};

use super::{
    AuthenticatedConnection, ConnectionChange, ConnectionChangedEvent, ConnectionDirection,
    HandoffDisposition, HandoffTransportCapability, HandoffTransportPayload, InboundHandoff,
    InboundTransfer, TransferDisposition, TransferTransportCapability, TransferTransportMessage,
    TransportError,
    authenticated::{AuthenticatedChannel, AuthenticatedReader},
    protocol::{ApplicationHello, MAX_CONTROL_FRAME_SIZE, ProtocolEnvelope, ProtocolMessage},
};

const COMMAND_QUEUE_LIMIT: usize = 64;
const CONNECTION_COMMAND_LIMIT: usize = 32;
const WORKER_EVENT_LIMIT: usize = 64;
const MAX_CONNECTIONS: usize = 16;
const MAX_PENDING_HANDSHAKES: usize = 8;
const INCOMING_RATE_WINDOW: Duration = Duration::from_secs(60);
const MAX_INCOMING_PER_ADDRESS: usize = 10;
const MAX_RATE_LIMIT_ADDRESSES: usize = 256;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const TRANSFER_OFFER_TIMEOUT: Duration = Duration::from_secs(65);

pub(crate) type ConnectionStore = Arc<Mutex<BTreeMap<DeviceId, AuthenticatedConnection>>>;

pub(crate) struct SupervisorRuntime {
    commands: mpsc::Sender<SupervisorCommand>,
    worker: JoinHandle<()>,
}

pub(crate) enum SupervisorCommand {
    Connect {
        device_id: DeviceId,
        endpoint: DiscoveryEndpoint,
        response: oneshot::Sender<Result<AuthenticatedConnection, TransportError>>,
    },
    Disconnect {
        device_id: DeviceId,
        response: oneshot::Sender<Result<(), TransportError>>,
    },
    Probe {
        device_id: DeviceId,
        response: oneshot::Sender<Result<(), TransportError>>,
    },
    CheckLocalVideoSupport {
        device_id: DeviceId,
        response: standard_mpsc::Sender<Result<(), TransportError>>,
    },
    SendHandoff {
        device_id: DeviceId,
        handoff_id: [u8; 16],
        payload: HandoffTransportPayload,
        response: standard_mpsc::Sender<Result<HandoffDisposition, TransportError>>,
    },
    SendTransfer {
        device_id: DeviceId,
        transfer_id: [u8; 16],
        message: TransferTransportMessage,
        response: standard_mpsc::Sender<Result<TransferDisposition, TransportError>>,
    },
    Shutdown {
        response: oneshot::Sender<()>,
    },
}

struct ActiveConnection {
    runtime_id: Uuid,
    snapshot: AuthenticatedConnection,
    fingerprint: [u8; 32],
    commands: mpsc::Sender<ConnectionCommand>,
}

enum ConnectionCommand {
    Probe(oneshot::Sender<Result<(), TransportError>>),
    SendHandoff {
        handoff_id: [u8; 16],
        payload: HandoffTransportPayload,
        response: standard_mpsc::Sender<Result<HandoffDisposition, TransportError>>,
    },
    SendTransfer {
        transfer_id: [u8; 16],
        message: TransferTransportMessage,
        response: standard_mpsc::Sender<Result<TransferDisposition, TransportError>>,
    },
    Close,
}

enum WorkerEvent {
    Established {
        runtime_id: Uuid,
        snapshot: AuthenticatedConnection,
        fingerprint: [u8; 32],
        commands: mpsc::Sender<ConnectionCommand>,
        accepted: oneshot::Sender<bool>,
        connect_response: Option<oneshot::Sender<Result<AuthenticatedConnection, TransportError>>>,
        incoming: bool,
    },
    Failed {
        device_id: Option<DeviceId>,
        error: TransportError,
        connect_response: Option<oneshot::Sender<Result<AuthenticatedConnection, TransportError>>>,
        incoming: bool,
    },
    Closed {
        runtime_id: Uuid,
        device_id: DeviceId,
    },
}

struct EstablishedChannel {
    channel: AuthenticatedChannel,
    snapshot: AuthenticatedConnection,
    fingerprint: [u8; 32],
    maximum_frame_size: usize,
    maximum_in_flight_requests: usize,
    idle_timeout: Duration,
    handoff: HandoffTransportCapability,
    transfer: TransferTransportCapability,
}

struct PendingRequest {
    started: Instant,
    timeout: Duration,
    operation: PendingOperation,
}

enum PendingOperation {
    Probe {
        nonce: u64,
        response: oneshot::Sender<Result<(), TransportError>>,
    },
    Handoff {
        handoff_id: [u8; 16],
        response: standard_mpsc::Sender<Result<HandoffDisposition, TransportError>>,
    },
    Transfer {
        transfer_id: [u8; 16],
        response: standard_mpsc::Sender<Result<TransferDisposition, TransportError>>,
    },
}

struct InboundTransferResult {
    request_id: u64,
    transfer_id: [u8; 16],
    disposition: TransferDisposition,
}

#[derive(Clone)]
struct WorkerContext {
    local_identity: LocalDeviceIdentity,
    security: SecurityCapability,
    trusted_peers: TrustedPeerLookup,
    handoff: HandoffTransportCapability,
    transfer: TransferTransportCapability,
    events: mpsc::Sender<WorkerEvent>,
}

pub(super) struct SupervisorContext {
    local_identity: LocalDeviceIdentity,
    security: SecurityCapability,
    trusted_peers: TrustedPeerLookup,
    handoff: HandoffTransportCapability,
    transfer: TransferTransportCapability,
    connections: ConnectionStore,
    changed: ConnectionChangedEvent,
}

impl SupervisorContext {
    pub(super) fn new(
        local_identity: LocalDeviceIdentity,
        security: SecurityCapability,
        trusted_peers: TrustedPeerLookup,
        handoff: HandoffTransportCapability,
        transfer: TransferTransportCapability,
        connections: ConnectionStore,
        changed: ConnectionChangedEvent,
    ) -> Self {
        Self {
            local_identity,
            security,
            trusted_peers,
            handoff,
            transfer,
            connections,
            changed,
        }
    }
}

impl SupervisorRuntime {
    pub(crate) fn start(listener: TcpListener, context: SupervisorContext) -> Self {
        let (commands, command_receiver) = mpsc::channel(COMMAND_QUEUE_LIMIT);
        let worker = tokio::spawn(run_supervisor(listener, command_receiver, context));
        Self { commands, worker }
    }

    pub(crate) fn commands(&self) -> mpsc::Sender<SupervisorCommand> {
        self.commands.clone()
    }

    pub(crate) async fn stop(self) -> Result<(), TransportError> {
        let (response, completed) = oneshot::channel();
        self.commands
            .send(SupervisorCommand::Shutdown { response })
            .await
            .map_err(|_| TransportError::CommandUnavailable)?;
        completed
            .await
            .map_err(|_| TransportError::CommandUnavailable)?;
        self.worker
            .await
            .map_err(|_| TransportError::WorkerStopFailed)
    }
}

async fn run_supervisor(
    listener: TcpListener,
    mut commands: mpsc::Receiver<SupervisorCommand>,
    context: SupervisorContext,
) {
    let SupervisorContext {
        local_identity,
        security,
        trusted_peers,
        handoff,
        transfer,
        connections,
        changed,
    } = context;
    let (worker_events, mut events) = mpsc::channel(WORKER_EVENT_LIMIT);
    let mut tasks = JoinSet::new();
    let mut active = BTreeMap::<DeviceId, ActiveConnection>::new();
    let mut pending_outgoing = BTreeSet::new();
    let mut pending_incoming = 0_usize;
    let mut incoming_attempts = HashMap::new();
    let mut trust_revision = trusted_peers.revision();
    let worker_context = WorkerContext {
        local_identity: local_identity.clone(),
        security: security.clone(),
        trusted_peers: trusted_peers.clone(),
        handoff,
        transfer,
        events: worker_events.clone(),
    };

    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else {
                    break;
                };
                if handle_command(
                    command,
                    &mut active,
                    &mut pending_outgoing,
                    &mut tasks,
                    &worker_context,
                    &connections,
                    &changed,
                ).await {
                    break;
                }
            }
            accepted = listener.accept(), if pending_incoming < MAX_PENDING_HANDSHAKES => {
                if let Ok((stream, address)) = accepted
                    && active.len() + pending_incoming < MAX_CONNECTIONS
                    && allow_incoming(&mut incoming_attempts, address.ip())
                {
                    pending_incoming += 1;
                    spawn_incoming(
                        &mut tasks,
                        stream,
                        worker_context.clone(),
                    );
                }
            }
            event = events.recv() => {
                if let Some(event) = event {
                    handle_worker_event(
                        event,
                        &local_identity,
                        &mut active,
                        &mut pending_outgoing,
                        &mut pending_incoming,
                        &connections,
                        &changed,
                    );
                }
            }
            revision = trust_revision.changed() => {
                if revision.is_ok() {
                    remove_untrusted_connections(
                        &trusted_peers,
                        &mut active,
                        &connections,
                        &changed,
                    );
                }
            }
            _ = tasks.join_next(), if !tasks.is_empty() => {}
        }
    }

    for connection in active.values() {
        let _ = connection.commands.try_send(ConnectionCommand::Close);
    }
    tasks.shutdown().await;
    let removed = active
        .into_values()
        .map(|connection| connection.snapshot)
        .collect::<Vec<_>>();
    replace_connections(&connections, BTreeMap::new());
    for snapshot in removed {
        changed.publish(ConnectionChange::Removed(snapshot));
    }
}

async fn handle_command(
    command: SupervisorCommand,
    active: &mut BTreeMap<DeviceId, ActiveConnection>,
    pending_outgoing: &mut BTreeSet<DeviceId>,
    tasks: &mut JoinSet<()>,
    worker_context: &WorkerContext,
    connections: &ConnectionStore,
    changed: &ConnectionChangedEvent,
) -> bool {
    match command {
        SupervisorCommand::Connect {
            device_id,
            endpoint,
            response,
        } => {
            if active.contains_key(&device_id) || pending_outgoing.contains(&device_id) {
                let _ = response.send(Err(TransportError::AlreadyConnected));
                return false;
            }
            if active.len() + pending_outgoing.len() >= MAX_CONNECTIONS {
                let _ = response.send(Err(TransportError::ConnectionLimit));
                return false;
            }
            let trusted_peer = match worker_context.trusted_peers.get(&device_id) {
                Ok(Some(peer)) => peer,
                Ok(None) => {
                    let _ = response.send(Err(TransportError::UntrustedPeer));
                    return false;
                }
                Err(_) => {
                    let _ = response.send(Err(TransportError::TrustUnavailable));
                    return false;
                }
            };
            pending_outgoing.insert(device_id.clone());
            spawn_outgoing(
                tasks,
                endpoint,
                trusted_peer,
                worker_context.clone(),
                response,
            );
        }
        SupervisorCommand::Disconnect {
            device_id,
            response,
        } => {
            let Some(connection) = active.remove(&device_id) else {
                let _ = response.send(Err(TransportError::NotConnected));
                return false;
            };
            let _ = connection.commands.try_send(ConnectionCommand::Close);
            remove_connection_snapshot(connections, &device_id);
            changed.publish(ConnectionChange::Removed(connection.snapshot));
            let _ = response.send(Ok(()));
        }
        SupervisorCommand::Probe {
            device_id,
            response,
        } => {
            let Some(connection) = active.get(&device_id) else {
                let _ = response.send(Err(TransportError::NotConnected));
                return false;
            };
            if connection
                .commands
                .try_send(ConnectionCommand::Probe(response))
                .is_err()
            {
                return false;
            }
        }
        SupervisorCommand::CheckLocalVideoSupport {
            device_id,
            response,
        } => {
            let result = match active.get(&device_id) {
                Some(connection)
                    if connection
                        .snapshot
                        .capabilities()
                        .contains(&Capability::LocalVideoHandoff)
                        && connection
                            .snapshot
                            .capabilities()
                            .contains(&Capability::FileTransfer) =>
                {
                    Ok(())
                }
                Some(_) => Err(TransportError::UnsupportedCapability),
                None => Err(TransportError::NotConnected),
            };
            let _ = response.send(result);
        }
        SupervisorCommand::SendHandoff {
            device_id,
            handoff_id,
            payload,
            response,
        } => {
            let Some(connection) = active.get(&device_id) else {
                let _ = response.send(Err(TransportError::NotConnected));
                return false;
            };
            if !connection
                .snapshot
                .capabilities()
                .contains(&payload.capability())
            {
                let _ = response.send(Err(TransportError::UnsupportedCapability));
                return false;
            }
            if connection
                .commands
                .try_send(ConnectionCommand::SendHandoff {
                    handoff_id,
                    payload,
                    response,
                })
                .is_err()
            {
                return false;
            }
        }
        SupervisorCommand::SendTransfer {
            device_id,
            transfer_id,
            message,
            response,
        } => {
            let Some(connection) = active.get(&device_id) else {
                let _ = response.send(Err(TransportError::NotConnected));
                return false;
            };
            if !connection
                .snapshot
                .capabilities()
                .contains(&Capability::FileTransfer)
            {
                let _ = response.send(Err(TransportError::UnsupportedCapability));
                return false;
            }
            if connection
                .commands
                .try_send(ConnectionCommand::SendTransfer {
                    transfer_id,
                    message,
                    response,
                })
                .is_err()
            {
                return false;
            }
        }
        SupervisorCommand::Shutdown { response } => {
            let _ = response.send(());
            return true;
        }
    }
    false
}

fn handle_worker_event(
    event: WorkerEvent,
    local_identity: &LocalDeviceIdentity,
    active: &mut BTreeMap<DeviceId, ActiveConnection>,
    pending_outgoing: &mut BTreeSet<DeviceId>,
    pending_incoming: &mut usize,
    connections: &ConnectionStore,
    changed: &ConnectionChangedEvent,
) {
    match event {
        WorkerEvent::Established {
            runtime_id,
            snapshot,
            fingerprint,
            commands,
            accepted,
            connect_response,
            incoming,
        } => {
            finish_pending(&snapshot, incoming, pending_outgoing, pending_incoming);
            let device_id = snapshot.device_id().clone();
            let preferred = preferred_direction(local_identity.id(), &device_id);
            let replace_existing = active.get(&device_id).is_some_and(|existing| {
                existing.snapshot.direction() != preferred && snapshot.direction() == preferred
            });
            if active.contains_key(&device_id) && !replace_existing {
                let _ = accepted.send(false);
                if let Some(response) = connect_response {
                    let _ = response.send(Err(TransportError::AlreadyConnected));
                }
                return;
            }
            if let Some(existing) = active.remove(&device_id) {
                let _ = existing.commands.try_send(ConnectionCommand::Close);
                remove_connection_snapshot(connections, &device_id);
                changed.publish(ConnectionChange::Removed(existing.snapshot));
            }
            active.insert(
                device_id.clone(),
                ActiveConnection {
                    runtime_id,
                    snapshot: snapshot.clone(),
                    fingerprint,
                    commands,
                },
            );
            insert_connection_snapshot(connections, snapshot.clone());
            changed.publish(ConnectionChange::Added(snapshot.clone()));
            let _ = accepted.send(true);
            if let Some(response) = connect_response {
                let _ = response.send(Ok(snapshot));
            }
        }
        WorkerEvent::Failed {
            device_id,
            error,
            connect_response,
            incoming,
        } => {
            if incoming {
                *pending_incoming = pending_incoming.saturating_sub(1);
            }
            if let Some(device_id) = device_id {
                pending_outgoing.remove(&device_id);
            }
            if let Some(response) = connect_response {
                let _ = response.send(Err(error));
            }
        }
        WorkerEvent::Closed {
            runtime_id,
            device_id,
        } => {
            let matches = active
                .get(&device_id)
                .is_some_and(|connection| connection.runtime_id == runtime_id);
            if matches && let Some(connection) = active.remove(&device_id) {
                remove_connection_snapshot(connections, &device_id);
                changed.publish(ConnectionChange::Removed(connection.snapshot));
            }
        }
    }
}

fn finish_pending(
    snapshot: &AuthenticatedConnection,
    incoming: bool,
    pending_outgoing: &mut BTreeSet<DeviceId>,
    pending_incoming: &mut usize,
) {
    if incoming {
        *pending_incoming = pending_incoming.saturating_sub(1);
    } else {
        pending_outgoing.remove(snapshot.device_id());
    }
}

fn spawn_outgoing(
    tasks: &mut JoinSet<()>,
    endpoint: DiscoveryEndpoint,
    trusted_peer: TrustedDevice,
    context: WorkerContext,
    response: oneshot::Sender<Result<AuthenticatedConnection, TransportError>>,
) {
    tasks.spawn(async move {
        let device_id = trusted_peer.device_id().clone();
        let result = establish_outgoing(
            &endpoint,
            &trusted_peer,
            &context.local_identity,
            &context.security,
            &context.trusted_peers,
            &context.handoff,
            &context.transfer,
        )
        .await;
        run_established(
            result,
            Some(device_id),
            false,
            Some(response),
            context.events,
        )
        .await;
    });
}

fn spawn_incoming(tasks: &mut JoinSet<()>, stream: TcpStream, context: WorkerContext) {
    tasks.spawn(async move {
        let result = establish_incoming(
            stream,
            &context.local_identity,
            &context.security,
            &context.trusted_peers,
            &context.handoff,
            &context.transfer,
        )
        .await;
        run_established(result, None, true, None, context.events).await;
    });
}

async fn run_established(
    result: Result<EstablishedChannel, TransportError>,
    device_id: Option<DeviceId>,
    incoming: bool,
    connect_response: Option<oneshot::Sender<Result<AuthenticatedConnection, TransportError>>>,
    worker_events: mpsc::Sender<WorkerEvent>,
) {
    let established = match result {
        Ok(established) => established,
        Err(error) => {
            let _ = worker_events
                .send(WorkerEvent::Failed {
                    device_id,
                    error,
                    connect_response,
                    incoming,
                })
                .await;
            return;
        }
    };
    let runtime_id = Uuid::new_v4();
    let snapshot = established.snapshot.clone();
    let (commands, command_receiver) = mpsc::channel(CONNECTION_COMMAND_LIMIT);
    let (acceptance, accepted) = oneshot::channel();
    if worker_events
        .send(WorkerEvent::Established {
            runtime_id,
            snapshot: snapshot.clone(),
            fingerprint: established.fingerprint,
            commands,
            accepted: acceptance,
            connect_response,
            incoming,
        })
        .await
        .is_err()
    {
        return;
    }
    if !accepted.await.unwrap_or(false) {
        return;
    }
    let device_id = snapshot.device_id().clone();
    let _ = run_connection(established, command_receiver).await;
    let _ = worker_events
        .send(WorkerEvent::Closed {
            runtime_id,
            device_id,
        })
        .await;
}

async fn establish_outgoing(
    endpoint: &DiscoveryEndpoint,
    trusted_peer: &TrustedDevice,
    local_identity: &LocalDeviceIdentity,
    security: &SecurityCapability,
    trusted_peers: &TrustedPeerLookup,
    handoff: &HandoffTransportCapability,
    transfer: &TransferTransportCapability,
) -> Result<EstablishedChannel, TransportError> {
    let cryptographic_identity = load_cryptographic_identity(security, local_identity.id()).await?;
    let channel =
        AuthenticatedChannel::connect(endpoint, &cryptographic_identity, trusted_peer).await?;
    establish(
        channel,
        ConnectionDirection::Outgoing,
        Some(trusted_peer.device_id()),
        local_identity,
        trusted_peers,
        handoff,
        transfer,
    )
    .await
}

async fn establish_incoming(
    stream: TcpStream,
    local_identity: &LocalDeviceIdentity,
    security: &SecurityCapability,
    trusted_peers: &TrustedPeerLookup,
    handoff: &HandoffTransportCapability,
    transfer: &TransferTransportCapability,
) -> Result<EstablishedChannel, TransportError> {
    let cryptographic_identity = load_cryptographic_identity(security, local_identity.id()).await?;
    let channel =
        AuthenticatedChannel::accept(stream, &cryptographic_identity, trusted_peers.clone())
            .await?;
    establish(
        channel,
        ConnectionDirection::Incoming,
        None,
        local_identity,
        trusted_peers,
        handoff,
        transfer,
    )
    .await
}

async fn establish(
    mut channel: AuthenticatedChannel,
    direction: ConnectionDirection,
    expected_device_id: Option<&DeviceId>,
    local_identity: &LocalDeviceIdentity,
    trusted_peers: &TrustedPeerLookup,
    handoff: &HandoffTransportCapability,
    transfer: &TransferTransportCapability,
) -> Result<EstablishedChannel, TransportError> {
    let mut capabilities = Vec::new();
    if handoff.is_supported() {
        capabilities.push(Capability::UrlHandoff);
        capabilities.push(Capability::PlaybackPositionHandoff);
    }
    if transfer.is_supported() {
        capabilities.push(Capability::FileTransfer);
        if handoff.is_supported() {
            capabilities.push(Capability::LocalVideoHandoff);
        }
    }
    let local_hello = ApplicationHello::local(local_identity, capabilities)?;
    channel
        .send(
            &ProtocolEnvelope::hello(local_hello.clone()),
            MAX_CONTROL_FRAME_SIZE,
        )
        .await?;
    let peer_envelope = channel.receive(MAX_CONTROL_FRAME_SIZE).await?;
    let peer_hello = peer_envelope
        .message()
        .hello()
        .ok_or(TransportError::ProtocolViolation)?;
    if expected_device_id.is_some_and(|expected| expected != peer_hello.device_id())
        || peer_hello.device_id() == local_identity.id()
        || peer_hello.nonce() == local_hello.nonce()
    {
        return Err(TransportError::IdentityMismatch);
    }
    let fingerprint = channel.peer_fingerprint();
    let trusted_peer = trusted_peers
        .get(peer_hello.device_id())
        .map_err(|_| TransportError::TrustUnavailable)?
        .ok_or(TransportError::UntrustedPeer)?;
    if trusted_peer.public_key_fingerprint() != &fingerprint {
        return Err(TransportError::IdentityMismatch);
    }
    let negotiated = peer_hello.negotiate()?;
    Ok(EstablishedChannel {
        channel,
        snapshot: AuthenticatedConnection::new(
            peer_hello.device_id().clone(),
            peer_hello.display_name().to_owned(),
            peer_hello.platform(),
            negotiated.version(),
            peer_hello.capabilities().to_vec(),
            direction,
        ),
        fingerprint,
        maximum_frame_size: negotiated.limits().maximum_frame_size(),
        maximum_in_flight_requests: negotiated.limits().maximum_in_flight_requests(),
        idle_timeout: Duration::from_secs(u64::from(negotiated.limits().idle_timeout_seconds())),
        handoff: handoff.clone(),
        transfer: transfer.clone(),
    })
}

async fn run_connection(
    established: EstablishedChannel,
    mut commands: mpsc::Receiver<ConnectionCommand>,
) -> Result<(), TransportError> {
    let maximum_frame_size = established.maximum_frame_size;
    let maximum_in_flight_requests = established.maximum_in_flight_requests;
    let idle_timeout = established.idle_timeout;
    let handoff = established.handoff.clone();
    let transfer = established.transfer.clone();
    let peer_device_id = established.snapshot.device_id().clone();
    let (reader, mut writer) = established.channel.split();
    let (received_messages, mut messages) = mpsc::channel(CONNECTION_COMMAND_LIMIT);
    let (inbound_transfer_results, mut transfer_results) =
        mpsc::channel::<InboundTransferResult>(CONNECTION_COMMAND_LIMIT);
    let reader_task = tokio::spawn(run_reader(reader, maximum_frame_size, received_messages));
    let mut request_identifier = 1_u64;
    let mut pending = BTreeMap::<u64, PendingRequest>::new();
    let mut inbound_transfers = BTreeSet::<u64>::new();
    let mut received = VecDeque::<u64>::new();
    let mut last_activity = Instant::now();
    let mut timer = interval(Duration::from_secs(1));
    timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let result = async {
        loop {
            tokio::select! {
                command = commands.recv() => {
                    match command {
                        Some(ConnectionCommand::Probe(response)) => {
                            if pending.len() >= maximum_in_flight_requests {
                                let _ = response.send(Err(TransportError::ConnectionLimit));
                                continue;
                            }
                            let nonce = random_u64()?;
                            let envelope = ProtocolEnvelope::ping(request_identifier, nonce)?;
                            writer.send(&envelope, maximum_frame_size).await?;
                            pending.insert(request_identifier, PendingRequest {
                                started: Instant::now(),
                                timeout: REQUEST_TIMEOUT,
                                operation: PendingOperation::Probe { nonce, response },
                            });
                            request_identifier = next_request_identifier(request_identifier);
                            last_activity = Instant::now();
                        }
                        Some(ConnectionCommand::SendHandoff {
                            handoff_id,
                            payload,
                            response,
                        }) => {
                            if pending.len() >= maximum_in_flight_requests {
                                let _ = response.send(Err(TransportError::ConnectionLimit));
                                continue;
                            }
                            let envelope = ProtocolEnvelope::handoff(
                                request_identifier,
                                handoff_id,
                                payload,
                            )?;
                            writer.send(&envelope, maximum_frame_size).await?;
                            pending.insert(request_identifier, PendingRequest {
                                started: Instant::now(),
                                timeout: REQUEST_TIMEOUT,
                                operation: PendingOperation::Handoff {
                                    handoff_id,
                                    response,
                                },
                            });
                            request_identifier = next_request_identifier(request_identifier);
                            last_activity = Instant::now();
                        }
                        Some(ConnectionCommand::SendTransfer {
                            transfer_id,
                            message,
                            response,
                        }) => {
                            if pending.len() >= maximum_in_flight_requests {
                                let _ = response.send(Err(TransportError::ConnectionLimit));
                                continue;
                            }
                            let timeout = if matches!(
                                &message,
                                TransferTransportMessage::Offer { .. }
                            ) {
                                TRANSFER_OFFER_TIMEOUT
                            } else {
                                REQUEST_TIMEOUT
                            };
                            let envelope = ProtocolEnvelope::transfer(
                                request_identifier,
                                transfer_id,
                                message,
                            )?;
                            writer.send(&envelope, maximum_frame_size).await?;
                            pending.insert(request_identifier, PendingRequest {
                                started: Instant::now(),
                                timeout,
                                operation: PendingOperation::Transfer {
                                    transfer_id,
                                    response,
                                },
                            });
                            request_identifier = next_request_identifier(request_identifier);
                            last_activity = Instant::now();
                        }
                        Some(ConnectionCommand::Close) | None => {
                            let _ = writer
                                .send(&ProtocolEnvelope::close(), maximum_frame_size)
                                .await;
                            return Ok(());
                        }
                    }
                }
                received_envelope = messages.recv() => {
                    let envelope = received_envelope
                        .ok_or(TransportError::ConnectionFailed)??;
                    last_activity = Instant::now();
                    match envelope.message() {
                        ProtocolMessage::Ping(nonce) => {
                            if received.contains(&envelope.request_id()) {
                                return Err(TransportError::ProtocolViolation);
                            }
                            received.push_back(envelope.request_id());
                            if received.len() > maximum_in_flight_requests * 2 {
                                received.pop_front();
                            }
                            writer.send(
                                &ProtocolEnvelope::pong(envelope.request_id(), *nonce)?,
                                maximum_frame_size,
                            ).await?;
                        }
                        ProtocolMessage::Pong(nonce) => {
                            let request = pending
                                .remove(&envelope.request_id())
                                .ok_or(TransportError::ProtocolViolation)?;
                            match request.operation {
                                PendingOperation::Probe {
                                    nonce: expected,
                                    response,
                                } if expected == *nonce => {
                                    let _ = response.send(Ok(()));
                                }
                                _ => return Err(TransportError::ProtocolViolation),
                            }
                        }
                        ProtocolMessage::Handoff {
                            handoff_id,
                            payload,
                        } => {
                            if received.contains(&envelope.request_id()) {
                                return Err(TransportError::ProtocolViolation);
                            }
                            received.push_back(envelope.request_id());
                            if received.len() > maximum_in_flight_requests * 2 {
                                received.pop_front();
                            }
                            let disposition = handoff
                                .receive(InboundHandoff::new(
                                    *handoff_id,
                                    peer_device_id.clone(),
                                    payload.clone(),
                                ))
                                .await;
                            let response = match disposition {
                                HandoffDisposition::Accepted => {
                                    ProtocolEnvelope::handoff_accepted(
                                        envelope.request_id(),
                                        *handoff_id,
                                    )?
                                }
                                HandoffDisposition::Rejected(reason) => {
                                    ProtocolEnvelope::handoff_rejected(
                                        envelope.request_id(),
                                        *handoff_id,
                                        reason,
                                    )?
                                }
                            };
                            writer.send(&response, maximum_frame_size).await?;
                        }
                        ProtocolMessage::HandoffAccepted(handoff_id) => {
                            complete_handoff(
                                &mut pending,
                                envelope.request_id(),
                                *handoff_id,
                                HandoffDisposition::Accepted,
                            )?;
                        }
                        ProtocolMessage::HandoffRejected { handoff_id, reason } => {
                            complete_handoff(
                                &mut pending,
                                envelope.request_id(),
                                *handoff_id,
                                HandoffDisposition::Rejected(*reason),
                            )?;
                        }
                        ProtocolMessage::Transfer {
                            transfer_id,
                            message,
                        } => {
                            if received.contains(&envelope.request_id())
                                || inbound_transfers.len() >= maximum_in_flight_requests
                            {
                                return Err(TransportError::ProtocolViolation);
                            }
                            received.push_back(envelope.request_id());
                            if received.len() > maximum_in_flight_requests * 2 {
                                received.pop_front();
                            }
                            inbound_transfers.insert(envelope.request_id());
                            let request_id = envelope.request_id();
                            let transfer_id = *transfer_id;
                            let incoming = InboundTransfer::new(
                                transfer_id,
                                peer_device_id.clone(),
                                message.clone(),
                            );
                            let transfer = transfer.clone();
                            let results = inbound_transfer_results.clone();
                            tokio::spawn(async move {
                                let disposition = transfer.receive(incoming).await;
                                let _ = results
                                    .send(InboundTransferResult {
                                        request_id,
                                        transfer_id,
                                        disposition,
                                    })
                                    .await;
                            });
                        }
                        ProtocolMessage::TransferAccepted(transfer_id) => {
                            complete_transfer(
                                &mut pending,
                                envelope.request_id(),
                                *transfer_id,
                                TransferDisposition::Accepted,
                            )?;
                        }
                        ProtocolMessage::TransferRejected {
                            transfer_id,
                            reason,
                        } => {
                            complete_transfer(
                                &mut pending,
                                envelope.request_id(),
                                *transfer_id,
                                TransferDisposition::Rejected(*reason),
                            )?;
                        }
                        ProtocolMessage::Close => return Ok(()),
                        ProtocolMessage::Hello(_) => return Err(TransportError::ProtocolViolation),
                    }
                }
                inbound_result = transfer_results.recv() => {
                    let result = inbound_result.ok_or(TransportError::ConnectionFailed)?;
                    if !inbound_transfers.remove(&result.request_id) {
                        return Err(TransportError::ProtocolViolation);
                    }
                    let envelope = match result.disposition {
                        TransferDisposition::Accepted => ProtocolEnvelope::transfer_accepted(
                            result.request_id,
                            result.transfer_id,
                        )?,
                        TransferDisposition::Rejected(reason) => {
                            ProtocolEnvelope::transfer_rejected(
                                result.request_id,
                                result.transfer_id,
                                reason,
                            )?
                        }
                    };
                    writer.send(&envelope, maximum_frame_size).await?;
                    last_activity = Instant::now();
                }
                _ = timer.tick() => {
                    let now = Instant::now();
                    let timed_out = pending
                        .iter()
                        .filter_map(|(identifier, request)| {
                            (now.duration_since(request.started) >= request.timeout)
                                .then_some(*identifier)
                        })
                        .collect::<Vec<_>>();
                    for identifier in timed_out {
                        if let Some(request) = pending.remove(&identifier) {
                            fail_pending(request, TransportError::TimedOut);
                        }
                    }
                    if now.duration_since(last_activity) >= idle_timeout {
                        return Ok(());
                    }
                }
            }
        }
    }
    .await;
    reader_task.abort();
    let _ = reader_task.await;
    for request in pending.into_values() {
        fail_pending(request, TransportError::ConnectionFailed);
    }
    result
}

fn complete_handoff(
    pending: &mut BTreeMap<u64, PendingRequest>,
    request_id: u64,
    handoff_id: [u8; 16],
    disposition: HandoffDisposition,
) -> Result<(), TransportError> {
    let request = pending
        .remove(&request_id)
        .ok_or(TransportError::ProtocolViolation)?;
    match request.operation {
        PendingOperation::Handoff {
            handoff_id: expected,
            response,
        } if expected == handoff_id => {
            let _ = response.send(Ok(disposition));
            Ok(())
        }
        _ => Err(TransportError::ProtocolViolation),
    }
}

fn complete_transfer(
    pending: &mut BTreeMap<u64, PendingRequest>,
    request_id: u64,
    transfer_id: [u8; 16],
    disposition: TransferDisposition,
) -> Result<(), TransportError> {
    let request = pending
        .remove(&request_id)
        .ok_or(TransportError::ProtocolViolation)?;
    match request.operation {
        PendingOperation::Transfer {
            transfer_id: expected,
            response,
        } if expected == transfer_id => {
            let _ = response.send(Ok(disposition));
            Ok(())
        }
        _ => Err(TransportError::ProtocolViolation),
    }
}

fn fail_pending(request: PendingRequest, error: TransportError) {
    match request.operation {
        PendingOperation::Probe { response, .. } => {
            let _ = response.send(Err(error));
        }
        PendingOperation::Handoff { response, .. } => {
            let _ = response.send(Err(error));
        }
        PendingOperation::Transfer { response, .. } => {
            let _ = response.send(Err(error));
        }
    }
}

async fn run_reader(
    mut reader: AuthenticatedReader,
    maximum_frame_size: usize,
    messages: mpsc::Sender<Result<ProtocolEnvelope, TransportError>>,
) {
    loop {
        let result = reader.receive(maximum_frame_size).await;
        let failed = result.is_err();
        if messages.send(result).await.is_err() || failed {
            return;
        }
    }
}

fn remove_untrusted_connections(
    trusted_peers: &TrustedPeerLookup,
    active: &mut BTreeMap<DeviceId, ActiveConnection>,
    connections: &ConnectionStore,
    changed: &ConnectionChangedEvent,
) {
    let removed = active
        .iter()
        .filter_map(|(device_id, connection)| {
            let remains_trusted = trusted_peers
                .get(device_id)
                .ok()
                .flatten()
                .is_some_and(|trusted| trusted.public_key_fingerprint() == &connection.fingerprint);
            (!remains_trusted).then_some(device_id.clone())
        })
        .collect::<Vec<_>>();
    for device_id in removed {
        if let Some(connection) = active.remove(&device_id) {
            let _ = connection.commands.try_send(ConnectionCommand::Close);
            remove_connection_snapshot(connections, &device_id);
            changed.publish(ConnectionChange::Removed(connection.snapshot));
        }
    }
}

fn preferred_direction(local: &DeviceId, peer: &DeviceId) -> ConnectionDirection {
    if local < peer {
        ConnectionDirection::Outgoing
    } else {
        ConnectionDirection::Incoming
    }
}

fn allow_incoming(attempts: &mut HashMap<IpAddr, VecDeque<Instant>>, address: IpAddr) -> bool {
    let now = Instant::now();
    let oldest_allowed = now - INCOMING_RATE_WINDOW;
    attempts.retain(|_, values| {
        values.retain(|attempt| *attempt >= oldest_allowed);
        !values.is_empty()
    });
    if !attempts.contains_key(&address) && attempts.len() >= MAX_RATE_LIMIT_ADDRESSES {
        return false;
    }
    let attempts = attempts.entry(address).or_default();
    if attempts.len() >= MAX_INCOMING_PER_ADDRESS {
        return false;
    }
    attempts.push_back(now);
    true
}

fn insert_connection_snapshot(store: &ConnectionStore, snapshot: AuthenticatedConnection) {
    lock(store).insert(snapshot.device_id().clone(), snapshot);
}

fn remove_connection_snapshot(store: &ConnectionStore, device_id: &DeviceId) {
    lock(store).remove(device_id);
}

fn replace_connections(
    store: &ConnectionStore,
    connections: BTreeMap<DeviceId, AuthenticatedConnection>,
) {
    *lock(store) = connections;
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

fn random_u64() -> Result<u64, TransportError> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes).map_err(|_| TransportError::ConnectionFailed)?;
    Ok(u64::from_be_bytes(bytes))
}

fn next_request_identifier(current: u64) -> u64 {
    let next = current.wrapping_add(1);
    if next == 0 { 1 } else { next }
}

async fn load_cryptographic_identity(
    security: &SecurityCapability,
    device_id: &DeviceId,
) -> Result<Arc<CryptographicIdentity>, TransportError> {
    let security = security.clone();
    let device_id = device_id.clone();
    tokio::task::spawn_blocking(move || security.identity(&device_id))
        .await
        .map_err(|_| TransportError::TlsConfigurationFailed)?
        .map_err(|_| TransportError::TlsConfigurationFailed)
}

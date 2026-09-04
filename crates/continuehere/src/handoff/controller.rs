use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{Arc, Mutex, MutexGuard, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    models::DeviceId,
    transport::{
        HandoffDisposition, HandoffRejection, HandoffTransportCapability, HandoffTransportPayload,
        InboundHandoff, InboundHandoffHandler, TransportError,
    },
};

use super::{
    Handoff, HandoffChange, HandoffChangedEvent, HandoffConfig, HandoffError, HandoffFailure,
    HandoffId, HandoffPayload, HandoffState, IncomingHandoff, IncomingHandoffChange,
    IncomingHandoffChangedEvent, PlaybackPosition, UrlHandoff, YouTubeHandoff,
};

const MAX_ACTIVE_OPERATIONS: usize = 32;
const MAX_INCOMING_HANDOFFS: usize = 64;
const MAX_SEEN_HANDOFFS: usize = 256;
const RESULT_POLL_INTERVAL: Duration = Duration::from_millis(25);

pub(crate) struct HandoffController {
    state: Mutex<ControllerState>,
    transport: HandoffTransportCapability,
    handoff_changed: HandoffChangedEvent,
    incoming_changed: IncomingHandoffChangedEvent,
}

struct ControllerState {
    running: bool,
    handoffs: BTreeMap<HandoffId, Handoff>,
    operations: HashMap<String, ActiveOperation>,
    incoming: BTreeMap<HandoffId, IncomingHandoff>,
    seen: BTreeMap<HandoffId, DeviceId>,
    seen_order: VecDeque<HandoffId>,
    orphan_workers: Vec<JoinHandle<()>>,
}

struct ActiveOperation {
    handoff_id: HandoffId,
    commands: mpsc::Sender<OperationCommand>,
    worker: Option<JoinHandle<()>>,
}

enum OperationCommand {
    Cancel,
}

impl HandoffController {
    pub(crate) fn new(
        transport: HandoffTransportCapability,
        handoff_changed: HandoffChangedEvent,
        incoming_changed: IncomingHandoffChangedEvent,
    ) -> Arc<Self> {
        let controller = Arc::new(Self {
            state: Mutex::new(ControllerState {
                running: false,
                handoffs: BTreeMap::new(),
                operations: HashMap::new(),
                incoming: BTreeMap::new(),
                seen: BTreeMap::new(),
                seen_order: VecDeque::new(),
                orphan_workers: Vec::new(),
            }),
            transport: transport.clone(),
            handoff_changed,
            incoming_changed,
        });
        let handler: Arc<dyn InboundHandoffHandler> = controller.clone();
        transport.set_handler(Arc::downgrade(&handler));
        controller
    }

    pub(crate) fn start(self: &Arc<Self>) -> Result<(), HandoffError> {
        let handler: Arc<dyn InboundHandoffHandler> = self.clone();
        self.transport.set_handler(Arc::downgrade(&handler));
        lock(&self.state)?.running = true;
        Ok(())
    }

    pub(crate) fn is_running(&self) -> bool {
        lock(&self.state)
            .map(|state| state.running)
            .unwrap_or(false)
    }

    pub(crate) fn stop(&self) -> Result<(), HandoffError> {
        let (handoffs, incoming, workers) = {
            let mut state = lock(&self.state)?;
            state.running = false;
            for operation in state.operations.values() {
                let _ = operation.commands.send(OperationCommand::Cancel);
            }
            let mut workers = state
                .operations
                .values_mut()
                .filter_map(|operation| operation.worker.take())
                .collect::<Vec<_>>();
            workers.append(&mut state.orphan_workers);
            state.operations.clear();
            state.seen.clear();
            state.seen_order.clear();
            let handoffs = std::mem::take(&mut state.handoffs)
                .into_values()
                .collect::<Vec<_>>();
            let incoming = std::mem::take(&mut state.incoming)
                .into_values()
                .collect::<Vec<_>>();
            (handoffs, incoming, workers)
        };
        self.transport.clear_handler();
        for handoff in handoffs {
            self.handoff_changed
                .publish(HandoffChange::Removed(handoff));
        }
        for handoff in incoming {
            self.incoming_changed
                .publish(IncomingHandoffChange::Removed(handoff));
        }
        for worker in workers {
            worker.join().map_err(|_| HandoffError::WorkerStopFailed)?;
        }
        Ok(())
    }

    pub(crate) fn begin(
        self: &Arc<Self>,
        handle_identifier: String,
        config: HandoffConfig,
    ) -> Result<(), HandoffError> {
        let handoff_id = HandoffId::new();
        let handoff = Handoff::new(handoff_id.clone(), config.clone());
        let (commands, receiver) = mpsc::channel();
        {
            let mut state = lock(&self.state)?;
            if !state.running {
                return Err(HandoffError::ManagerUnavailable);
            }
            if state.operations.len() >= MAX_ACTIVE_OPERATIONS {
                return Err(HandoffError::OperationLimit);
            }
            state.handoffs.insert(handoff_id.clone(), handoff.clone());
            state.operations.insert(
                handle_identifier.clone(),
                ActiveOperation {
                    handoff_id: handoff_id.clone(),
                    commands,
                    worker: None,
                },
            );
        }
        self.handoff_changed.publish(HandoffChange::Added(handoff));

        let controller = Arc::clone(self);
        let worker_identifier = handle_identifier.clone();
        let spawn_result = thread::Builder::new()
            .name("continuehere-handoff".to_owned())
            .spawn(move || {
                run_outgoing(controller, worker_identifier, handoff_id, config, receiver);
            });
        let worker = match spawn_result {
            Ok(worker) => worker,
            Err(_) => {
                self.remove(&handle_identifier);
                return Err(HandoffError::CommandUnavailable);
            }
        };
        let mut state = lock(&self.state)?;
        match state.operations.get_mut(&handle_identifier) {
            Some(operation) => operation.worker = Some(worker),
            None => state.orphan_workers.push(worker),
        }
        Ok(())
    }

    pub(crate) fn cancel(&self, handle_identifier: &str) {
        let changed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let handoff_id = match state.operations.get(handle_identifier) {
                Some(operation) => {
                    let _ = operation.commands.send(OperationCommand::Cancel);
                    operation.handoff_id.clone()
                }
                None => return,
            };
            let handoff = match state.handoffs.get_mut(&handoff_id) {
                Some(handoff) if handoff.state() == HandoffState::Sending => handoff,
                _ => return,
            };
            handoff.cancel();
            handoff.clone()
        };
        self.handoff_changed
            .publish(HandoffChange::Updated(changed));
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
            let _ = operation.commands.send(OperationCommand::Cancel);
            if let Some(worker) = operation.worker.take() {
                state.orphan_workers.push(worker);
            }
            state.handoffs.remove(&operation.handoff_id)
        };
        if let Some(handoff) = removed {
            self.handoff_changed
                .publish(HandoffChange::Removed(handoff));
        }
    }

    pub(crate) fn handoff_for_handle(&self, handle_identifier: &str) -> Option<Handoff> {
        let state = lock(&self.state).ok()?;
        let handoff_id = &state.operations.get(handle_identifier)?.handoff_id;
        state.handoffs.get(handoff_id).cloned()
    }

    pub(crate) fn handoffs(&self) -> Vec<Handoff> {
        lock(&self.state)
            .map(|state| state.handoffs.values().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn incoming(&self) -> Vec<IncomingHandoff> {
        lock(&self.state)
            .map(|state| state.incoming.values().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn remove_incoming(&self, handoff_id: &HandoffId) -> Result<(), HandoffError> {
        let removed = lock(&self.state)?
            .incoming
            .remove(handoff_id)
            .ok_or(HandoffError::IncomingHandoffNotFound)?;
        self.incoming_changed
            .publish(IncomingHandoffChange::Removed(removed));
        Ok(())
    }

    fn finish(&self, handle_identifier: &str, result: OperationResult) {
        let changed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let handoff_id = match state.operations.get(handle_identifier) {
                Some(operation) => operation.handoff_id.clone(),
                None => return,
            };
            let Some(handoff) = state.handoffs.get_mut(&handoff_id) else {
                return;
            };
            if handoff.state() != HandoffState::Sending {
                return;
            }
            match result {
                OperationResult::Delivered => handoff.deliver(),
                OperationResult::Rejected(failure) => handoff.reject(failure),
                OperationResult::Failed(failure) => handoff.fail(failure),
            }
            handoff.clone()
        };
        self.handoff_changed
            .publish(HandoffChange::Updated(changed));
    }
}

impl InboundHandoffHandler for HandoffController {
    fn receive(&self, handoff: InboundHandoff) -> HandoffDisposition {
        let (handoff_id, sender_device_id, transport_payload) = handoff.into_parts();
        let payload = match transport_payload {
            HandoffTransportPayload::Url(url) => match UrlHandoff::new(&url) {
                Ok(url) => HandoffPayload::Url(url),
                Err(_) => {
                    return HandoffDisposition::Rejected(HandoffRejection::Invalid);
                }
            },
            HandoffTransportPayload::YouTube {
                video_id,
                playback_position_millis,
            } => match YouTubeHandoff::from_parts(
                video_id,
                PlaybackPosition::from_millis(playback_position_millis),
            ) {
                Ok(youtube) => HandoffPayload::YouTube(youtube),
                Err(_) => {
                    return HandoffDisposition::Rejected(HandoffRejection::Invalid);
                }
            },
        };
        let id = HandoffId::from_bytes(handoff_id);
        let incoming = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => {
                    return HandoffDisposition::Rejected(HandoffRejection::Unavailable);
                }
            };
            if !state.running {
                return HandoffDisposition::Rejected(HandoffRejection::Unavailable);
            }
            if let Some(sender) = state.seen.get(&id) {
                if sender == &sender_device_id {
                    return HandoffDisposition::Accepted;
                }
                return HandoffDisposition::Rejected(HandoffRejection::Invalid);
            }
            if state.incoming.len() >= MAX_INCOMING_HANDOFFS {
                return HandoffDisposition::Rejected(HandoffRejection::Busy);
            }
            let incoming = IncomingHandoff::new(id.clone(), sender_device_id.clone(), payload);
            state.incoming.insert(id.clone(), incoming.clone());
            state.seen.insert(id.clone(), sender_device_id);
            state.seen_order.push_back(id);
            if state.seen_order.len() > MAX_SEEN_HANDOFFS
                && let Some(expired) = state.seen_order.pop_front()
            {
                state.seen.remove(&expired);
            }
            incoming
        };
        self.incoming_changed
            .publish(IncomingHandoffChange::Added(incoming));
        HandoffDisposition::Accepted
    }
}

enum OperationResult {
    Delivered,
    Rejected(HandoffFailure),
    Failed(HandoffFailure),
}

fn run_outgoing(
    controller: Arc<HandoffController>,
    handle_identifier: String,
    handoff_id: HandoffId,
    config: HandoffConfig,
    commands: mpsc::Receiver<OperationCommand>,
) {
    let (device_id, payload) = config.into_parts();
    let transport_payload = match payload {
        HandoffPayload::Url(url) => HandoffTransportPayload::Url(url.url().to_owned()),
        HandoffPayload::YouTube(youtube) => HandoffTransportPayload::YouTube {
            video_id: youtube.video_id().to_owned(),
            playback_position_millis: youtube.playback_position().as_millis(),
        },
    };
    let response = match controller
        .transport
        .send(device_id, handoff_id.bytes(), transport_payload)
    {
        Ok(response) => response,
        Err(error) => {
            controller.finish(&handle_identifier, map_transport_error(error));
            return;
        }
    };
    loop {
        match response.try_recv() {
            Ok(Ok(HandoffDisposition::Accepted)) => {
                controller.finish(&handle_identifier, OperationResult::Delivered);
                return;
            }
            Ok(Ok(HandoffDisposition::Rejected(reason))) => {
                controller.finish(
                    &handle_identifier,
                    OperationResult::Rejected(map_rejection(reason)),
                );
                return;
            }
            Ok(Err(error)) => {
                controller.finish(&handle_identifier, map_transport_error(error));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                controller.finish(
                    &handle_identifier,
                    OperationResult::Failed(HandoffFailure::Transport),
                );
                return;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        match commands.recv_timeout(RESULT_POLL_INTERVAL) {
            Ok(OperationCommand::Cancel) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

fn map_rejection(reason: HandoffRejection) -> HandoffFailure {
    match reason {
        HandoffRejection::Invalid => HandoffFailure::Invalid,
        HandoffRejection::Busy => HandoffFailure::Busy,
        HandoffRejection::Unavailable => HandoffFailure::Unsupported,
    }
}

fn map_transport_error(error: TransportError) -> OperationResult {
    match error {
        TransportError::NotConnected => OperationResult::Failed(HandoffFailure::NotConnected),
        TransportError::UnsupportedCapability => {
            OperationResult::Rejected(HandoffFailure::Unsupported)
        }
        TransportError::TimedOut => OperationResult::Failed(HandoffFailure::TimedOut),
        _ => OperationResult::Failed(HandoffFailure::Transport),
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, HandoffError> {
    value
        .lock()
        .map_err(|_| HandoffError::SynchronizationFailed)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::{HandoffController, InboundHandoffHandler};
    use crate::{
        handoff::{
            HandoffChangedEvent, IncomingHandoffChange, IncomingHandoffChangedDelegate,
            IncomingHandoffChangedEvent,
        },
        models::DeviceId,
        transport::{
            HandoffDisposition, HandoffRejection, HandoffTransportCapability,
            HandoffTransportPayload, InboundHandoff,
        },
    };

    #[test]
    fn duplicate_incoming_handoff_is_acknowledged_once() {
        let changed = IncomingHandoffChangedEvent::default();
        let (sender, receiver) = mpsc::channel();
        let _subscription = changed.subscribe(IncomingHandoffChangedDelegate::new(move |change| {
            sender
                .send(change)
                .expect("receiver should remain available");
        }));
        let controller = HandoffController::new(
            HandoffTransportCapability::new(),
            HandoffChangedEvent::default(),
            changed,
        );
        controller.start().expect("controller should start");
        let device_id = DeviceId::new("sender").expect("device identifier should be valid");

        for _ in 0..2 {
            assert_eq!(
                controller.receive(InboundHandoff::new(
                    [3_u8; 16],
                    device_id.clone(),
                    HandoffTransportPayload::Url("https://example.com".to_owned()),
                )),
                HandoffDisposition::Accepted
            );
        }

        assert_eq!(controller.incoming().len(), 1);
        assert!(matches!(
            receiver.recv().expect("added event should be published"),
            IncomingHandoffChange::Added(_)
        ));
        assert!(receiver.try_recv().is_err());

        assert_eq!(
            controller.receive(InboundHandoff::new(
                [4_u8; 16],
                device_id,
                HandoffTransportPayload::YouTube {
                    video_id: "invalid".to_owned(),
                    playback_position_millis: 10_000,
                },
            )),
            HandoffDisposition::Rejected(HandoffRejection::Invalid)
        );
        assert_eq!(controller.incoming().len(), 1);
        controller.stop().expect("controller should stop");
    }
}

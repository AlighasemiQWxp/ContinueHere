use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use uuid::Uuid;

use crate::models::ProtocolVersion;

use super::{
    CandidateStore, DiscoveryBackend, DiscoveryBackendEvent, DiscoveryCandidate,
    DiscoveryCandidateId, DiscoveryChange, DiscoveryChangedEvent, DiscoveryError, DiscoveryMode,
    DiscoverySource, DiscoveryStatus, DiscoveryStatusChangedEvent, MdnsDiscoveryBackend,
};

const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(50);

type BackendFactory =
    Arc<dyn Fn() -> Result<Box<dyn DiscoveryBackend>, DiscoveryError> + Send + Sync>;

pub(crate) struct DiscoveryRuntime {
    command_sender: Mutex<Option<mpsc::Sender<RuntimeCommand>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    candidates: Arc<Mutex<CandidateStore>>,
    status: Arc<Mutex<DiscoveryStatus>>,
    changed: DiscoveryChangedEvent,
    status_changed: DiscoveryStatusChangedEvent,
    backend_factory: BackendFactory,
}

impl DiscoveryRuntime {
    pub(crate) fn new(
        changed: DiscoveryChangedEvent,
        status_changed: DiscoveryStatusChangedEvent,
    ) -> Self {
        Self::with_backend_factory(
            changed,
            status_changed,
            Arc::new(|| {
                MdnsDiscoveryBackend::new()
                    .map(|backend| Box::new(backend) as Box<dyn DiscoveryBackend>)
            }),
        )
    }

    fn with_backend_factory(
        changed: DiscoveryChangedEvent,
        status_changed: DiscoveryStatusChangedEvent,
        backend_factory: BackendFactory,
    ) -> Self {
        Self {
            command_sender: Mutex::new(None),
            worker: Mutex::new(None),
            candidates: Arc::new(Mutex::new(CandidateStore::default())),
            status: Arc::new(Mutex::new(DiscoveryStatus::Idle)),
            changed,
            status_changed,
            backend_factory,
        }
    }

    pub(crate) fn start(&self) -> Result<(), DiscoveryError> {
        let mut worker = lock(&self.worker);
        if worker.is_some() {
            return Ok(());
        }

        let (command_sender, command_receiver) = mpsc::channel();
        let candidates = Arc::clone(&self.candidates);
        let status = Arc::clone(&self.status);
        let changed = self.changed.clone();
        let status_changed = self.status_changed.clone();
        let backend_factory = Arc::clone(&self.backend_factory);
        let thread = thread::Builder::new()
            .name("continuehere-discovery".to_owned())
            .spawn(move || {
                run_worker(
                    command_receiver,
                    candidates,
                    status,
                    changed,
                    status_changed,
                    backend_factory,
                );
            })
            .map_err(|_| DiscoveryError::BackendUnavailable)?;
        *lock(&self.command_sender) = Some(command_sender);
        *worker = Some(thread);
        Ok(())
    }

    pub(crate) fn stop(&self) -> Result<(), DiscoveryError> {
        let sender = lock(&self.command_sender).take();
        if let Some(sender) = sender {
            let _send_result = sender.send(RuntimeCommand::Shutdown);
        }
        let worker = lock(&self.worker).take();
        if let Some(worker) = worker {
            worker
                .join()
                .map_err(|_| DiscoveryError::WorkerStopFailed)?;
        }
        Ok(())
    }

    pub(crate) fn activate(
        &self,
        identifier: String,
        mode: DiscoveryMode,
    ) -> Result<(), DiscoveryError> {
        self.send(RuntimeCommand::Activate { identifier, mode })
    }

    pub(crate) fn deactivate(&self, identifier: String) -> Result<(), DiscoveryError> {
        self.send(RuntimeCommand::Deactivate { identifier })
    }

    pub(crate) fn candidates(&self) -> Vec<DiscoveryCandidate> {
        lock(&self.candidates).candidates()
    }

    pub(crate) fn status(&self) -> DiscoveryStatus {
        *lock(&self.status)
    }

    pub(crate) fn is_running(&self) -> bool {
        lock(&self.command_sender).is_some()
    }

    fn send(&self, command: RuntimeCommand) -> Result<(), DiscoveryError> {
        let sender = lock(&self.command_sender);
        let sender = sender.as_ref().ok_or(DiscoveryError::ManagerUnavailable)?;
        sender
            .send(command)
            .map_err(|_| DiscoveryError::CommandChannelUnavailable)
    }
}

impl Drop for DiscoveryRuntime {
    fn drop(&mut self) {
        let _stop_result = self.stop();
    }
}

enum RuntimeCommand {
    Activate {
        identifier: String,
        mode: DiscoveryMode,
    },
    Deactivate {
        identifier: String,
    },
    Shutdown,
}

enum ActiveOperation {
    LocalBrowse,
    Manual(DiscoveryCandidateId),
    Advertisement {
        id: DiscoveryCandidateId,
        fullname: String,
    },
}

struct WorkerState {
    backend: Option<Box<dyn DiscoveryBackend>>,
    operations: HashMap<String, ActiveOperation>,
    local_browse_count: usize,
    local_instance_ids: HashSet<DiscoveryCandidateId>,
}

struct Worker {
    state: WorkerState,
    candidates: Arc<Mutex<CandidateStore>>,
    status: Arc<Mutex<DiscoveryStatus>>,
    changed: DiscoveryChangedEvent,
    status_changed: DiscoveryStatusChangedEvent,
    backend_factory: BackendFactory,
}

impl Worker {
    fn new(
        candidates: Arc<Mutex<CandidateStore>>,
        status: Arc<Mutex<DiscoveryStatus>>,
        changed: DiscoveryChangedEvent,
        status_changed: DiscoveryStatusChangedEvent,
        backend_factory: BackendFactory,
    ) -> Self {
        Self {
            state: WorkerState {
                backend: None,
                operations: HashMap::new(),
                local_browse_count: 0,
                local_instance_ids: HashSet::new(),
            },
            candidates,
            status,
            changed,
            status_changed,
            backend_factory,
        }
    }

    fn run(&mut self, command_receiver: mpsc::Receiver<RuntimeCommand>) {
        let mut shutdown = false;
        while !shutdown {
            match command_receiver.recv_timeout(WORKER_POLL_INTERVAL) {
                Ok(command) => {
                    shutdown = self.process_command(command);
                    while !shutdown {
                        let Ok(command) = command_receiver.try_recv() else {
                            break;
                        };
                        shutdown = self.process_command(command);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => shutdown = true,
            }

            if !shutdown {
                self.poll_backend();
            }
        }

        if let Some(mut backend) = self.state.backend.take() {
            let _shutdown_result = backend.shutdown();
        }
        let changes = lock(&self.candidates).clear();
        self.publish_changes(changes);
        self.set_status(DiscoveryStatus::Idle);
    }

    fn process_command(&mut self, command: RuntimeCommand) -> bool {
        match command {
            RuntimeCommand::Activate { identifier, mode } => {
                let _activation_result = self.activate_operation(identifier, mode);
                false
            }
            RuntimeCommand::Deactivate { identifier } => {
                self.deactivate_operation(&identifier);
                false
            }
            RuntimeCommand::Shutdown => true,
        }
    }

    fn activate_operation(
        &mut self,
        identifier: String,
        mode: DiscoveryMode,
    ) -> Result<(), DiscoveryError> {
        if self.state.operations.contains_key(&identifier) {
            return Ok(());
        }

        let mut pending_change = None;
        let operation = match mode {
            DiscoveryMode::LocalBrowse => {
                let should_start = self.state.local_browse_count == 0;
                if !self.ensure_backend() {
                    self.set_status(DiscoveryStatus::Unavailable);
                    return Err(DiscoveryError::BackendUnavailable);
                }
                let start_result = if should_start {
                    let Some(backend) = self.state.backend.as_mut() else {
                        self.set_status(DiscoveryStatus::Unavailable);
                        return Err(DiscoveryError::BackendUnavailable);
                    };
                    backend.start_browse()
                } else {
                    Ok(())
                };
                if let Err(error) = start_result {
                    self.fail_backend();
                    return Err(error);
                }
                self.state.local_browse_count += 1;
                ActiveOperation::LocalBrowse
            }
            DiscoveryMode::ManualEndpoint(endpoint) => {
                let id = DiscoveryCandidateId::new(format!("manual-{}", Uuid::new_v4()))?;
                let candidate = DiscoveryCandidate::new(
                    id.clone(),
                    [endpoint],
                    ProtocolVersion::CURRENT,
                    DiscoverySource::Manual,
                )?;
                match lock(&self.candidates).upsert(candidate) {
                    Ok(change) => pending_change = change,
                    Err(error) => {
                        self.set_status(DiscoveryStatus::Unavailable);
                        return Err(error);
                    }
                }
                ActiveOperation::Manual(id)
            }
            DiscoveryMode::AdvertiseEndpoint(endpoint) => {
                let id = DiscoveryCandidateId::new(Uuid::new_v4().to_string())?;
                if !self.ensure_backend() {
                    self.set_status(DiscoveryStatus::Unavailable);
                    return Err(DiscoveryError::BackendUnavailable);
                }
                let Some(backend) = self.state.backend.as_mut() else {
                    self.set_status(DiscoveryStatus::Unavailable);
                    return Err(DiscoveryError::BackendUnavailable);
                };
                let fullname = match backend.start_advertisement(&id, &endpoint) {
                    Ok(fullname) => fullname,
                    Err(error) => {
                        self.fail_backend();
                        return Err(error);
                    }
                };
                self.state.local_instance_ids.insert(id.clone());
                ActiveOperation::Advertisement { id, fullname }
            }
        };

        let restores_network_status = !matches!(operation, ActiveOperation::Manual(_));
        self.state.operations.insert(identifier, operation);
        if restores_network_status || *lock(&self.status) != DiscoveryStatus::Unavailable {
            self.set_status(DiscoveryStatus::Active);
        }
        if let Some(change) = pending_change {
            self.changed.publish(change);
        }
        Ok(())
    }

    fn deactivate_operation(&mut self, identifier: &str) {
        let Some(operation) = self.state.operations.remove(identifier) else {
            return;
        };
        match operation {
            ActiveOperation::LocalBrowse => {
                self.state.local_browse_count = self.state.local_browse_count.saturating_sub(1);
                if self.state.local_browse_count == 0 {
                    if let Some(backend) = self.state.backend.as_mut() {
                        let _stop_result = backend.stop_browse();
                    }
                    let changes = lock(&self.candidates).remove_source(DiscoverySource::Local);
                    self.publish_changes(changes);
                }
            }
            ActiveOperation::Manual(id) => {
                let change = lock(&self.candidates).remove(&id);
                if let Some(change) = change {
                    self.changed.publish(change);
                }
            }
            ActiveOperation::Advertisement { id, fullname } => {
                if let Some(backend) = self.state.backend.as_mut() {
                    let _stop_result = backend.stop_advertisement(&fullname);
                }
                self.state.local_instance_ids.remove(&id);
            }
        }

        let has_network_operation = self.state.operations.values().any(|operation| {
            matches!(
                operation,
                ActiveOperation::LocalBrowse | ActiveOperation::Advertisement { .. }
            )
        });
        if !has_network_operation {
            if let Some(mut backend) = self.state.backend.take() {
                let _shutdown_result = backend.shutdown();
            }
        }
        if self.state.operations.is_empty() {
            self.set_status(DiscoveryStatus::Idle);
        }
    }

    fn poll_backend(&mut self) {
        let Some(backend) = self.state.backend.as_mut() else {
            return;
        };
        let event = match backend.poll_event(Duration::ZERO) {
            Ok(event) => event,
            Err(_) => {
                self.fail_backend();
                return;
            }
        };

        match event {
            Some(DiscoveryBackendEvent::Resolved {
                id,
                endpoints,
                protocol_version,
            }) => {
                if self.state.local_instance_ids.contains(&id) {
                    return;
                }
                let candidate = match DiscoveryCandidate::new(
                    id,
                    endpoints,
                    protocol_version,
                    DiscoverySource::Local,
                ) {
                    Ok(candidate) => candidate,
                    Err(_) => return,
                };
                let change = lock(&self.candidates).upsert(candidate);
                if let Ok(Some(change)) = change {
                    self.changed.publish(change);
                }
            }
            Some(DiscoveryBackendEvent::Removed(id)) => {
                let change = lock(&self.candidates).remove(&id);
                if let Some(change) = change {
                    self.changed.publish(change);
                }
            }
            None => {}
        }
    }

    fn ensure_backend(&mut self) -> bool {
        if self.state.backend.is_none() {
            self.state.backend = (self.backend_factory)().ok();
        }
        self.state.backend.is_some()
    }

    fn fail_backend(&mut self) {
        if let Some(mut backend) = self.state.backend.take() {
            let _shutdown_result = backend.shutdown();
        }
        let changes = lock(&self.candidates).remove_source(DiscoverySource::Local);
        self.publish_changes(changes);
        self.state
            .operations
            .retain(|_, operation| matches!(operation, ActiveOperation::Manual(_)));
        self.state.local_browse_count = 0;
        self.state.local_instance_ids.clear();
        self.set_status(DiscoveryStatus::Unavailable);
    }

    fn publish_changes(&self, changes: Vec<DiscoveryChange>) {
        for change in changes {
            self.changed.publish(change);
        }
    }

    fn set_status(&self, updated: DiscoveryStatus) {
        let changed = {
            let mut current = lock(&self.status);
            if *current == updated {
                false
            } else {
                *current = updated;
                true
            }
        };
        if changed {
            self.status_changed.publish(updated);
        }
    }
}

fn run_worker(
    command_receiver: mpsc::Receiver<RuntimeCommand>,
    candidates: Arc<Mutex<CandidateStore>>,
    status: Arc<Mutex<DiscoveryStatus>>,
    changed: DiscoveryChangedEvent,
    status_changed: DiscoveryStatusChangedEvent,
    backend_factory: BackendFactory,
) {
    Worker::new(candidates, status, changed, status_changed, backend_factory).run(command_receiver);
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex, mpsc},
        thread,
        time::{Duration, Instant},
    };

    use super::{BackendFactory, DiscoveryBackend, DiscoveryBackendEvent, DiscoveryRuntime};
    use crate::discovery::{
        DiscoveryCandidateId, DiscoveryChangedEvent, DiscoveryEndpoint, DiscoveryError,
        DiscoveryMode, DiscoveryStatus, DiscoveryStatusChangedDelegate,
        DiscoveryStatusChangedEvent,
    };

    #[derive(Default)]
    struct BackendCounts {
        browse_starts: usize,
        browse_stops: usize,
        shutdowns: usize,
    }

    struct FakeBackend {
        counts: Arc<Mutex<BackendCounts>>,
    }

    impl DiscoveryBackend for FakeBackend {
        fn start_browse(&mut self) -> Result<(), DiscoveryError> {
            self.counts
                .lock()
                .expect("counts should remain available")
                .browse_starts += 1;
            Ok(())
        }

        fn stop_browse(&mut self) -> Result<(), DiscoveryError> {
            self.counts
                .lock()
                .expect("counts should remain available")
                .browse_stops += 1;
            Ok(())
        }

        fn start_advertisement(
            &mut self,
            _instance_id: &DiscoveryCandidateId,
            _endpoint: &DiscoveryEndpoint,
        ) -> Result<String, DiscoveryError> {
            Ok("fake._continuehere._tcp.local.".to_owned())
        }

        fn stop_advertisement(&mut self, _advertisement: &str) -> Result<(), DiscoveryError> {
            Ok(())
        }

        fn poll_event(
            &mut self,
            _timeout: Duration,
        ) -> Result<Option<DiscoveryBackendEvent>, DiscoveryError> {
            Ok(None)
        }

        fn shutdown(&mut self) -> Result<(), DiscoveryError> {
            self.counts
                .lock()
                .expect("counts should remain available")
                .shutdowns += 1;
            Ok(())
        }
    }

    #[test]
    fn local_browse_backend_follows_first_and_last_operation() {
        let counts = Arc::new(Mutex::new(BackendCounts::default()));
        let backend_counts = Arc::clone(&counts);
        let backend_factory: BackendFactory = Arc::new(move || {
            Ok(Box::new(FakeBackend {
                counts: Arc::clone(&backend_counts),
            }))
        });
        let runtime = DiscoveryRuntime::with_backend_factory(
            DiscoveryChangedEvent::default(),
            DiscoveryStatusChangedEvent::default(),
            backend_factory,
        );
        runtime.start().expect("runtime should start");

        runtime
            .activate("first".to_owned(), DiscoveryMode::LocalBrowse)
            .expect("first browse should activate");
        runtime
            .activate("second".to_owned(), DiscoveryMode::LocalBrowse)
            .expect("second browse should activate");
        wait_until(|| {
            counts
                .lock()
                .expect("counts should remain available")
                .browse_starts
                == 1
        });

        runtime
            .deactivate("first".to_owned())
            .expect("first browse should deactivate");
        runtime
            .deactivate("second".to_owned())
            .expect("second browse should deactivate");
        wait_until(|| {
            let counts = counts.lock().expect("counts should remain available");
            counts.browse_stops == 1 && counts.shutdowns == 1
        });
        runtime.stop().expect("runtime should stop");
    }

    #[test]
    fn unavailable_backend_publishes_status_without_exposing_backend_details() {
        let status_changed = DiscoveryStatusChangedEvent::default();
        let (sender, receiver) = mpsc::channel();
        let _subscription =
            status_changed.subscribe(DiscoveryStatusChangedDelegate::new(move |status| {
                sender
                    .send(status)
                    .expect("status receiver should remain available");
            }));
        let backend_factory: BackendFactory = Arc::new(|| Err(DiscoveryError::BackendUnavailable));
        let runtime = DiscoveryRuntime::with_backend_factory(
            DiscoveryChangedEvent::default(),
            status_changed,
            backend_factory,
        );
        runtime.start().expect("runtime should start");
        runtime
            .activate("unavailable".to_owned(), DiscoveryMode::LocalBrowse)
            .expect("browse command should be accepted");

        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("unavailable status should be published"),
            DiscoveryStatus::Unavailable
        );
        assert_eq!(runtime.status(), DiscoveryStatus::Unavailable);
        runtime.stop().expect("runtime should stop");
    }

    fn wait_until(condition: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(1);
        while !condition() {
            assert!(Instant::now() < deadline, "condition should become true");
            thread::sleep(Duration::from_millis(10));
        }
    }
}

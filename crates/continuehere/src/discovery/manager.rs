use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;

use crate::{
    core::{error::ModuleError, module::Module},
    handles::{BaseHandleProvider, HandleError},
};

use super::{
    DiscoveryCandidate, DiscoveryChangedDelegate, DiscoveryChangedEvent,
    DiscoveryChangedSubscription, DiscoveryError, DiscoveryHandle, DiscoveryOperation,
    DiscoveryRuntime, DiscoveryStatus, DiscoveryStatusChangedDelegate, DiscoveryStatusChangedEvent,
    DiscoveryStatusChangedSubscription,
};

pub struct DiscoveryManager {
    handles: Mutex<BaseHandleProvider<DiscoveryOperation>>,
    runtime: Arc<DiscoveryRuntime>,
    changed: DiscoveryChangedEvent,
    status_changed: DiscoveryStatusChangedEvent,
}

impl DiscoveryManager {
    pub(crate) fn new() -> Self {
        let changed = DiscoveryChangedEvent::default();
        let status_changed = DiscoveryStatusChangedEvent::default();
        let runtime = Arc::new(DiscoveryRuntime::new(
            changed.clone(),
            status_changed.clone(),
        ));
        Self {
            handles: Mutex::new(BaseHandleProvider::new()),
            runtime,
            changed,
            status_changed,
        }
    }

    pub fn get_handle(&self, identifier: &str) -> Result<DiscoveryHandle, DiscoveryError> {
        if !self.runtime.is_running() {
            return Err(DiscoveryError::ManagerUnavailable);
        }
        let runtime = Arc::clone(&self.runtime);
        let reference = lock(&self.handles)
            .get_handle(identifier, move |identifier| {
                DiscoveryOperation::new(identifier, runtime)
            })
            .map_err(map_handle_error)?;
        Ok(DiscoveryHandle::new(reference))
    }

    pub fn candidates(&self) -> Vec<DiscoveryCandidate> {
        self.runtime.candidates()
    }

    pub fn status(&self) -> DiscoveryStatus {
        self.runtime.status()
    }

    pub fn on_changed(&self, delegate: DiscoveryChangedDelegate) -> DiscoveryChangedSubscription {
        self.changed.subscribe(delegate)
    }

    pub fn on_status_changed(
        &self,
        delegate: DiscoveryStatusChangedDelegate,
    ) -> DiscoveryStatusChangedSubscription {
        self.status_changed.subscribe(delegate)
    }
}

#[async_trait]
impl Module for DiscoveryManager {
    fn name(&self) -> &'static str {
        "discovery"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        self.runtime.start()?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        let release_error = lock(&self.handles)
            .release_all()
            .err()
            .map(map_handle_error);
        let stop_error = self.runtime.stop().err();
        match release_error.or(stop_error) {
            Some(error) => Err(Box::new(error)),
            None => Ok(()),
        }
    }
}

fn map_handle_error(error: HandleError) -> DiscoveryError {
    match error {
        HandleError::EmptyIdentifier => DiscoveryError::EmptyHandleIdentifier,
        HandleError::SynchronizationFailed => DiscoveryError::HandleSynchronizationFailed,
        HandleError::NotConfigured { .. }
        | HandleError::InvalidState { .. }
        | HandleError::Rejected(_) => DiscoveryError::InvalidHandleState,
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use crate::core::module::Module;

    use super::DiscoveryManager;
    use crate::discovery::{
        DiscoveryChange, DiscoveryChangedDelegate, DiscoveryEndpoint, DiscoveryMode,
        DiscoverySource, DiscoveryStatus,
    };

    #[test]
    fn manual_candidate_follows_handle_lifetime() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("test runtime should build");
        runtime.block_on(async {
            let mut manager = DiscoveryManager::new();
            manager.start().await.expect("discovery should start");
            let (sender, receiver) = mpsc::channel();
            let _subscription = manager.on_changed(DiscoveryChangedDelegate::new(move |change| {
                sender
                    .send(change)
                    .expect("change receiver should remain available");
            }));
            let handle = manager
                .get_handle("manual-test")
                .expect("manual handle should be available");
            let endpoint =
                DiscoveryEndpoint::new("127.0.0.1", 5200).expect("endpoint should be valid");
            handle
                .configure(DiscoveryMode::ManualEndpoint(endpoint.clone()))
                .expect("manual handle should configure");
            handle.use_handle().expect("manual handle should start");

            let added = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("candidate should be added");
            let candidate = match added {
                DiscoveryChange::Added(candidate) => candidate,
                _ => panic!("first change should add a candidate"),
            };
            assert_eq!(candidate.source(), DiscoverySource::Manual);
            assert_eq!(candidate.endpoints(), &[endpoint]);
            assert_eq!(manager.candidates(), vec![candidate.clone()]);
            assert_eq!(manager.status(), DiscoveryStatus::Active);

            assert_eq!(handle.release(), Ok(true));
            let removed = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("candidate should be removed");
            assert_eq!(removed, DiscoveryChange::Removed(candidate));
            assert!(manager.candidates().is_empty());
            manager.stop().await.expect("discovery should stop");
        });
    }

    #[test]
    fn dropping_a_handle_releases_its_candidate() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("test runtime should build");
        runtime.block_on(async {
            let mut manager = DiscoveryManager::new();
            manager.start().await.expect("discovery should start");
            let (sender, receiver) = mpsc::channel();
            let _subscription = manager.on_changed(DiscoveryChangedDelegate::new(move |change| {
                sender
                    .send(change)
                    .expect("change receiver should remain available");
            }));
            let handle = manager
                .get_handle("drop-test")
                .expect("manual handle should be available");
            handle
                .configure(DiscoveryMode::ManualEndpoint(
                    DiscoveryEndpoint::new("127.0.0.1", 5200).expect("endpoint should be valid"),
                ))
                .expect("manual handle should configure");
            handle.use_handle().expect("manual handle should start");
            let _added = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("candidate should be added");

            drop(handle);

            assert!(matches!(
                receiver.recv_timeout(Duration::from_secs(1)),
                Ok(DiscoveryChange::Removed(_))
            ));
            manager.stop().await.expect("discovery should stop");
        });
    }
}

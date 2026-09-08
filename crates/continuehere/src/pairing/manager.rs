use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;

use crate::{
    core::{error::ModuleError, module::Module},
    devices::DeviceIdentityCapability,
    discovery::DiscoveryEndpoint,
    handles::BaseHandleProvider,
    models::DeviceId,
    security::SecurityCapability,
    transport::PairingTransportCapability,
};

use super::{
    PairingController, PairingError, PairingHandle, PairingOperation, PairingSession,
    PairingSessionChangedDelegate, PairingSessionChangedEvent, PairingSessionChangedSubscription,
    TrustedDevice, TrustedDeviceChangedDelegate, TrustedDeviceChangedEvent,
    TrustedDeviceChangedSubscription, TrustedDeviceRegistry, handle::map_handle_error,
};

pub struct PairingManager {
    handles: Mutex<BaseHandleProvider<PairingOperation>>,
    controller: Arc<PairingController>,
    transport: PairingTransportCapability,
    session_changed: PairingSessionChangedEvent,
    trusted_changed: TrustedDeviceChangedEvent,
}

impl PairingManager {
    pub(crate) fn new(
        trusted: TrustedDeviceRegistry,
        device_identity: DeviceIdentityCapability,
        security: SecurityCapability,
        transport: PairingTransportCapability,
    ) -> Self {
        let session_changed = PairingSessionChangedEvent::default();
        let trusted_changed = TrustedDeviceChangedEvent::default();
        let controller = PairingController::new(
            trusted,
            device_identity,
            security,
            transport.clone(),
            session_changed.clone(),
            trusted_changed.clone(),
        );
        Self {
            handles: Mutex::new(BaseHandleProvider::new()),
            controller,
            transport,
            session_changed,
            trusted_changed,
        }
    }

    pub fn get_handle(&self, identifier: &str) -> Result<PairingHandle, PairingError> {
        if !self.controller.is_running() {
            return Err(PairingError::ManagerUnavailable);
        }
        let controller = Arc::clone(&self.controller);
        let reference = lock(&self.handles)?
            .get_handle(identifier, {
                let controller = Arc::clone(&controller);
                move |identifier| PairingOperation::new(identifier, controller)
            })
            .map_err(map_handle_error)?;
        Ok(PairingHandle::new(reference, controller))
    }

    pub fn listening_endpoint(&self) -> Result<DiscoveryEndpoint, PairingError> {
        self.transport
            .endpoint()
            .map_err(|_| PairingError::ManagerUnavailable)
    }

    pub fn sessions(&self) -> Vec<PairingSession> {
        self.controller.sessions()
    }

    pub fn trusted_devices(&self) -> Vec<TrustedDevice> {
        self.controller.trusted_devices()
    }

    pub fn remove_trusted_device(&self, device_id: &DeviceId) -> Result<(), PairingError> {
        self.controller.remove_trusted_device(device_id)
    }

    pub fn on_session_changed(
        &self,
        delegate: PairingSessionChangedDelegate,
    ) -> PairingSessionChangedSubscription {
        self.session_changed.subscribe(delegate)
    }

    pub fn on_trusted_device_changed(
        &self,
        delegate: TrustedDeviceChangedDelegate,
    ) -> TrustedDeviceChangedSubscription {
        self.trusted_changed.subscribe(delegate)
    }
}

#[async_trait]
impl Module for PairingManager {
    fn name(&self) -> &'static str {
        "pairing"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        self.controller.start()?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        let release_error = lock(&self.handles)
            .and_then(|mut handles| handles.release_all().map_err(map_handle_error))
            .err();
        let stop_error = self.controller.stop().err();
        match release_error.or(stop_error) {
            Some(error) => Err(Box::new(error)),
            None => Ok(()),
        }
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, PairingError> {
    value
        .lock()
        .map_err(|_| PairingError::HandleSynchronizationFailed)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::Path,
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };

    use tempfile::tempdir;

    use super::PairingManager;
    use crate::{
        core::module::Module,
        devices::DeviceManager,
        discovery::DiscoveryEndpoint,
        models::DeviceId,
        pairing::{PairingMode, PairingState, TrustedDeviceRegistry},
        security::{CredentialStore, SecurityError, SecurityManager},
        transport::TransportManager,
    };

    #[derive(Default)]
    struct MemoryCredentialStore {
        secrets: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl CredentialStore for MemoryCredentialStore {
        fn load(&self, device_id: &DeviceId) -> Result<Option<Vec<u8>>, SecurityError> {
            Ok(self
                .secrets
                .lock()
                .expect("memory credential store should be available")
                .get(device_id.as_str())
                .cloned())
        }

        fn save(&self, device_id: &DeviceId, secret: &[u8]) -> Result<(), SecurityError> {
            self.secrets
                .lock()
                .expect("memory credential store should be available")
                .insert(device_id.as_str().to_owned(), secret.to_vec());
            Ok(())
        }
    }

    struct TestStack {
        devices: DeviceManager,
        security: SecurityManager,
        transport: TransportManager,
        pairing: PairingManager,
    }

    impl TestStack {
        async fn start(path: &Path) -> Self {
            let mut devices = DeviceManager::new(path.to_path_buf());
            let mut security =
                SecurityManager::with_store(Arc::new(MemoryCredentialStore::default()));
            let trusted = TrustedDeviceRegistry::new(path.to_path_buf());
            let mut transport = TransportManager::new(
                devices.capability(),
                security.capability(),
                trusted.lookup(),
            );
            let mut pairing = PairingManager::new(
                trusted,
                devices.capability(),
                security.capability(),
                transport.pairing_capability(),
            );
            devices.start().await.expect("devices should start");
            security.start().await.expect("security should start");
            transport.start().await.expect("transport should start");
            pairing.start().await.expect("pairing should start");
            Self {
                devices,
                security,
                transport,
                pairing,
            }
        }

        async fn stop(mut self) {
            self.pairing.stop().await.expect("pairing should stop");
            self.transport.stop().await.expect("transport should stop");
            self.security.stop().await.expect("security should stop");
            self.devices.stop().await.expect("devices should stop");
        }
    }

    #[test]
    fn two_peers_pair_through_handle_owned_tls_sessions() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build");
        runtime.block_on(async {
            let first_directory = tempdir().expect("first project directory should be available");
            let second_directory = tempdir().expect("second project directory should be available");
            let first = TestStack::start(first_directory.path()).await;
            let second = TestStack::start(second_directory.path()).await;
            let receiver = second
                .pairing
                .get_handle("receive-first-peer")
                .expect("receiver handle should be available");
            receiver
                .configure(PairingMode::Receive)
                .expect("receiver should configure");
            receiver.use_handle().expect("receiver should start");
            let port = second
                .pairing
                .listening_endpoint()
                .expect("receiver endpoint should be available")
                .port();
            let endpoint = DiscoveryEndpoint::new("127.0.0.1", port)
                .expect("loopback endpoint should be valid");
            let initiator = first
                .pairing
                .get_handle("pair-second-peer")
                .expect("initiator handle should be available");
            initiator
                .configure(PairingMode::Initiate(endpoint))
                .expect("initiator should configure");
            initiator.use_handle().expect("initiator should start");

            assert!(
                wait_until(|| {
                    initiator.session().is_some_and(|session| {
                        session.state() == PairingState::AwaitingVerification
                    }) && receiver.session().is_some_and(|session| {
                        session.state() == PairingState::AwaitingVerification
                    })
                }),
                "pairing sessions should reach verification; initiator: {:?}; receiver: {:?}",
                initiator.session(),
                receiver.session()
            );
            let initiator_verification = initiator
                .session()
                .and_then(|session| session.verification().cloned())
                .expect("initiator verification should be available");
            let receiver_verification = receiver
                .session()
                .and_then(|session| session.verification().cloned())
                .expect("receiver verification should be available");
            assert_eq!(initiator_verification, receiver_verification);

            initiator.approve().expect("initiator should approve");
            receiver.approve().expect("receiver should approve");
            assert!(
                wait_until(|| {
                    first.pairing.trusted_devices().len() == 1
                        && second.pairing.trusted_devices().len() == 1
                        && initiator
                            .session()
                            .is_some_and(|session| session.state() == PairingState::Trusted)
                        && receiver
                            .session()
                            .is_some_and(|session| session.state() == PairingState::Trusted)
                }),
                "pairing sessions should become trusted; first trusted: {}; second trusted: {}; initiator: {:?}; receiver: {:?}",
                first.pairing.trusted_devices().len(),
                second.pairing.trusted_devices().len(),
                initiator.session(),
                receiver.session()
            );

            initiator.release().expect("initiator should release");
            receiver.release().expect("receiver should release");
            assert!(first.pairing.sessions().is_empty());
            assert!(second.pairing.sessions().is_empty());
            first.stop().await;
            second.stop().await;
        });
    }

    fn wait_until(condition: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !condition() {
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
        true
    }
}

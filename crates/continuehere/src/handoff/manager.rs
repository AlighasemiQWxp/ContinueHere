use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;

use crate::{
    core::{error::ModuleError, module::Module},
    handles::BaseHandleProvider,
    transport::HandoffTransportCapability,
};

use super::{
    Handoff, HandoffChangedDelegate, HandoffChangedEvent, HandoffChangedSubscription,
    HandoffController, HandoffError, HandoffHandle, HandoffId, HandoffOperation, IncomingHandoff,
    IncomingHandoffChangedDelegate, IncomingHandoffChangedEvent,
    IncomingHandoffChangedSubscription, handle::map_handle_error,
};

pub struct HandoffManager {
    handles: Mutex<BaseHandleProvider<HandoffOperation>>,
    controller: Arc<HandoffController>,
    handoff_changed: HandoffChangedEvent,
    incoming_changed: IncomingHandoffChangedEvent,
}

impl HandoffManager {
    pub(crate) fn new(transport: HandoffTransportCapability) -> Self {
        let handoff_changed = HandoffChangedEvent::default();
        let incoming_changed = IncomingHandoffChangedEvent::default();
        let controller =
            HandoffController::new(transport, handoff_changed.clone(), incoming_changed.clone());
        Self {
            handles: Mutex::new(BaseHandleProvider::new()),
            controller,
            handoff_changed,
            incoming_changed,
        }
    }

    pub fn get_handle(&self, identifier: &str) -> Result<HandoffHandle, HandoffError> {
        if !self.controller.is_running() {
            return Err(HandoffError::ManagerUnavailable);
        }
        let controller = Arc::clone(&self.controller);
        let reference = lock(&self.handles)?
            .get_handle(identifier, {
                let controller = Arc::clone(&controller);
                move |identifier| HandoffOperation::new(identifier, controller)
            })
            .map_err(map_handle_error)?;
        Ok(HandoffHandle::new(reference, controller))
    }

    pub fn handoffs(&self) -> Vec<Handoff> {
        self.controller.handoffs()
    }

    pub fn incoming(&self) -> Vec<IncomingHandoff> {
        self.controller.incoming()
    }

    pub fn remove_incoming(&self, handoff_id: &HandoffId) -> Result<(), HandoffError> {
        self.controller.remove_incoming(handoff_id)
    }

    pub fn on_handoff_changed(
        &self,
        delegate: HandoffChangedDelegate,
    ) -> HandoffChangedSubscription {
        self.handoff_changed.subscribe(delegate)
    }

    pub fn on_incoming_changed(
        &self,
        delegate: IncomingHandoffChangedDelegate,
    ) -> IncomingHandoffChangedSubscription {
        self.incoming_changed.subscribe(delegate)
    }
}

#[async_trait]
impl Module for HandoffManager {
    fn name(&self) -> &'static str {
        "handoff"
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

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, HandoffError> {
    value
        .lock()
        .map_err(|_| HandoffError::HandleSynchronizationFailed)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex, mpsc},
        time::Duration,
    };

    use tempfile::tempdir;

    use super::HandoffManager;
    use crate::{
        core::module::Module,
        discovery::DiscoveryEndpoint,
        managers::DeviceManager,
        models::{Capability, DeviceId},
        pairing::{TrustedDevice, TrustedDeviceRegistry},
        security::{CredentialStore, SecurityError, SecurityManager},
        transport::TransportManager,
    };

    use crate::handoff::{
        HandoffPayload, HandoffState, IncomingHandoffChange, IncomingHandoffChangedDelegate,
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
                .expect("credential store should be available")
                .get(device_id.as_str())
                .cloned())
        }

        fn save(&self, device_id: &DeviceId, secret: &[u8]) -> Result<(), SecurityError> {
            self.secrets
                .lock()
                .expect("credential store should be available")
                .insert(device_id.as_str().to_owned(), secret.to_vec());
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn handles_send_typed_payloads_to_authenticated_peer() {
        let first_directory = tempdir().expect("first directory should be available");
        let second_directory = tempdir().expect("second directory should be available");
        let mut first_devices = DeviceManager::new(first_directory.path().to_path_buf());
        let mut second_devices = DeviceManager::new(second_directory.path().to_path_buf());
        let mut first_security =
            SecurityManager::with_store(Arc::new(MemoryCredentialStore::default()));
        let mut second_security =
            SecurityManager::with_store(Arc::new(MemoryCredentialStore::default()));
        first_devices.start().await.expect("devices should start");
        second_devices.start().await.expect("devices should start");
        first_security.start().await.expect("security should start");
        second_security
            .start()
            .await
            .expect("security should start");

        let first_identity = first_devices.identity();
        let second_identity = second_devices.identity();
        let first_fingerprint = first_security
            .capability()
            .identity(first_identity.id())
            .expect("identity should be available")
            .fingerprint();
        let second_fingerprint = second_security
            .capability()
            .identity(second_identity.id())
            .expect("identity should be available")
            .fingerprint();
        let first_trusted = TrustedDeviceRegistry::new(first_directory.path().to_path_buf());
        first_trusted.load().expect("trust should load");
        first_trusted
            .persist(TrustedDevice::new(
                second_identity.id().clone(),
                second_fingerprint,
                second_identity.display_name().to_owned(),
                second_identity.platform(),
            ))
            .expect("peer should become trusted");
        let second_trusted = TrustedDeviceRegistry::new(second_directory.path().to_path_buf());
        second_trusted.load().expect("trust should load");
        second_trusted
            .persist(TrustedDevice::new(
                first_identity.id().clone(),
                first_fingerprint,
                first_identity.display_name().to_owned(),
                first_identity.platform(),
            ))
            .expect("peer should become trusted");

        let mut first_transport = TransportManager::new(
            first_devices.capability(),
            first_security.capability(),
            first_trusted.lookup(),
        );
        let mut second_transport = TransportManager::new(
            second_devices.capability(),
            second_security.capability(),
            second_trusted.lookup(),
        );
        let mut first_handoff = HandoffManager::new(first_transport.handoff_capability());
        let mut second_handoff = HandoffManager::new(second_transport.handoff_capability());
        first_transport
            .start()
            .await
            .expect("transport should start");
        second_transport
            .start()
            .await
            .expect("transport should start");
        first_handoff.start().await.expect("handoff should start");
        second_handoff.start().await.expect("handoff should start");

        let endpoint = DiscoveryEndpoint::new(
            "127.0.0.1",
            second_transport
                .listening_endpoint()
                .expect("endpoint should be available")
                .port(),
        )
        .expect("loopback endpoint should be valid");
        let connection = first_transport
            .connect(second_identity.id(), &endpoint)
            .await
            .expect("trusted peers should connect");
        assert!(connection.capabilities().contains(&Capability::UrlHandoff));
        assert!(
            connection
                .capabilities()
                .contains(&Capability::PlaybackPositionHandoff)
        );

        let (event_sender, event_receiver) = mpsc::channel();
        let _subscription = second_handoff.on_incoming_changed(
            IncomingHandoffChangedDelegate::new(move |change| {
                event_sender
                    .send(change)
                    .expect("event receiver should remain available");
            }),
        );
        let handle = first_handoff
            .get_handle("send-example")
            .expect("handoff handle should be available");
        handle
            .configure(
                second_identity.id().clone(),
                "https://example.com/watch?v=1",
            )
            .expect("handoff should configure");
        handle.use_handle().expect("handoff should start");

        wait_until(|| {
            handle
                .handoff()
                .is_some_and(|handoff| handoff.state() == HandoffState::Delivered)
        })
        .await;
        let incoming = event_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("incoming event should be published");
        assert!(matches!(
            incoming,
            IncomingHandoffChange::Added(handoff)
                if handoff.sender_device_id() == first_identity.id()
                    && matches!(
                        handoff.payload(),
                        HandoffPayload::Url(url)
                            if url.url() == "https://example.com/watch?v=1"
                    )
        ));
        let youtube_handle = first_handoff
            .get_handle("continue-youtube")
            .expect("YouTube handoff handle should be available");
        youtube_handle
            .configure_youtube(
                second_identity.id().clone(),
                "https://youtu.be/dQw4w9WgXcQ",
                Duration::from_secs(452),
            )
            .expect("YouTube handoff should configure");
        youtube_handle
            .use_handle()
            .expect("YouTube handoff should start");

        wait_until(|| {
            youtube_handle
                .handoff()
                .is_some_and(|handoff| handoff.state() == HandoffState::Delivered)
        })
        .await;
        let incoming = event_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("YouTube event should be published");
        assert!(matches!(
            incoming,
            IncomingHandoffChange::Added(handoff)
                if matches!(
                    handoff.payload(),
                    HandoffPayload::YouTube(youtube)
                        if youtube.video_id() == "dQw4w9WgXcQ"
                            && youtube.playback_position().duration() == Duration::from_secs(452)
                )
        ));
        assert_eq!(second_handoff.incoming().len(), 2);

        handle.release().expect("handle should release");
        youtube_handle
            .release()
            .expect("YouTube handle should release");
        second_handoff.stop().await.expect("handoff should stop");
        first_handoff.stop().await.expect("handoff should stop");
        second_transport
            .stop()
            .await
            .expect("transport should stop");
        first_transport.stop().await.expect("transport should stop");
        second_security.stop().await.expect("security should stop");
        first_security.stop().await.expect("security should stop");
        second_devices.stop().await.expect("devices should stop");
        first_devices.stop().await.expect("devices should stop");
    }

    async fn wait_until(condition: impl Fn() -> bool) {
        for _ in 0..200 {
            if condition() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(condition(), "condition should become true");
    }
}

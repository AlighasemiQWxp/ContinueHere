use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, Mutex, MutexGuard},
};

use async_trait::async_trait;
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};

use crate::{
    core::{error::ModuleError, module::Module},
    devices::DeviceIdentityCapability,
    discovery::DiscoveryEndpoint,
    models::DeviceId,
    pairing::TrustedPeerLookup,
    security::SecurityCapability,
};

use super::{
    AuthenticatedConnection, ConnectionChangedDelegate, ConnectionChangedEvent,
    ConnectionChangedSubscription, HandoffTransportCapability, PairingTransportCapability,
    TransferTransportCapability, TransportError,
    supervisor::{ConnectionStore, SupervisorCommand, SupervisorContext, SupervisorRuntime},
};

pub struct TransportManager {
    endpoints: Arc<Mutex<super::endpoints::EndpointStore>>,
    pairing: PairingTransportCapability,
    handoff: HandoffTransportCapability,
    transfer: TransferTransportCapability,
    access: Arc<TransportAccess>,
    device_identity: DeviceIdentityCapability,
    security: SecurityCapability,
    trusted_peers: TrustedPeerLookup,
    runtime: Option<SupervisorRuntime>,
}

struct TransportAccess {
    commands: Mutex<Option<mpsc::Sender<SupervisorCommand>>>,
    endpoint: Mutex<Option<DiscoveryEndpoint>>,
    connections: ConnectionStore,
    changed: ConnectionChangedEvent,
}

#[derive(Clone)]
pub(crate) struct ConnectionCapability {
    access: Arc<TransportAccess>,
}

#[derive(Clone)]
pub(crate) struct PairingConnectionCapability {
    access: Arc<TransportAccess>,
    endpoints: Arc<Mutex<super::endpoints::EndpointStore>>,
}

impl ConnectionCapability {
    pub(crate) fn is_connected(&self, device_id: &DeviceId) -> bool {
        lock_or_recover(&self.access.connections).contains_key(device_id)
    }

    pub(crate) fn on_changed(
        &self,
        delegate: ConnectionChangedDelegate,
    ) -> ConnectionChangedSubscription {
        self.access.changed.subscribe(delegate)
    }
}

impl PairingConnectionCapability {
    pub(crate) fn listening_port(&self) -> Result<u16, TransportError> {
        lock(&self.access.endpoint)?
            .as_ref()
            .map(DiscoveryEndpoint::port)
            .ok_or(TransportError::ManagerUnavailable)
    }

    pub(crate) fn remember_verified_endpoint(
        &self,
        device_id: &DeviceId,
        endpoint: &DiscoveryEndpoint,
    ) -> Result<(), TransportError> {
        lock(&self.endpoints)?
            .remember(device_id, endpoint)
            .map_err(|_| TransportError::EndpointStorage)
    }
}

impl TransportManager {
    pub(crate) fn with_endpoint_store(mut self, directory: std::path::PathBuf) -> Self {
        self.endpoints = Arc::new(Mutex::new(super::endpoints::EndpointStore::persistent(
            directory,
        )));
        self
    }

    pub fn known_endpoint(&self, device_id: &DeviceId) -> Option<DiscoveryEndpoint> {
        self.trusted_peers.get(device_id).ok().flatten()?;
        lock_or_recover(&self.endpoints).get(device_id)
    }
    pub(crate) fn connection_capability(&self) -> ConnectionCapability {
        ConnectionCapability {
            access: Arc::clone(&self.access),
        }
    }

    pub(crate) fn pairing_connection_capability(&self) -> PairingConnectionCapability {
        PairingConnectionCapability {
            access: Arc::clone(&self.access),
            endpoints: Arc::clone(&self.endpoints),
        }
    }

    pub(crate) fn new(
        device_identity: DeviceIdentityCapability,
        security: SecurityCapability,
        trusted_peers: TrustedPeerLookup,
    ) -> Self {
        Self {
            endpoints: Arc::new(Mutex::new(super::endpoints::EndpointStore::default())),
            pairing: PairingTransportCapability::new(),
            handoff: HandoffTransportCapability::new(),
            transfer: TransferTransportCapability::new(),
            access: Arc::new(TransportAccess {
                commands: Mutex::new(None),
                endpoint: Mutex::new(None),
                connections: Arc::new(Mutex::new(BTreeMap::new())),
                changed: ConnectionChangedEvent::default(),
            }),
            device_identity,
            security,
            trusted_peers,
            runtime: None,
        }
    }

    pub(crate) fn pairing_capability(&self) -> PairingTransportCapability {
        self.pairing.clone()
    }

    pub(crate) fn handoff_capability(&self) -> HandoffTransportCapability {
        self.handoff.clone()
    }

    pub(crate) fn transfer_capability(&self) -> TransferTransportCapability {
        self.transfer.clone()
    }

    pub fn listening_endpoint(&self) -> Result<DiscoveryEndpoint, TransportError> {
        lock(&self.access.endpoint)?
            .clone()
            .ok_or(TransportError::ManagerUnavailable)
    }

    pub fn connections(&self) -> Vec<AuthenticatedConnection> {
        lock_or_recover(&self.access.connections)
            .values()
            .cloned()
            .collect()
    }

    pub async fn connect(
        &self,
        device_id: &DeviceId,
        endpoint: &DiscoveryEndpoint,
    ) -> Result<AuthenticatedConnection, TransportError> {
        let commands = self.commands()?;
        let (response, result) = oneshot::channel();
        commands
            .send(SupervisorCommand::Connect {
                device_id: device_id.clone(),
                endpoint: endpoint.clone(),
                response,
            })
            .await
            .map_err(|_| TransportError::CommandUnavailable)?;
        let connection = result
            .await
            .map_err(|_| TransportError::CommandUnavailable)??;
        lock(&self.endpoints)?
            .remember(device_id, endpoint)
            .map_err(|_| TransportError::EndpointStorage)?;
        Ok(connection)
    }

    pub async fn disconnect(&self, device_id: &DeviceId) -> Result<(), TransportError> {
        let commands = self.commands()?;
        let (response, result) = oneshot::channel();
        commands
            .send(SupervisorCommand::Disconnect {
                device_id: device_id.clone(),
                response,
            })
            .await
            .map_err(|_| TransportError::CommandUnavailable)?;
        result
            .await
            .map_err(|_| TransportError::CommandUnavailable)?
    }

    pub fn on_connection_changed(
        &self,
        delegate: ConnectionChangedDelegate,
    ) -> ConnectionChangedSubscription {
        self.access.changed.subscribe(delegate)
    }

    pub async fn probe(&self, device_id: &DeviceId) -> Result<(), TransportError> {
        let commands = self.commands()?;
        let (response, result) = oneshot::channel();
        commands
            .send(SupervisorCommand::Probe {
                device_id: device_id.clone(),
                response,
            })
            .await
            .map_err(|_| TransportError::CommandUnavailable)?;
        result
            .await
            .map_err(|_| TransportError::CommandUnavailable)?
    }

    fn commands(&self) -> Result<mpsc::Sender<SupervisorCommand>, TransportError> {
        lock(&self.access.commands)?
            .clone()
            .ok_or(TransportError::ManagerUnavailable)
    }
}

#[async_trait]
impl Module for TransportManager {
    fn name(&self) -> &'static str {
        "transport"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        lock(&self.endpoints)?
            .load()
            .map_err(|_| TransportError::EndpointStorage)?;
        if self.runtime.is_some() {
            return Ok(());
        }
        self.pairing.start()?;
        let local_identity = match self.device_identity.identity() {
            Some(identity) => identity,
            None => {
                self.pairing.stop()?;
                return Err(Box::new(TransportError::ManagerUnavailable));
            }
        };
        let listener = match TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await {
            Ok(listener) => listener,
            Err(_) => {
                self.pairing.stop()?;
                return Err(Box::new(TransportError::ListenerUnavailable));
            }
        };
        let port = match listener.local_addr() {
            Ok(address) => address.port(),
            Err(_) => {
                self.pairing.stop()?;
                return Err(Box::new(TransportError::ListenerUnavailable));
            }
        };
        let endpoint = DiscoveryEndpoint::new("0.0.0.0", port)
            .map_err(|_| Box::new(TransportError::ListenerUnavailable) as ModuleError)?;
        let context = SupervisorContext::new(
            local_identity,
            self.security.clone(),
            self.trusted_peers.clone(),
            self.handoff.clone(),
            self.transfer.clone(),
            Arc::clone(&self.access.connections),
            self.access.changed.clone(),
        );
        let runtime = SupervisorRuntime::start(listener, context);
        *lock_or_recover(&self.access.endpoint) = Some(endpoint);
        let commands = runtime.commands();
        self.handoff.set_commands(Some(commands.clone()));
        self.transfer.set_commands(Some(commands.clone()));
        *lock_or_recover(&self.access.commands) = Some(commands);
        self.runtime = Some(runtime);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        *lock_or_recover(&self.access.commands) = None;
        *lock_or_recover(&self.access.endpoint) = None;
        self.handoff.set_commands(None);
        self.transfer.set_commands(None);
        let runtime_error = match self.runtime.take() {
            Some(runtime) => runtime.stop().await.err(),
            None => None,
        };
        let pairing_error = self.pairing.stop().err();
        match runtime_error.or(pairing_error) {
            Some(error) => Err(Box::new(error)),
            None => Ok(()),
        }
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, TransportError> {
    value
        .lock()
        .map_err(|_| TransportError::SynchronizationFailed)
}

fn lock_or_recover<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::Path,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use tempfile::tempdir;

    use super::{Module, TransportManager};
    use crate::{
        devices::DeviceManager,
        discovery::DiscoveryEndpoint,
        pairing::{TrustedDevice, TrustedDeviceRegistry},
        security::{CredentialStore, SecurityError, SecurityManager},
    };

    #[derive(Default)]
    struct MemoryCredentialStore {
        secrets: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl CredentialStore for MemoryCredentialStore {
        fn load(
            &self,
            device_id: &crate::models::DeviceId,
        ) -> Result<Option<Vec<u8>>, SecurityError> {
            Ok(self
                .secrets
                .lock()
                .expect("memory credential store should be available")
                .get(device_id.as_str())
                .cloned())
        }

        fn save(
            &self,
            device_id: &crate::models::DeviceId,
            secret: &[u8],
        ) -> Result<(), SecurityError> {
            self.secrets
                .lock()
                .expect("memory credential store should be available")
                .insert(device_id.as_str().to_owned(), secret.to_vec());
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn trusted_peers_connect_probe_and_disconnect_after_revocation() {
        let first_directory = tempdir().expect("first directory should be available");
        let second_directory = tempdir().expect("second directory should be available");
        let mut first_devices = start_devices(first_directory.path()).await;
        let mut second_devices = start_devices(second_directory.path()).await;
        let mut first_security = start_security().await;
        let mut second_security = start_security().await;
        let first_identity = first_devices.identity();
        let second_identity = second_devices.identity();
        let first_fingerprint = first_security
            .capability()
            .identity(first_identity.id())
            .expect("first cryptographic identity should be available")
            .fingerprint();
        let second_fingerprint = second_security
            .capability()
            .identity(second_identity.id())
            .expect("second cryptographic identity should be available")
            .fingerprint();

        let first_trusted = TrustedDeviceRegistry::new(first_directory.path().to_path_buf());
        first_trusted.load().expect("first trust should load");
        first_trusted
            .persist(TrustedDevice::new(
                second_identity.id().clone(),
                second_fingerprint,
                second_identity.display_name().to_owned(),
                second_identity.platform(),
            ))
            .expect("second device should become trusted");
        let second_trusted = TrustedDeviceRegistry::new(second_directory.path().to_path_buf());
        second_trusted.load().expect("second trust should load");
        second_trusted
            .persist(TrustedDevice::new(
                first_identity.id().clone(),
                first_fingerprint,
                first_identity.display_name().to_owned(),
                first_identity.platform(),
            ))
            .expect("first device should become trusted");

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
        first_transport
            .start()
            .await
            .expect("first transport should start");
        second_transport
            .start()
            .await
            .expect("second transport should start");

        let second_listener = second_transport
            .listening_endpoint()
            .expect("second listener should be available");
        let endpoint = DiscoveryEndpoint::new("127.0.0.1", second_listener.port())
            .expect("loopback endpoint should be valid");
        let connected = first_transport
            .connect(second_identity.id(), &endpoint)
            .await
            .expect("trusted peers should connect");

        assert_eq!(connected.device_id(), second_identity.id());
        first_transport
            .probe(second_identity.id())
            .await
            .expect("authenticated probe should succeed");
        wait_until(|| second_transport.connections().len() == 1).await;

        second_trusted
            .remove(first_identity.id())
            .expect("trust revocation should persist");
        wait_until(|| {
            first_transport.connections().is_empty() && second_transport.connections().is_empty()
        })
        .await;

        second_transport
            .stop()
            .await
            .expect("second transport should stop");
        first_transport
            .stop()
            .await
            .expect("first transport should stop");
        second_security
            .stop()
            .await
            .expect("second security should stop");
        first_security
            .stop()
            .await
            .expect("first security should stop");
        second_devices
            .stop()
            .await
            .expect("second devices should stop");
        first_devices
            .stop()
            .await
            .expect("first devices should stop");
    }

    async fn start_devices(path: &Path) -> DeviceManager {
        let mut devices = DeviceManager::new(path.to_path_buf());
        devices.start().await.expect("devices should start");
        devices
    }

    async fn start_security() -> SecurityManager {
        let mut security = SecurityManager::with_store(Arc::new(MemoryCredentialStore::default()));
        security.start().await.expect("security should start");
        security
    }

    async fn wait_until(condition: impl Fn() -> bool) {
        for _ in 0..100 {
            if condition() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(condition(), "condition should become true");
    }
}

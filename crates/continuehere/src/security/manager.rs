use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;

use crate::{
    core::{error::ModuleError, module::Module},
    models::DeviceId,
};

use super::{CredentialStore, CryptographicIdentity, OsCredentialStore, SecurityError};

pub struct SecurityManager {
    capability: SecurityCapability,
}

#[derive(Clone)]
pub(crate) struct SecurityCapability {
    state: Arc<Mutex<SecurityState>>,
    store: Arc<dyn CredentialStore>,
}

struct SecurityState {
    running: bool,
    identity: Option<(DeviceId, Arc<CryptographicIdentity>)>,
}

impl SecurityManager {
    pub(crate) fn new() -> Self {
        Self::with_store(Arc::new(OsCredentialStore))
    }

    pub(crate) fn with_store(store: Arc<dyn CredentialStore>) -> Self {
        Self {
            capability: SecurityCapability {
                state: Arc::new(Mutex::new(SecurityState {
                    running: false,
                    identity: None,
                })),
                store,
            },
        }
    }

    pub(crate) fn capability(&self) -> SecurityCapability {
        self.capability.clone()
    }
}

impl SecurityCapability {
    pub(crate) fn identity(
        &self,
        device_id: &DeviceId,
    ) -> Result<Arc<CryptographicIdentity>, SecurityError> {
        let mut state = lock(&self.state)?;
        if !state.running {
            return Err(SecurityError::ManagerUnavailable);
        }
        if let Some((loaded_id, identity)) = state.identity.as_ref() {
            if loaded_id == device_id {
                return Ok(Arc::clone(identity));
            }
        }

        let identity = match self.store.load(device_id)? {
            Some(private_key) => CryptographicIdentity::from_stored(private_key)?,
            None => {
                let identity = CryptographicIdentity::generate()?;
                self.store.save(device_id, identity.private_key())?;
                identity
            }
        };
        let identity = Arc::new(identity);
        state.identity = Some((device_id.clone(), Arc::clone(&identity)));
        Ok(identity)
    }
}

#[async_trait]
impl Module for SecurityManager {
    fn name(&self) -> &'static str {
        "security"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        lock(&self.capability.state)?.running = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        let mut state = lock(&self.capability.state)?;
        state.running = false;
        state.identity = None;
        Ok(())
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, SecurityError> {
    value
        .lock()
        .map_err(|_| SecurityError::SynchronizationFailed)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};

    use super::{CredentialStore, DeviceId, Module, SecurityError, SecurityManager};

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

    #[tokio::test]
    async fn identity_is_stable_for_the_same_device() {
        let store = std::sync::Arc::new(MemoryCredentialStore::default());
        let mut first = SecurityManager::with_store(store.clone());
        first.start().await.expect("security should start");
        let device_id = DeviceId::new("device-1").expect("identifier should be valid");
        let first_fingerprint = first
            .capability()
            .identity(&device_id)
            .expect("identity should be created")
            .fingerprint();
        first.stop().await.expect("security should stop");

        let mut second = SecurityManager::with_store(store);
        second.start().await.expect("security should restart");
        let second_fingerprint = second
            .capability()
            .identity(&device_id)
            .expect("identity should load")
            .fingerprint();

        assert_eq!(second_fingerprint, first_fingerprint);
    }
}

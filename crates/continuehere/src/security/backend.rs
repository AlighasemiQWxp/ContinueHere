#[cfg(not(target_os = "android"))]
use keyring::{Entry, Error as KeyringError};
#[cfg(target_os = "android")]
use keyring_core::{Entry, Error as KeyringError};

use crate::models::DeviceId;

use super::SecurityError;

const SERVICE_NAME: &str = "ContinueHere";

pub(crate) trait CredentialStore: Send + Sync {
    fn load(&self, device_id: &DeviceId) -> Result<Option<Vec<u8>>, SecurityError>;

    fn save(&self, device_id: &DeviceId, secret: &[u8]) -> Result<(), SecurityError>;
}

pub(crate) struct OsCredentialStore;

impl CredentialStore for OsCredentialStore {
    fn load(&self, device_id: &DeviceId) -> Result<Option<Vec<u8>>, SecurityError> {
        let entry = entry(device_id)?;
        match entry.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(SecurityError::SecureStorageUnavailable),
        }
    }

    fn save(&self, device_id: &DeviceId, secret: &[u8]) -> Result<(), SecurityError> {
        entry(device_id)?
            .set_secret(secret)
            .map_err(|_| SecurityError::SecureStorageUnavailable)
    }
}

fn entry(device_id: &DeviceId) -> Result<Entry, SecurityError> {
    #[cfg(target_os = "android")]
    initialize_android_store()?;
    Entry::new(SERVICE_NAME, device_id.as_str())
        .map_err(|_| SecurityError::SecureStorageUnavailable)
}

#[cfg(target_os = "android")]
fn initialize_android_store() -> Result<(), SecurityError> {
    use std::sync::OnceLock;

    static INITIALIZED: OnceLock<Result<(), ()>> = OnceLock::new();
    match INITIALIZED.get_or_init(|| {
        let store = android_native_keyring_store::Store::new().map_err(|_| ())?;
        keyring_core::set_default_store(store);
        Ok(())
    }) {
        Ok(()) => Ok(()),
        Err(()) => Err(SecurityError::SecureStorageUnavailable),
    }
}

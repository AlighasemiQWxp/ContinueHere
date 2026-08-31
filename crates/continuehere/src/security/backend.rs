use keyring::{Entry, Error as KeyringError};

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
    Entry::new(SERVICE_NAME, device_id.as_str())
        .map_err(|_| SecurityError::SecureStorageUnavailable)
}

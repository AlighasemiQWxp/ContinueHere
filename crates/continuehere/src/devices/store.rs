use std::{fs, io::Write, mem::size_of, path::PathBuf};

use atomic_write_file::AtomicWriteFile;
use uuid::Uuid;

use crate::models::{DeviceId, LocalDeviceIdentity, MAX_DISPLAY_NAME_SIZE, Platform};

use super::DeviceIdentityError;

const FILE_MAGIC: &[u8; 7] = b"CHIDBIN";
const FORMAT_VERSION: u16 = 1;
const MAX_FILE_SIZE: u64 = 1024;
const MAX_IDENTIFIER_SIZE: usize = 64;

pub(super) struct DeviceIdentityStore {
    path: PathBuf,
}

impl DeviceIdentityStore {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(super) fn load_or_create(
        &self,
        platform: Platform,
    ) -> Result<LocalDeviceIdentity, DeviceIdentityError> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return self.create(platform);
            }
            Err(source) => {
                return Err(DeviceIdentityError::FileOperation {
                    operation: "inspect",
                    path: self.path.clone(),
                    source,
                });
            }
        };

        if metadata.len() > MAX_FILE_SIZE {
            return Err(DeviceIdentityError::FileTooLarge {
                size: metadata.len(),
                maximum: MAX_FILE_SIZE,
            });
        }

        let bytes = fs::read(&self.path).map_err(|source| DeviceIdentityError::FileOperation {
            operation: "read",
            path: self.path.clone(),
            source,
        })?;
        decode_identity(&bytes, platform)
    }

    pub(super) fn save(&self, identity: &LocalDeviceIdentity) -> Result<(), DeviceIdentityError> {
        let bytes = encode_identity(identity)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| DeviceIdentityError::FileOperation {
                operation: "create the device identity directory for",
                path: self.path.clone(),
                source,
            })?;
        }

        let mut file = AtomicWriteFile::open(&self.path).map_err(|source| {
            DeviceIdentityError::FileOperation {
                operation: "open",
                path: self.path.clone(),
                source,
            }
        })?;
        file.write_all(&bytes)
            .map_err(|source| DeviceIdentityError::FileOperation {
                operation: "write",
                path: self.path.clone(),
                source,
            })?;
        file.commit()
            .map_err(|source| DeviceIdentityError::FileOperation {
                operation: "commit",
                path: self.path.clone(),
                source,
            })?;
        Ok(())
    }

    fn create(&self, platform: Platform) -> Result<LocalDeviceIdentity, DeviceIdentityError> {
        let id = DeviceId::new(Uuid::new_v4().hyphenated().to_string())
            .map_err(|_| DeviceIdentityError::InvalidIdentifier)?;
        let identity = LocalDeviceIdentity::new(id, initial_display_name(platform), platform)?;
        self.save(&identity)?;
        Ok(identity)
    }
}

fn initial_display_name(platform: Platform) -> String {
    for variable in ["COMPUTERNAME", "HOSTNAME"] {
        if let Ok(value) = std::env::var(variable) {
            let value = value.trim();
            if !value.is_empty() && value.len() <= MAX_DISPLAY_NAME_SIZE {
                return value.to_owned();
            }
        }
    }
    platform.default_device_name().to_owned()
}

fn encode_identity(identity: &LocalDeviceIdentity) -> Result<Vec<u8>, DeviceIdentityError> {
    let identifier = identity.id().as_str().as_bytes();
    let display_name = identity.display_name().as_bytes();
    if identifier.is_empty() || identifier.len() > MAX_IDENTIFIER_SIZE {
        return Err(DeviceIdentityError::InvalidIdentifier);
    }
    if display_name.is_empty() || display_name.len() > MAX_DISPLAY_NAME_SIZE {
        return Err(DeviceIdentityError::InvalidDisplayNameData);
    }

    let identifier_length =
        u16::try_from(identifier.len()).map_err(|_| DeviceIdentityError::InvalidIdentifier)?;
    let display_name_length = u16::try_from(display_name.len())
        .map_err(|_| DeviceIdentityError::InvalidDisplayNameData)?;

    let mut bytes = Vec::new();
    bytes.extend_from_slice(FILE_MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&identifier_length.to_le_bytes());
    bytes.extend_from_slice(identifier);
    bytes.extend_from_slice(&display_name_length.to_le_bytes());
    bytes.extend_from_slice(display_name);
    Ok(bytes)
}

fn decode_identity(
    bytes: &[u8],
    platform: Platform,
) -> Result<LocalDeviceIdentity, DeviceIdentityError> {
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(DeviceIdentityError::FileTooLarge {
            size: bytes.len() as u64,
            maximum: MAX_FILE_SIZE,
        });
    }

    let mut cursor = 0;
    if take(bytes, &mut cursor, FILE_MAGIC.len())? != FILE_MAGIC {
        return Err(DeviceIdentityError::InvalidHeader);
    }

    let version = read_u16(bytes, &mut cursor)?;
    if version != FORMAT_VERSION {
        return Err(DeviceIdentityError::UnsupportedFormatVersion { version });
    }

    let identifier_length = usize::from(read_u16(bytes, &mut cursor)?);
    if identifier_length == 0 || identifier_length > MAX_IDENTIFIER_SIZE {
        return Err(DeviceIdentityError::InvalidIdentifier);
    }
    let identifier_bytes = take(bytes, &mut cursor, identifier_length)?;
    let identifier_text = std::str::from_utf8(identifier_bytes)
        .map_err(|_| DeviceIdentityError::InvalidIdentifier)?;
    let uuid =
        Uuid::parse_str(identifier_text).map_err(|_| DeviceIdentityError::InvalidIdentifier)?;
    if uuid.hyphenated().to_string() != identifier_text {
        return Err(DeviceIdentityError::InvalidIdentifier);
    }
    let id = DeviceId::new(identifier_text).map_err(|_| DeviceIdentityError::InvalidIdentifier)?;

    let display_name_length = usize::from(read_u16(bytes, &mut cursor)?);
    if display_name_length == 0 || display_name_length > MAX_DISPLAY_NAME_SIZE {
        return Err(DeviceIdentityError::InvalidDisplayNameData);
    }
    let display_name_bytes = take(bytes, &mut cursor, display_name_length)?;
    let display_name = std::str::from_utf8(display_name_bytes)
        .map_err(|_| DeviceIdentityError::InvalidDisplayNameData)?
        .to_owned();

    if cursor != bytes.len() {
        return Err(DeviceIdentityError::TrailingData);
    }

    let identity = LocalDeviceIdentity::new(id, display_name.clone(), platform)?;
    if identity.display_name() != display_name {
        return Err(DeviceIdentityError::InvalidDisplayNameData);
    }
    Ok(identity)
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], DeviceIdentityError> {
    let end = cursor
        .checked_add(length)
        .ok_or(DeviceIdentityError::TruncatedFile)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(DeviceIdentityError::TruncatedFile)?;
    *cursor = end;
    Ok(value)
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, DeviceIdentityError> {
    let value = take(bytes, cursor, size_of::<u16>())?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{DeviceIdentityStore, FILE_MAGIC, FORMAT_VERSION};
    use crate::models::Platform;

    #[test]
    fn store_creates_and_reloads_the_same_identity() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("device_identity.bin");
        let store = DeviceIdentityStore::new(path.clone());

        let created = store
            .load_or_create(Platform::Windows)
            .expect("identity should be created");
        let loaded = store
            .load_or_create(Platform::Windows)
            .expect("identity should be loaded");

        assert_eq!(loaded, created);
        assert!(path.is_file());
    }

    #[test]
    fn separate_stores_create_different_identifiers() {
        let first_directory = tempdir().expect("first temporary directory should be available");
        let second_directory = tempdir().expect("second temporary directory should be available");
        let first = DeviceIdentityStore::new(first_directory.path().join("device_identity.bin"))
            .load_or_create(Platform::Windows)
            .expect("first identity should be created");
        let second = DeviceIdentityStore::new(second_directory.path().join("device_identity.bin"))
            .load_or_create(Platform::Windows)
            .expect("second identity should be created");

        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn malformed_identity_is_rejected_and_preserved() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("device_identity.bin");
        fs::write(&path, b"malformed").expect("malformed fixture should be written");
        let store = DeviceIdentityStore::new(path.clone());

        assert!(store.load_or_create(Platform::Windows).is_err());
        assert_eq!(
            fs::read(path).expect("malformed fixture should remain"),
            b"malformed"
        );
    }

    #[test]
    fn unsupported_identity_version_is_rejected() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("device_identity.bin");
        let mut bytes = FILE_MAGIC.to_vec();
        bytes.extend_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());
        fs::write(&path, bytes).expect("version fixture should be written");
        let store = DeviceIdentityStore::new(path);

        assert!(store.load_or_create(Platform::Windows).is_err());
    }

    #[test]
    fn trailing_identity_data_is_rejected() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("device_identity.bin");
        let store = DeviceIdentityStore::new(path.clone());
        store
            .load_or_create(Platform::Windows)
            .expect("identity should be created");
        let mut bytes = fs::read(&path).expect("identity fixture should be readable");
        bytes.push(0);
        fs::write(&path, bytes).expect("identity fixture should be changed");

        assert!(store.load_or_create(Platform::Windows).is_err());
    }
}

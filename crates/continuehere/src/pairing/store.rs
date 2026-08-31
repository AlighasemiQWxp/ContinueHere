use std::{collections::BTreeMap, fs, io::Write, mem::size_of, path::PathBuf};

use atomic_write_file::AtomicWriteFile;

use crate::models::{DeviceId, Platform};

use super::{PairingError, TrustedDevice};

const FILE_MAGIC: &[u8; 7] = b"CHTRBIN";
const FORMAT_VERSION: u16 = 1;
const MAX_FILE_SIZE: u64 = 64 * 1024;
const MAX_TRUSTED_DEVICES: usize = 128;
const MAX_IDENTIFIER_SIZE: usize = 64;
const MAX_DISPLAY_NAME_SIZE: usize = 128;

pub(crate) struct TrustedDeviceStore {
    path: PathBuf,
}

impl TrustedDeviceStore {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn load(&self) -> Result<BTreeMap<DeviceId, TrustedDevice>, PairingError> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new());
            }
            Err(source) => return Err(self.file_error("inspect", source)),
        };
        if metadata.len() > MAX_FILE_SIZE {
            return Err(PairingError::FileTooLarge {
                size: metadata.len(),
                maximum: MAX_FILE_SIZE,
            });
        }
        let bytes = fs::read(&self.path).map_err(|source| self.file_error("read", source))?;
        decode(&bytes)
    }

    pub(crate) fn save(
        &self,
        trusted: &BTreeMap<DeviceId, TrustedDevice>,
    ) -> Result<(), PairingError> {
        let bytes = encode(trusted)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|source| self.file_error("create the directory for", source))?;
        }
        let mut file =
            AtomicWriteFile::open(&self.path).map_err(|source| self.file_error("open", source))?;
        file.write_all(&bytes)
            .map_err(|source| self.file_error("write", source))?;
        file.commit()
            .map_err(|source| self.file_error("commit", source))
    }

    fn file_error(&self, operation: &'static str, source: std::io::Error) -> PairingError {
        PairingError::FileOperation {
            operation,
            path: self.path.clone(),
            source,
        }
    }
}

fn encode(trusted: &BTreeMap<DeviceId, TrustedDevice>) -> Result<Vec<u8>, PairingError> {
    if trusted.len() > MAX_TRUSTED_DEVICES {
        return Err(PairingError::TrustedDeviceLimit);
    }
    let count = u16::try_from(trusted.len()).map_err(|_| PairingError::TrustedDeviceLimit)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(FILE_MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    for device in trusted.values() {
        write_text(&mut bytes, device.device_id().as_str(), MAX_IDENTIFIER_SIZE)?;
        bytes.extend_from_slice(device.public_key_fingerprint());
        write_text(&mut bytes, device.display_name(), MAX_DISPLAY_NAME_SIZE)?;
        bytes.push(encode_platform(device.platform()));
    }
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(PairingError::FileTooLarge {
            size: bytes.len() as u64,
            maximum: MAX_FILE_SIZE,
        });
    }
    Ok(bytes)
}

fn decode(bytes: &[u8]) -> Result<BTreeMap<DeviceId, TrustedDevice>, PairingError> {
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(PairingError::FileTooLarge {
            size: bytes.len() as u64,
            maximum: MAX_FILE_SIZE,
        });
    }
    let mut cursor = 0;
    if take(bytes, &mut cursor, FILE_MAGIC.len())? != FILE_MAGIC {
        return Err(PairingError::InvalidHeader);
    }
    let version = read_u16(bytes, &mut cursor)?;
    if version != FORMAT_VERSION {
        return Err(PairingError::UnsupportedFormatVersion { version });
    }
    let count = usize::from(read_u16(bytes, &mut cursor)?);
    if count > MAX_TRUSTED_DEVICES {
        return Err(PairingError::TrustedDeviceLimit);
    }
    let mut trusted = BTreeMap::new();
    let mut fingerprints = Vec::new();
    for _ in 0..count {
        let device_id = DeviceId::new(read_text(bytes, &mut cursor, MAX_IDENTIFIER_SIZE)?)
            .map_err(|_| PairingError::InvalidData)?;
        let fingerprint_bytes = take(bytes, &mut cursor, 32)?;
        let mut fingerprint = [0_u8; 32];
        fingerprint.copy_from_slice(fingerprint_bytes);
        if fingerprints.contains(&fingerprint) {
            return Err(PairingError::InvalidData);
        }
        fingerprints.push(fingerprint);
        let display_name = read_text(bytes, &mut cursor, MAX_DISPLAY_NAME_SIZE)?;
        let platform = decode_platform(read_u8(bytes, &mut cursor)?)?;
        let device = TrustedDevice::new(device_id.clone(), fingerprint, display_name, platform);
        if trusted.insert(device_id, device).is_some() {
            return Err(PairingError::InvalidData);
        }
    }
    if cursor != bytes.len() {
        return Err(PairingError::TrailingData);
    }
    Ok(trusted)
}

fn write_text(bytes: &mut Vec<u8>, value: &str, maximum: usize) -> Result<(), PairingError> {
    if value.is_empty() || value.len() > maximum || value.trim() != value {
        return Err(PairingError::InvalidData);
    }
    let length = u16::try_from(value.len()).map_err(|_| PairingError::InvalidData)?;
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_text(bytes: &[u8], cursor: &mut usize, maximum: usize) -> Result<String, PairingError> {
    let length = usize::from(read_u16(bytes, cursor)?);
    if length == 0 || length > maximum {
        return Err(PairingError::InvalidData);
    }
    let text =
        std::str::from_utf8(take(bytes, cursor, length)?).map_err(|_| PairingError::InvalidData)?;
    if text.trim() != text {
        return Err(PairingError::InvalidData);
    }
    Ok(text.to_owned())
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, length: usize) -> Result<&'a [u8], PairingError> {
    let end = cursor
        .checked_add(length)
        .ok_or(PairingError::TruncatedFile)?;
    let value = bytes.get(*cursor..end).ok_or(PairingError::TruncatedFile)?;
    *cursor = end;
    Ok(value)
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, PairingError> {
    Ok(take(bytes, cursor, size_of::<u8>())?[0])
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, PairingError> {
    let value = take(bytes, cursor, size_of::<u16>())?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn encode_platform(platform: Platform) -> u8 {
    match platform {
        Platform::Windows => 1,
        Platform::Linux => 2,
        Platform::MacOs => 3,
        Platform::Android => 4,
        Platform::Ios => 5,
        Platform::Unknown => 0,
    }
}

fn decode_platform(value: u8) -> Result<Platform, PairingError> {
    match value {
        0 => Ok(Platform::Unknown),
        1 => Ok(Platform::Windows),
        2 => Ok(Platform::Linux),
        3 => Ok(Platform::MacOs),
        4 => Ok(Platform::Android),
        5 => Ok(Platform::Ios),
        _ => Err(PairingError::InvalidData),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::TrustedDeviceStore;
    use crate::{
        models::{DeviceId, Platform},
        pairing::TrustedDevice,
    };

    #[test]
    fn trust_records_round_trip_and_malformed_data_is_preserved() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("trusted_devices.bin");
        let store = TrustedDeviceStore::new(path.clone());
        let device_id = DeviceId::new("device-1").expect("identifier should be valid");
        let device = TrustedDevice::new(
            device_id.clone(),
            [7_u8; 32],
            "Laptop".to_owned(),
            Platform::Windows,
        );
        let mut records = std::collections::BTreeMap::new();
        records.insert(device_id, device.clone());

        store.save(&records).expect("trust should save");
        assert_eq!(
            store.load().expect("trust should load").values().next(),
            Some(&device)
        );

        fs::write(&path, b"malformed").expect("malformed fixture should be written");
        assert!(store.load().is_err());
        assert_eq!(fs::read(path).expect("fixture should remain"), b"malformed");
    }
}

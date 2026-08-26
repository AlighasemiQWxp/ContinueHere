use std::{collections::BTreeMap, fs, io::Write, mem::size_of, path::PathBuf};

use atomic_write_file::AtomicWriteFile;

use super::error::SettingsError;

const FILE_MAGIC: &[u8; 8] = b"CHSETBIN";
const FORMAT_VERSION: u16 = 1;
const MAX_FILE_SIZE: u64 = 1024 * 1024;
const MAX_SECTION_COUNT: usize = 256;
const MAX_SECTION_NAME_SIZE: usize = 64;
const MAX_SECTION_PAYLOAD_SIZE: usize = 1024 * 1024;

#[derive(Clone)]
pub(super) struct StoredSection {
    version: u16,
    payload: Vec<u8>,
}

impl StoredSection {
    pub(super) fn version(&self) -> u16 {
        self.version
    }

    pub(super) fn payload(&self) -> &[u8] {
        &self.payload
    }
}

pub(super) struct SettingsStore {
    path: PathBuf,
    sections: BTreeMap<String, StoredSection>,
}

impl SettingsStore {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            sections: BTreeMap::new(),
        }
    }

    pub(super) fn load(&mut self) -> Result<(), SettingsError> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                self.sections.clear();
                return Ok(());
            }
            Err(source) => {
                return Err(SettingsError::FileOperation {
                    operation: "inspect",
                    path: self.path.clone(),
                    source,
                });
            }
        };

        if metadata.len() > MAX_FILE_SIZE {
            return Err(SettingsError::FileTooLarge {
                size: metadata.len(),
                maximum: MAX_FILE_SIZE,
            });
        }

        let bytes = fs::read(&self.path).map_err(|source| SettingsError::FileOperation {
            operation: "read",
            path: self.path.clone(),
            source,
        })?;
        if bytes.len() as u64 > MAX_FILE_SIZE {
            return Err(SettingsError::FileTooLarge {
                size: bytes.len() as u64,
                maximum: MAX_FILE_SIZE,
            });
        }

        self.sections = decode_document(&bytes)?;
        Ok(())
    }

    pub(super) fn section(&self, name: &str) -> Option<StoredSection> {
        self.sections.get(name).cloned()
    }

    pub(super) fn write_section(
        &mut self,
        name: &str,
        version: u16,
        payload: Vec<u8>,
    ) -> Result<(), SettingsError> {
        validate_section_name(name)?;
        if payload.len() > MAX_SECTION_PAYLOAD_SIZE {
            return Err(SettingsError::SectionTooLarge {
                name: name.to_owned(),
            });
        }

        let mut sections = self.sections.clone();
        sections.insert(name.to_owned(), StoredSection { version, payload });
        let bytes = encode_document(&sections)?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| SettingsError::FileOperation {
                operation: "create the settings directory for",
                path: self.path.clone(),
                source,
            })?;
        }

        let mut file =
            AtomicWriteFile::open(&self.path).map_err(|source| SettingsError::FileOperation {
                operation: "open",
                path: self.path.clone(),
                source,
            })?;
        file.write_all(&bytes)
            .map_err(|source| SettingsError::FileOperation {
                operation: "write",
                path: self.path.clone(),
                source,
            })?;
        file.commit()
            .map_err(|source| SettingsError::FileOperation {
                operation: "commit",
                path: self.path.clone(),
                source,
            })?;

        self.sections = sections;
        Ok(())
    }
}

fn encode_document(sections: &BTreeMap<String, StoredSection>) -> Result<Vec<u8>, SettingsError> {
    let section_count =
        u16::try_from(sections.len()).map_err(|_| SettingsError::TooManySections)?;
    if sections.len() > MAX_SECTION_COUNT {
        return Err(SettingsError::TooManySections);
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(FILE_MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&section_count.to_le_bytes());

    for (name, section) in sections {
        validate_section_name(name)?;
        let name_length =
            u16::try_from(name.len()).map_err(|_| SettingsError::InvalidSectionName)?;
        let payload_length =
            u32::try_from(section.payload.len()).map_err(|_| SettingsError::SectionTooLarge {
                name: name.to_owned(),
            })?;

        if section.payload.len() > MAX_SECTION_PAYLOAD_SIZE {
            return Err(SettingsError::SectionTooLarge {
                name: name.to_owned(),
            });
        }

        bytes.extend_from_slice(&name_length.to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&section.version.to_le_bytes());
        bytes.extend_from_slice(&payload_length.to_le_bytes());
        bytes.extend_from_slice(&section.payload);
    }

    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(SettingsError::FileTooLarge {
            size: bytes.len() as u64,
            maximum: MAX_FILE_SIZE,
        });
    }

    Ok(bytes)
}

fn decode_document(bytes: &[u8]) -> Result<BTreeMap<String, StoredSection>, SettingsError> {
    let mut cursor = 0;
    if take(bytes, &mut cursor, FILE_MAGIC.len())? != &FILE_MAGIC[..] {
        return Err(SettingsError::InvalidHeader);
    }

    let version = read_u16(bytes, &mut cursor)?;
    if version != FORMAT_VERSION {
        return Err(SettingsError::UnsupportedFormatVersion { version });
    }

    let section_count = usize::from(read_u16(bytes, &mut cursor)?);
    if section_count > MAX_SECTION_COUNT {
        return Err(SettingsError::TooManySections);
    }

    let mut sections = BTreeMap::new();
    for _ in 0..section_count {
        let name_length = usize::from(read_u16(bytes, &mut cursor)?);
        if name_length == 0 || name_length > MAX_SECTION_NAME_SIZE {
            return Err(SettingsError::InvalidSectionName);
        }

        let name_bytes = take(bytes, &mut cursor, name_length)?;
        let name = std::str::from_utf8(name_bytes)
            .map_err(|_| SettingsError::InvalidSectionName)?
            .to_owned();
        validate_section_name(&name)?;

        let version = read_u16(bytes, &mut cursor)?;
        let payload_length = read_u32(bytes, &mut cursor)? as usize;
        if payload_length > MAX_SECTION_PAYLOAD_SIZE {
            return Err(SettingsError::SectionTooLarge { name });
        }
        let payload = take(bytes, &mut cursor, payload_length)?.to_vec();

        if sections
            .insert(name.clone(), StoredSection { version, payload })
            .is_some()
        {
            return Err(SettingsError::DuplicateSection { name });
        }
    }

    if cursor != bytes.len() {
        return Err(SettingsError::TrailingData);
    }

    Ok(sections)
}

fn validate_section_name(name: &str) -> Result<(), SettingsError> {
    let valid = !name.is_empty()
        && name.len() <= MAX_SECTION_NAME_SIZE
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if !valid {
        return Err(SettingsError::InvalidSectionName);
    }
    Ok(())
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, length: usize) -> Result<&'a [u8], SettingsError> {
    let end = cursor
        .checked_add(length)
        .ok_or(SettingsError::TruncatedFile)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(SettingsError::TruncatedFile)?;
    *cursor = end;
    Ok(value)
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, SettingsError> {
    let value = take(bytes, cursor, size_of::<u16>())?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, SettingsError> {
    let value = take(bytes, cursor, size_of::<u32>())?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{FILE_MAGIC, FORMAT_VERSION, MAX_FILE_SIZE, SettingsStore};

    #[test]
    fn store_round_trips_independently_versioned_sections() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("settings.bin");
        let mut store = SettingsStore::new(path.clone());

        store
            .write_section("future_system", 7, vec![1, 2, 3])
            .expect("section should save");
        store
            .write_section("directories", 1, b"C:/Transfers".to_vec())
            .expect("section should save");

        let mut loaded = SettingsStore::new(path);
        loaded.load().expect("settings should load");

        let future = loaded
            .section("future_system")
            .expect("unknown section should be preserved");
        assert_eq!(future.version(), 7);
        assert_eq!(future.payload(), [1, 2, 3]);
        assert_eq!(
            loaded
                .section("directories")
                .expect("directory section should exist")
                .payload(),
            b"C:/Transfers"
        );
    }

    #[test]
    fn missing_file_loads_an_empty_document() {
        let directory = tempdir().expect("temporary directory should be available");
        let mut store = SettingsStore::new(directory.path().join("settings.bin"));

        store.load().expect("missing settings should use defaults");

        assert!(store.section("directories").is_none());
    }

    #[test]
    fn invalid_header_is_rejected_without_replacing_the_file() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("settings.bin");
        fs::write(&path, b"invalid settings").expect("invalid fixture should be written");
        let mut store = SettingsStore::new(path.clone());

        assert!(store.load().is_err());
        assert_eq!(
            fs::read(path).expect("invalid fixture should remain"),
            b"invalid settings"
        );
    }

    #[test]
    fn unsupported_format_version_is_rejected() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("settings.bin");
        let mut bytes = FILE_MAGIC.to_vec();
        bytes.extend_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        fs::write(&path, bytes).expect("version fixture should be written");
        let mut store = SettingsStore::new(path);

        assert!(store.load().is_err());
    }

    #[test]
    fn oversized_file_is_rejected_before_decoding() {
        let directory = tempdir().expect("temporary directory should be available");
        let path = directory.path().join("settings.bin");
        fs::write(&path, vec![0; MAX_FILE_SIZE as usize + 1])
            .expect("oversized fixture should be written");
        let mut store = SettingsStore::new(path);

        assert!(store.load().is_err());
    }
}

use std::{fmt, path::PathBuf};

use uuid::Uuid;

use crate::models::DeviceId;

use super::FileTransferError;

pub(crate) const MAX_FILE_NAME_SIZE: usize = 255;
pub(crate) const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024 * 1024;
pub(crate) const TRANSFER_CHUNK_SIZE: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileTransferId {
    value: String,
    bytes: [u8; 16],
}

impl FileTransferId {
    pub(crate) fn new() -> Self {
        Self::from_uuid(Uuid::new_v4())
    }

    pub(crate) fn from_bytes(bytes: [u8; 16]) -> Self {
        Self::from_uuid(Uuid::from_bytes(bytes))
    }

    pub(crate) const fn bytes(&self) -> [u8; 16] {
        self.bytes
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    fn from_uuid(uuid: Uuid) -> Self {
        Self {
            value: uuid.hyphenated().to_string(),
            bytes: *uuid.as_bytes(),
        }
    }
}

impl fmt::Display for FileTransferId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileTransferDirection {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileTransferState {
    Preparing,
    WaitingForAcceptance,
    Offered,
    Transferring,
    Verifying,
    Completed,
    Rejected,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileTransferFailure {
    NotConnected,
    Unsupported,
    Invalid,
    Busy,
    Declined,
    TimedOut,
    DestinationConflict,
    Integrity,
    FileSystem,
    Transport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTransfer {
    folder: bool,
    id: FileTransferId,
    peer_device_id: DeviceId,
    file_name: String,
    file_size: u64,
    transferred_bytes: u64,
    direction: FileTransferDirection,
    state: FileTransferState,
    failure: Option<FileTransferFailure>,
    destination: Option<PathBuf>,
    source: Option<PathBuf>,
}

impl FileTransfer {
    pub(crate) fn outgoing(id: FileTransferId, config: &FileTransferConfig) -> Self {
        Self {
            folder: config.folder,
            id,
            peer_device_id: config.device_id.clone(),
            file_name: config.file_name.clone(),
            file_size: config.file_size,
            transferred_bytes: 0,
            direction: FileTransferDirection::Outgoing,
            state: FileTransferState::Preparing,
            failure: None,
            destination: None,
            source: Some(config.source.clone()),
        }
    }

    pub(crate) fn incoming(
        id: FileTransferId,
        peer_device_id: DeviceId,
        file_name: String,
        file_size: u64,
    ) -> Self {
        Self {
            folder: false,
            id,
            peer_device_id,
            file_name,
            file_size,
            transferred_bytes: 0,
            direction: FileTransferDirection::Incoming,
            state: FileTransferState::Offered,
            failure: None,
            destination: None,
            source: None,
        }
    }

    pub fn id(&self) -> &FileTransferId {
        &self.id
    }

    pub const fn is_folder(&self) -> bool {
        self.folder
    }

    pub(crate) fn set_folder(&mut self) {
        self.folder = true;
    }
    pub(crate) fn set_size(&mut self, size: u64) {
        self.file_size = size;
    }

    pub fn peer_device_id(&self) -> &DeviceId {
        &self.peer_device_id
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub const fn file_size(&self) -> u64 {
        self.file_size
    }

    pub const fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    pub const fn direction(&self) -> FileTransferDirection {
        self.direction
    }

    pub const fn state(&self) -> FileTransferState {
        self.state
    }

    pub const fn failure(&self) -> Option<FileTransferFailure> {
        self.failure
    }

    pub fn destination(&self) -> Option<&std::path::Path> {
        self.destination.as_deref()
    }

    pub(crate) fn source(&self) -> Option<&std::path::Path> {
        self.source.as_deref()
    }

    pub(crate) fn set_state(&mut self, state: FileTransferState) {
        self.state = state;
        self.failure = None;
    }

    pub(crate) fn set_progress(&mut self, transferred_bytes: u64) {
        self.transferred_bytes = transferred_bytes;
    }

    pub(crate) fn set_destination(&mut self, destination: PathBuf) {
        self.destination = Some(destination);
    }

    pub(crate) fn reject(&mut self, failure: FileTransferFailure) {
        self.state = FileTransferState::Rejected;
        self.failure = Some(failure);
    }

    pub(crate) fn fail(&mut self, failure: FileTransferFailure) {
        self.state = FileTransferState::Failed;
        self.failure = Some(failure);
    }

    pub(crate) fn cancel(&mut self) {
        self.state = FileTransferState::Cancelled;
        self.failure = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileTransferChange {
    Added(FileTransfer),
    Updated(FileTransfer),
    Removed(FileTransfer),
}

#[derive(Clone)]
pub(crate) struct FileTransferConfig {
    pub(crate) folder: bool,
    pub(crate) device_id: DeviceId,
    pub(crate) source: PathBuf,
    pub(crate) file_name: String,
    pub(crate) file_size: u64,
}

impl FileTransferConfig {
    pub(crate) fn folder(device_id: DeviceId, source: PathBuf) -> Result<Self, FileTransferError> {
        super::folder::validate_root(&source)
            .map_err(|_| FileTransferError::InvalidSourceFolder)?;
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(FileTransferError::InvalidFileName)?
            .to_owned();
        validate_file_name(&file_name)?;
        Ok(Self {
            device_id,
            source,
            file_name,
            file_size: 0,
            folder: true,
        })
    }
    pub(crate) fn new(device_id: DeviceId, source: PathBuf) -> Result<Self, FileTransferError> {
        if !source.is_absolute() {
            return Err(FileTransferError::InvalidSourceFile);
        }
        let metadata = source
            .symlink_metadata()
            .map_err(|_| FileTransferError::InvalidSourceFile)?;
        if !metadata.file_type().is_file() {
            return Err(FileTransferError::InvalidSourceFile);
        }
        if metadata.len() > MAX_FILE_SIZE {
            return Err(FileTransferError::FileTooLarge);
        }
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(FileTransferError::InvalidFileName)?
            .to_owned();
        validate_file_name(&file_name)?;
        Ok(Self {
            folder: false,
            device_id,
            source,
            file_name,
            file_size: metadata.len(),
        })
    }
}

pub(crate) fn validate_file_name(file_name: &str) -> Result<(), FileTransferError> {
    if file_name.is_empty()
        || file_name.len() > MAX_FILE_NAME_SIZE
        || file_name == "."
        || file_name == ".."
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains('\0')
        || file_name.chars().any(|character| {
            character.is_control() || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
        })
        || file_name.ends_with(' ')
        || file_name.ends_with('.')
        || is_reserved_windows_name(file_name)
    {
        return Err(FileTransferError::InvalidFileName);
    }
    Ok(())
}

fn is_reserved_windows_name(file_name: &str) -> bool {
    let stem = file_name.split('.').next().unwrap_or(file_name);
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper.strip_prefix("COM").is_some_and(|number| {
            matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || upper.strip_prefix("LPT").is_some_and(|number| {
            matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}

#[cfg(test)]
mod tests {
    use super::validate_file_name;

    #[test]
    fn portable_file_names_reject_paths_and_reserved_devices() {
        assert!(validate_file_name("report.pdf").is_ok());
        assert!(validate_file_name("../report.pdf").is_err());
        assert!(validate_file_name("folder/report.pdf").is_err());
        assert!(validate_file_name("CON.txt").is_err());
        assert!(validate_file_name("report?.pdf").is_err());
        assert!(validate_file_name("report. ").is_err());
    }
}

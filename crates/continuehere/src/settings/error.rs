use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum SettingsError {
    #[error("settings path `{path}` must be an absolute directory")]
    InvalidProjectDirectory { path: PathBuf },

    #[error("directory path `{path}` must be absolute and non-empty")]
    InvalidDirectoryPath { path: PathBuf },

    #[error("directory path `{path}` cannot be represented in the settings file")]
    UnsupportedDirectoryPath { path: PathBuf },

    #[error("settings state lock is unavailable")]
    StateUnavailable,

    #[error("failed to {operation} settings file `{path}`")]
    FileOperation {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("settings file is {size} bytes, exceeding the {maximum} byte limit")]
    FileTooLarge { size: u64, maximum: u64 },

    #[error("settings file has an invalid header")]
    InvalidHeader,

    #[error("settings file format version {version} is unsupported")]
    UnsupportedFormatVersion { version: u16 },

    #[error("settings file ended before all declared data was read")]
    TruncatedFile,

    #[error("settings file contains trailing data")]
    TrailingData,

    #[error("settings file declares too many sections")]
    TooManySections,

    #[error("settings section name is invalid")]
    InvalidSectionName,

    #[error("settings section `{name}` appears more than once")]
    DuplicateSection { name: String },

    #[error("settings section `{name}` exceeds the payload size limit")]
    SectionTooLarge { name: String },

    #[error("settings section `{name}` uses unsupported version {version}")]
    UnsupportedSectionVersion { name: &'static str, version: u16 },

    #[error("settings section `{name}` contains invalid data")]
    InvalidSectionData { name: &'static str },
}

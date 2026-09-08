use std::{io, path::PathBuf};

use thiserror::Error;

use crate::models::LocalDeviceIdentityError;

#[derive(Debug, Error)]
pub(crate) enum DeviceIdentityError {
    #[error(transparent)]
    InvalidIdentity(#[from] LocalDeviceIdentityError),

    #[error("device identity state is unavailable")]
    IdentityUnavailable,

    #[error("failed to {operation} device identity file `{path}`")]
    FileOperation {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("device identity file is {size} bytes, exceeding the {maximum} byte limit")]
    FileTooLarge { size: u64, maximum: u64 },

    #[error("device identity file has an invalid header")]
    InvalidHeader,

    #[error("device identity file format version {version} is unsupported")]
    UnsupportedFormatVersion { version: u16 },

    #[error("device identity file ended before all declared data was read")]
    TruncatedFile,

    #[error("device identity file contains trailing data")]
    TrailingData,

    #[error("device identity file contains an invalid identifier")]
    InvalidIdentifier,

    #[error("device identity file contains invalid display-name data")]
    InvalidDisplayNameData,
}

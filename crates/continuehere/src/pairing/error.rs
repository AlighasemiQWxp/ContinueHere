use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PairingError {
    #[error("pairing manager is not running")]
    ManagerUnavailable,
    #[error("pairing handle identifier cannot be empty")]
    EmptyHandleIdentifier,
    #[error("pairing handle is not configured")]
    HandleNotConfigured,
    #[error("pairing handle cannot perform that operation in its current state")]
    InvalidHandleState,
    #[error("pairing handle synchronization failed")]
    HandleSynchronizationFailed,
    #[error("pairing session was not found")]
    SessionNotFound,
    #[error("pairing session is not waiting for this operation")]
    InvalidSessionState,
    #[error("pairing session limit has been reached")]
    SessionLimit,
    #[error("trusted-device limit has been reached")]
    TrustedDeviceLimit,
    #[error("device is already trusted with a different cryptographic identity")]
    IdentityConflict,
    #[error("trusted device was not found")]
    TrustedDeviceNotFound,
    #[error("trusted-device state synchronization failed")]
    TrustSynchronizationFailed,
    #[error("pairing command channel is unavailable")]
    CommandChannelUnavailable,
    #[error("pairing worker failed to stop")]
    WorkerStopFailed,
    #[error("trusted-device file is too large: {size} bytes exceeds {maximum}")]
    FileTooLarge { size: u64, maximum: u64 },
    #[error("unsupported trusted-device format version {version}")]
    UnsupportedFormatVersion { version: u16 },
    #[error("trusted-device file has an invalid header")]
    InvalidHeader,
    #[error("trusted-device file is truncated")]
    TruncatedFile,
    #[error("trusted-device file contains invalid data")]
    InvalidData,
    #[error("trusted-device file contains trailing data")]
    TrailingData,
    #[error("failed to {operation} trusted-device file `{path}`")]
    FileOperation {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

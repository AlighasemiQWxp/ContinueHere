use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileTransferError {
    #[error("file-transfer manager is not running")]
    ManagerUnavailable,
    #[error("file-transfer handle identifier cannot be empty")]
    EmptyHandleIdentifier,
    #[error("file-transfer handle is not configured")]
    HandleNotConfigured,
    #[error("file-transfer handle cannot perform that operation in its current state")]
    InvalidHandleState,
    #[error("file-transfer handle synchronization failed")]
    HandleSynchronizationFailed,
    #[error("source path must identify an absolute regular file")]
    InvalidSourceFile,
    #[error("file name is invalid")]
    InvalidFileName,
    #[error("file exceeds the supported size")]
    FileTooLarge,
    #[error("file-transfer operation limit has been reached")]
    OperationLimit,
    #[error("transfer was not found")]
    TransferNotFound,
    #[error("incoming transfer is not awaiting a decision")]
    IncomingTransferNotPending,
    #[error("destination directory is invalid or unavailable")]
    InvalidDestinationDirectory,
    #[error("destination file already exists")]
    DestinationConflict,
    #[error("file-transfer state synchronization failed")]
    SynchronizationFailed,
    #[error("file-transfer command channel is unavailable")]
    CommandUnavailable,
    #[error("file-transfer worker failed to stop")]
    WorkerStopFailed,
}

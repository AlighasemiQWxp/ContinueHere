use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum HandoffError {
    #[error("handoff manager is not running")]
    ManagerUnavailable,
    #[error("handoff handle identifier cannot be empty")]
    EmptyHandleIdentifier,
    #[error("handoff handle is not configured")]
    HandleNotConfigured,
    #[error("handoff handle cannot perform that operation in its current state")]
    InvalidHandleState,
    #[error("handoff handle synchronization failed")]
    HandleSynchronizationFailed,
    #[error("URL cannot be empty")]
    EmptyUrl,
    #[error("URL exceeds the supported size")]
    UrlTooLarge,
    #[error("URL is malformed")]
    InvalidUrl,
    #[error("only HTTP and HTTPS URLs are supported")]
    UnsupportedUrlScheme,
    #[error("URLs containing credentials are not supported")]
    UrlContainsCredentials,
    #[error("handoff operation limit has been reached")]
    OperationLimit,
    #[error("received handoff was not found")]
    IncomingHandoffNotFound,
    #[error("handoff state synchronization failed")]
    SynchronizationFailed,
    #[error("handoff command channel is unavailable")]
    CommandUnavailable,
    #[error("handoff worker failed to stop")]
    WorkerStopFailed,
}

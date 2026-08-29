use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DiscoveryError {
    #[error("discovery manager is not running")]
    ManagerUnavailable,
    #[error("discovery handle identifier cannot be empty")]
    EmptyHandleIdentifier,
    #[error("discovery handle is not configured")]
    HandleNotConfigured,
    #[error("discovery handle cannot perform that operation in its current state")]
    InvalidHandleState,
    #[error("discovery handle synchronization failed")]
    HandleSynchronizationFailed,
    #[error("discovery command channel is unavailable")]
    CommandChannelUnavailable,
    #[error("discovery endpoint is empty")]
    EmptyEndpoint,
    #[error("discovery endpoint exceeds the supported length")]
    EndpointTooLong,
    #[error("discovery endpoint must include a host and port")]
    MissingEndpointPort,
    #[error("discovery endpoint host is invalid")]
    InvalidEndpointHost,
    #[error("discovery endpoint port is invalid")]
    InvalidEndpointPort,
    #[error("discovery candidate identifier is invalid")]
    InvalidCandidateIdentifier,
    #[error("discovery candidate has no usable endpoints")]
    MissingCandidateEndpoint,
    #[error("discovery candidate exceeds the endpoint limit")]
    CandidateEndpointLimit,
    #[error("discovery candidate limit has been reached")]
    CandidateLimit,
    #[error("discovery backend is unavailable")]
    BackendUnavailable,
    #[error("discovery worker failed to stop")]
    WorkerStopFailed,
}

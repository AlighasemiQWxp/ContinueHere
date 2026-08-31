use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TransportError {
    #[error("transport manager is not running")]
    ManagerUnavailable,
    #[error("pairing listener is unavailable")]
    ListenerUnavailable,
    #[error("pairing connection failed")]
    ConnectionFailed,
    #[error("pairing TLS configuration failed")]
    TlsConfigurationFailed,
    #[error("pairing TLS handshake failed")]
    TlsHandshakeFailed,
    #[error("pairing peer did not present one certificate")]
    MissingPeerIdentity,
    #[error("pairing message is malformed")]
    InvalidMessage,
    #[error("pairing message exceeds the supported size")]
    MessageTooLarge,
    #[error("pairing connection timed out")]
    TimedOut,
    #[error("pairing transport synchronization failed")]
    SynchronizationFailed,
    #[error("pairing listener failed to stop")]
    WorkerStopFailed,
}

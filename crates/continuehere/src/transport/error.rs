use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TransportError {
    #[error("transport manager is not running")]
    ManagerUnavailable,
    #[error("pairing listener is unavailable")]
    ListenerUnavailable,
    #[error("transport connection failed")]
    ConnectionFailed,
    #[error("TLS configuration failed")]
    TlsConfigurationFailed,
    #[error("TLS handshake failed")]
    TlsHandshakeFailed,
    #[error("peer did not present one valid certificate")]
    MissingPeerIdentity,
    #[error("transport message is malformed")]
    InvalidMessage,
    #[error("transport message exceeds the supported size")]
    MessageTooLarge,
    #[error("transport operation timed out")]
    TimedOut,
    #[error("transport synchronization failed")]
    SynchronizationFailed,
    #[error("transport worker failed to stop")]
    WorkerStopFailed,
    #[error("trusted-device state is unavailable")]
    TrustUnavailable,
    #[error("peer is not trusted")]
    UntrustedPeer,
    #[error("authenticated identity does not match the trusted device")]
    IdentityMismatch,
    #[error("peer is already connected")]
    AlreadyConnected,
    #[error("peer is not connected")]
    NotConnected,
    #[error("peer uses an incompatible protocol version")]
    IncompatibleProtocol,
    #[error("peer violated the application protocol")]
    ProtocolViolation,
    #[error("transport command channel is unavailable")]
    CommandUnavailable,
    #[error("transport connection limit reached")]
    ConnectionLimit,
}

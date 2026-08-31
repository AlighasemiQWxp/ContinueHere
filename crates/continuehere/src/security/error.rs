use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SecurityError {
    #[error("security manager is not running")]
    ManagerUnavailable,
    #[error("operating-system secure storage is unavailable")]
    SecureStorageUnavailable,
    #[error("stored cryptographic identity is malformed")]
    InvalidStoredIdentity,
    #[error("cryptographic identity generation failed")]
    IdentityGenerationFailed,
    #[error("TLS identity construction failed")]
    TlsIdentityFailed,
    #[error("cryptographic identity synchronization failed")]
    SynchronizationFailed,
}

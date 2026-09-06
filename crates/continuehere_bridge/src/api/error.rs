use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiBridgeErrorKind {
    InvalidInput,
    NotRunning,
    OperationFailed,
    SynchronizationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiBridgeError {
    pub kind: UiBridgeErrorKind,
    pub message: String,
}

impl UiBridgeError {
    pub(crate) fn invalid_input(error: impl fmt::Display) -> Self {
        Self::new(UiBridgeErrorKind::InvalidInput, error)
    }

    pub(crate) fn not_running() -> Self {
        Self::new(UiBridgeErrorKind::NotRunning, "ContinueHere is not running")
    }

    pub(crate) fn operation(error: impl fmt::Display) -> Self {
        Self::new(UiBridgeErrorKind::OperationFailed, error)
    }

    pub(crate) fn synchronization() -> Self {
        Self::new(
            UiBridgeErrorKind::SynchronizationFailed,
            "UI bridge synchronization failed",
        )
    }

    fn new(kind: UiBridgeErrorKind, message: impl fmt::Display) -> Self {
        Self {
            kind,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for UiBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for UiBridgeError {}

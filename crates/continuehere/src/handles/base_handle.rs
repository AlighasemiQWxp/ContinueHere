use std::fmt;

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HandleState {
    Idle,
    Using,
    Used,
    Releasing,
    Released,
}

impl fmt::Display for HandleState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Idle => "idle",
            Self::Using => "in use",
            Self::Used => "used",
            Self::Releasing => "releasing",
            Self::Released => "released",
        };

        formatter.write_str(name)
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub(crate) enum HandleError {
    #[error("handle identifier cannot be empty")]
    EmptyIdentifier,

    #[error("handle `{identifier}` must be configured before use")]
    NotConfigured { identifier: String },

    #[error("cannot {operation} handle `{identifier}` while it is {state}")]
    InvalidState {
        identifier: String,
        operation: &'static str,
        state: HandleState,
    },

    #[error("handle operation was rejected: {0}")]
    Rejected(&'static str),

    #[error("handle synchronization failed")]
    SynchronizationFailed,
}

pub(crate) struct BaseHandle {
    identifier: String,
    state: HandleState,
    configured: bool,
    release_after_use: bool,
}

impl BaseHandle {
    pub(crate) fn new(identifier: String) -> Self {
        Self {
            identifier,
            state: HandleState::Idle,
            configured: false,
            release_after_use: false,
        }
    }

    pub(crate) fn identifier(&self) -> &str {
        &self.identifier
    }

    pub(crate) fn state(&self) -> HandleState {
        self.state
    }

    pub(crate) fn is_released(&self) -> bool {
        self.state == HandleState::Releasing || self.state == HandleState::Released
    }

    pub(super) fn is_configured(&self) -> bool {
        self.configured
    }

    pub(super) fn release_after_use_requested(&self) -> bool {
        self.release_after_use
    }

    pub(super) fn set_state(&mut self, state: HandleState) {
        self.state = state;
    }

    pub(super) fn mark_configured(&mut self) {
        self.configured = true;
    }

    pub(super) fn request_release_after_use(&mut self) {
        self.release_after_use = true;
    }

    pub(super) fn finish_release(&mut self) {
        self.configured = false;
        self.release_after_use = false;
        self.state = HandleState::Released;
    }
}

pub(crate) trait Handle: Send + 'static {
    fn base(&self) -> &BaseHandle;

    fn base_mut(&mut self) -> &mut BaseHandle;

    fn can_release(&mut self) -> bool {
        true
    }

    fn on_use_begin(&mut self) -> Result<(), HandleError> {
        Ok(())
    }

    fn on_use_end(&mut self) {}

    fn on_release_beginning(&mut self) {}

    fn on_usage_releasing(&mut self) {}

    fn on_release_finished(&mut self) {}

    fn on_released(&mut self) {}
}

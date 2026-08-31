use std::sync::Arc;

use crate::handles::{BaseHandle, Handle, HandleError, HandleReference, UsageHandle};

use super::{PairingController, PairingError, PairingMode, PairingSession};

const MANAGER_UNAVAILABLE: &str = "pairing manager is not running";
const SESSION_LIMIT: &str = "pairing session limit reached";
const COMMAND_UNAVAILABLE: &str = "pairing command channel is unavailable";
const OPERATION_REJECTED: &str = "pairing operation was rejected";

pub struct PairingHandle {
    reference: HandleReference<PairingOperation>,
    controller: Arc<PairingController>,
}

impl PairingHandle {
    pub(crate) fn new(
        reference: HandleReference<PairingOperation>,
        controller: Arc<PairingController>,
    ) -> Self {
        Self {
            reference,
            controller,
        }
    }

    pub fn identifier(&self) -> &str {
        self.reference.identifier()
    }

    pub fn configure(&self, mode: PairingMode) -> Result<(), PairingError> {
        self.reference.configure(mode).map_err(map_handle_error)
    }

    pub fn use_handle(&self) -> Result<(), PairingError> {
        self.reference.use_handle().map_err(map_handle_error)
    }

    pub fn session(&self) -> Option<PairingSession> {
        self.controller.session_for_handle(self.identifier())
    }

    pub fn approve(&self) -> Result<(), PairingError> {
        self.controller.approve(self.identifier())
    }

    pub fn reject(&self) -> Result<(), PairingError> {
        self.controller.reject(self.identifier())
    }

    pub fn release(&self) -> Result<bool, PairingError> {
        self.reference.release().map_err(map_handle_error)
    }
}

impl Drop for PairingHandle {
    fn drop(&mut self) {
        let _release_result = self.reference.release();
    }
}

pub(crate) struct PairingOperation {
    base: BaseHandle,
    mode: Option<PairingMode>,
    controller: Arc<PairingController>,
}

impl PairingOperation {
    pub(crate) fn new(identifier: String, controller: Arc<PairingController>) -> Self {
        Self {
            base: BaseHandle::new(identifier),
            mode: None,
            controller,
        }
    }
}

impl Handle for PairingOperation {
    fn base(&self) -> &BaseHandle {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseHandle {
        &mut self.base
    }

    fn on_use_begin(&mut self) -> Result<(), HandleError> {
        let mode = self
            .mode
            .clone()
            .ok_or(HandleError::Rejected(OPERATION_REJECTED))?;
        self.controller
            .begin(self.base.identifier().to_owned(), mode)
            .map_err(map_pairing_error)
    }

    fn on_use_end(&mut self) {
        self.controller.cancel(self.base.identifier());
    }

    fn on_released(&mut self) {
        self.controller.remove(self.base.identifier());
        self.mode = None;
    }
}

impl UsageHandle for PairingOperation {
    type Config = PairingMode;

    fn on_configure(&mut self, mode: Self::Config) -> Result<(), HandleError> {
        self.mode = Some(mode);
        Ok(())
    }
}

fn map_pairing_error(error: PairingError) -> HandleError {
    match error {
        PairingError::ManagerUnavailable => HandleError::Rejected(MANAGER_UNAVAILABLE),
        PairingError::SessionLimit => HandleError::Rejected(SESSION_LIMIT),
        PairingError::CommandChannelUnavailable => HandleError::Rejected(COMMAND_UNAVAILABLE),
        _ => HandleError::Rejected(OPERATION_REJECTED),
    }
}

pub(crate) fn map_handle_error(error: HandleError) -> PairingError {
    match error {
        HandleError::EmptyIdentifier => PairingError::EmptyHandleIdentifier,
        HandleError::NotConfigured { .. } => PairingError::HandleNotConfigured,
        HandleError::InvalidState { .. } => PairingError::InvalidHandleState,
        HandleError::Rejected(MANAGER_UNAVAILABLE) => PairingError::ManagerUnavailable,
        HandleError::Rejected(SESSION_LIMIT) => PairingError::SessionLimit,
        HandleError::Rejected(COMMAND_UNAVAILABLE) => PairingError::CommandChannelUnavailable,
        HandleError::Rejected(_) => PairingError::InvalidHandleState,
        HandleError::SynchronizationFailed => PairingError::HandleSynchronizationFailed,
    }
}

use std::sync::Arc;

use crate::handles::{BaseHandle, Handle, HandleError, HandleReference, UsageHandle};

use super::{DiscoveryError, DiscoveryMode, DiscoveryRuntime};

const MANAGER_UNAVAILABLE: &str = "discovery manager is not running";
const COMMAND_UNAVAILABLE: &str = "discovery command channel is unavailable";
const CANDIDATE_LIMIT_REACHED: &str = "discovery candidate limit was reached";
const BACKEND_UNAVAILABLE: &str = "discovery backend is unavailable";
const OPERATION_REJECTED: &str = "discovery command was rejected";

pub struct DiscoveryHandle {
    reference: HandleReference<DiscoveryOperation>,
}

impl DiscoveryHandle {
    pub(crate) fn new(reference: HandleReference<DiscoveryOperation>) -> Self {
        Self { reference }
    }

    pub fn identifier(&self) -> &str {
        self.reference.identifier()
    }

    pub fn configure(&self, mode: DiscoveryMode) -> Result<(), DiscoveryError> {
        self.reference.configure(mode).map_err(map_handle_error)
    }

    pub fn use_handle(&self) -> Result<(), DiscoveryError> {
        self.reference.use_handle().map_err(map_handle_error)
    }

    pub fn release(&self) -> Result<bool, DiscoveryError> {
        self.reference.release().map_err(map_handle_error)
    }
}

impl Drop for DiscoveryHandle {
    fn drop(&mut self) {
        let _release_result = self.reference.release();
    }
}

pub(crate) struct DiscoveryOperation {
    base: BaseHandle,
    mode: Option<DiscoveryMode>,
    runtime: Arc<DiscoveryRuntime>,
}

impl DiscoveryOperation {
    pub(crate) fn new(identifier: String, runtime: Arc<DiscoveryRuntime>) -> Self {
        Self {
            base: BaseHandle::new(identifier),
            mode: None,
            runtime,
        }
    }
}

impl Handle for DiscoveryOperation {
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
            .ok_or(HandleError::Rejected("discovery handle is not configured"))?;
        self.runtime
            .activate(self.base.identifier().to_owned(), mode)
            .map_err(map_discovery_error)
    }

    fn on_use_end(&mut self) {
        let _deactivate_result = self.runtime.deactivate(self.base.identifier().to_owned());
    }

    fn on_released(&mut self) {
        self.mode = None;
    }
}

impl UsageHandle for DiscoveryOperation {
    type Config = DiscoveryMode;

    fn on_configure(&mut self, mode: Self::Config) -> Result<(), HandleError> {
        self.mode = Some(mode);
        Ok(())
    }
}

fn map_discovery_error(error: DiscoveryError) -> HandleError {
    match error {
        DiscoveryError::ManagerUnavailable => HandleError::Rejected(MANAGER_UNAVAILABLE),
        DiscoveryError::CommandChannelUnavailable => HandleError::Rejected(COMMAND_UNAVAILABLE),
        DiscoveryError::CandidateLimit => HandleError::Rejected(CANDIDATE_LIMIT_REACHED),
        DiscoveryError::BackendUnavailable => HandleError::Rejected(BACKEND_UNAVAILABLE),
        _ => HandleError::Rejected(OPERATION_REJECTED),
    }
}

fn map_handle_error(error: HandleError) -> DiscoveryError {
    match error {
        HandleError::EmptyIdentifier => DiscoveryError::EmptyHandleIdentifier,
        HandleError::NotConfigured { .. } => DiscoveryError::HandleNotConfigured,
        HandleError::InvalidState { .. } => DiscoveryError::InvalidHandleState,
        HandleError::Rejected(MANAGER_UNAVAILABLE) => DiscoveryError::ManagerUnavailable,
        HandleError::Rejected(COMMAND_UNAVAILABLE) => DiscoveryError::CommandChannelUnavailable,
        HandleError::Rejected(CANDIDATE_LIMIT_REACHED) => DiscoveryError::CandidateLimit,
        HandleError::Rejected(BACKEND_UNAVAILABLE) => DiscoveryError::BackendUnavailable,
        HandleError::Rejected(_) => DiscoveryError::InvalidHandleState,
        HandleError::SynchronizationFailed => DiscoveryError::HandleSynchronizationFailed,
    }
}

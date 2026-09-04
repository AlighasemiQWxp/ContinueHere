use std::{sync::Arc, time::Duration};

use crate::{
    handles::{BaseHandle, Handle, HandleError, HandleReference, UsageHandle},
    models::DeviceId,
};

use super::{Handoff, HandoffConfig, HandoffController, HandoffError, UrlHandoff, YouTubeHandoff};

const MANAGER_UNAVAILABLE: &str = "handoff manager is not running";
const OPERATION_LIMIT: &str = "handoff operation limit reached";
const COMMAND_UNAVAILABLE: &str = "handoff command channel is unavailable";
const OPERATION_REJECTED: &str = "handoff operation was rejected";

pub struct HandoffHandle {
    reference: HandleReference<HandoffOperation>,
    controller: Arc<HandoffController>,
}

impl HandoffHandle {
    pub(crate) fn new(
        reference: HandleReference<HandoffOperation>,
        controller: Arc<HandoffController>,
    ) -> Self {
        Self {
            reference,
            controller,
        }
    }

    pub fn identifier(&self) -> &str {
        self.reference.identifier()
    }

    pub fn configure(&self, device_id: DeviceId, url: &str) -> Result<(), HandoffError> {
        self.configure_url(device_id, url)
    }

    pub fn configure_url(&self, device_id: DeviceId, url: &str) -> Result<(), HandoffError> {
        let config = HandoffConfig::url(device_id, UrlHandoff::new(url)?);
        self.reference.configure(config).map_err(map_handle_error)
    }

    pub fn configure_youtube(
        &self,
        device_id: DeviceId,
        url: &str,
        playback_position: Duration,
    ) -> Result<(), HandoffError> {
        let config =
            HandoffConfig::youtube(device_id, YouTubeHandoff::new(url, playback_position)?);
        self.reference.configure(config).map_err(map_handle_error)
    }

    pub fn use_handle(&self) -> Result<(), HandoffError> {
        self.reference.use_handle().map_err(map_handle_error)
    }

    pub fn handoff(&self) -> Option<Handoff> {
        self.controller.handoff_for_handle(self.identifier())
    }

    pub fn release(&self) -> Result<bool, HandoffError> {
        self.reference.release().map_err(map_handle_error)
    }
}

impl Drop for HandoffHandle {
    fn drop(&mut self) {
        let _ = self.reference.release();
    }
}

pub(crate) struct HandoffOperation {
    base: BaseHandle,
    config: Option<HandoffConfig>,
    controller: Arc<HandoffController>,
}

impl HandoffOperation {
    pub(crate) fn new(identifier: String, controller: Arc<HandoffController>) -> Self {
        Self {
            base: BaseHandle::new(identifier),
            config: None,
            controller,
        }
    }
}

impl Handle for HandoffOperation {
    fn base(&self) -> &BaseHandle {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseHandle {
        &mut self.base
    }

    fn on_use_begin(&mut self) -> Result<(), HandleError> {
        let config = self
            .config
            .clone()
            .ok_or(HandleError::Rejected(OPERATION_REJECTED))?;
        self.controller
            .begin(self.base.identifier().to_owned(), config)
            .map_err(map_handoff_error)
    }

    fn on_use_end(&mut self) {
        self.controller.cancel(self.base.identifier());
    }

    fn on_released(&mut self) {
        self.controller.remove(self.base.identifier());
        self.config = None;
    }
}

impl UsageHandle for HandoffOperation {
    type Config = HandoffConfig;

    fn on_configure(&mut self, config: Self::Config) -> Result<(), HandleError> {
        self.config = Some(config);
        Ok(())
    }
}

fn map_handoff_error(error: HandoffError) -> HandleError {
    match error {
        HandoffError::ManagerUnavailable => HandleError::Rejected(MANAGER_UNAVAILABLE),
        HandoffError::OperationLimit => HandleError::Rejected(OPERATION_LIMIT),
        HandoffError::CommandUnavailable => HandleError::Rejected(COMMAND_UNAVAILABLE),
        _ => HandleError::Rejected(OPERATION_REJECTED),
    }
}

pub(crate) fn map_handle_error(error: HandleError) -> HandoffError {
    match error {
        HandleError::EmptyIdentifier => HandoffError::EmptyHandleIdentifier,
        HandleError::NotConfigured { .. } => HandoffError::HandleNotConfigured,
        HandleError::InvalidState { .. } => HandoffError::InvalidHandleState,
        HandleError::Rejected(MANAGER_UNAVAILABLE) => HandoffError::ManagerUnavailable,
        HandleError::Rejected(OPERATION_LIMIT) => HandoffError::OperationLimit,
        HandleError::Rejected(COMMAND_UNAVAILABLE) => HandoffError::CommandUnavailable,
        HandleError::Rejected(_) => HandoffError::InvalidHandleState,
        HandleError::SynchronizationFailed => HandoffError::HandleSynchronizationFailed,
    }
}

use std::{path::PathBuf, sync::Arc};

use crate::{
    handles::{BaseHandle, Handle, HandleError, HandleReference, UsageHandle},
    models::DeviceId,
};

use super::{FileTransfer, FileTransferConfig, FileTransferController, FileTransferError};

const MANAGER_UNAVAILABLE: &str = "file-transfer manager is not running";
const OPERATION_LIMIT: &str = "file-transfer operation limit reached";
const COMMAND_UNAVAILABLE: &str = "file-transfer command channel is unavailable";
const OPERATION_REJECTED: &str = "file-transfer operation was rejected";

pub struct FileTransferHandle {
    reference: HandleReference<FileTransferOperation>,
    controller: Arc<FileTransferController>,
}

impl FileTransferHandle {
    pub(crate) fn new(
        reference: HandleReference<FileTransferOperation>,
        controller: Arc<FileTransferController>,
    ) -> Self {
        Self {
            reference,
            controller,
        }
    }

    pub fn identifier(&self) -> &str {
        self.reference.identifier()
    }

    pub fn configure(
        &self,
        device_id: DeviceId,
        source: impl Into<PathBuf>,
    ) -> Result<(), FileTransferError> {
        let config = FileTransferConfig::new(device_id, source.into())?;
        self.reference.configure(config).map_err(map_handle_error)
    }

    pub fn use_handle(&self) -> Result<(), FileTransferError> {
        self.reference.use_handle().map_err(map_handle_error)
    }

    pub fn transfer(&self) -> Option<FileTransfer> {
        self.controller.transfer_for_handle(self.identifier())
    }

    pub fn release(&self) -> Result<bool, FileTransferError> {
        self.reference.release().map_err(map_handle_error)
    }
}

impl Drop for FileTransferHandle {
    fn drop(&mut self) {
        let _ = self.reference.release();
    }
}

pub(crate) struct FileTransferOperation {
    base: BaseHandle,
    config: Option<FileTransferConfig>,
    controller: Arc<FileTransferController>,
}

impl FileTransferOperation {
    pub(crate) fn new(identifier: String, controller: Arc<FileTransferController>) -> Self {
        Self {
            base: BaseHandle::new(identifier),
            config: None,
            controller,
        }
    }
}

impl Handle for FileTransferOperation {
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
            .map_err(map_transfer_error)
    }

    fn on_use_end(&mut self) {
        self.controller.cancel(self.base.identifier());
    }

    fn on_released(&mut self) {
        self.controller.remove_for_handle(self.base.identifier());
        self.config = None;
    }
}

impl UsageHandle for FileTransferOperation {
    type Config = FileTransferConfig;

    fn on_configure(&mut self, config: Self::Config) -> Result<(), HandleError> {
        self.config = Some(config);
        Ok(())
    }
}

fn map_transfer_error(error: FileTransferError) -> HandleError {
    match error {
        FileTransferError::ManagerUnavailable => HandleError::Rejected(MANAGER_UNAVAILABLE),
        FileTransferError::OperationLimit => HandleError::Rejected(OPERATION_LIMIT),
        FileTransferError::CommandUnavailable => HandleError::Rejected(COMMAND_UNAVAILABLE),
        _ => HandleError::Rejected(OPERATION_REJECTED),
    }
}

pub(crate) fn map_handle_error(error: HandleError) -> FileTransferError {
    match error {
        HandleError::EmptyIdentifier => FileTransferError::EmptyHandleIdentifier,
        HandleError::NotConfigured { .. } => FileTransferError::HandleNotConfigured,
        HandleError::InvalidState { .. } => FileTransferError::InvalidHandleState,
        HandleError::Rejected(MANAGER_UNAVAILABLE) => FileTransferError::ManagerUnavailable,
        HandleError::Rejected(OPERATION_LIMIT) => FileTransferError::OperationLimit,
        HandleError::Rejected(COMMAND_UNAVAILABLE) => FileTransferError::CommandUnavailable,
        HandleError::Rejected(_) => FileTransferError::InvalidHandleState,
        HandleError::SynchronizationFailed => FileTransferError::HandleSynchronizationFailed,
    }
}

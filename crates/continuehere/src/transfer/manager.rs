use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use async_trait::async_trait;

use crate::{
    core::{error::ModuleError, module::Module},
    handles::BaseHandleProvider,
    settings::DirectorySettings,
    transport::TransferTransportCapability,
};

use super::{
    FileTransfer, FileTransferChangedDelegate, FileTransferChangedEvent,
    FileTransferChangedSubscription, FileTransferController, FileTransferError, FileTransferHandle,
    FileTransferId, FileTransferOperation, handle::map_handle_error,
};

pub struct FileTransferManager {
    handles: Arc<Mutex<BaseHandleProvider<FileTransferOperation>>>,
    controller: Arc<FileTransferController>,
    changed: FileTransferChangedEvent,
}

impl FileTransferManager {
    pub(crate) fn new(
        transport: TransferTransportCapability,
        directories: DirectorySettings,
    ) -> Self {
        let changed = FileTransferChangedEvent::default();
        let controller = FileTransferController::new(transport, directories, changed.clone());
        Self {
            handles: Arc::new(Mutex::new(BaseHandleProvider::new())),
            controller,
            changed,
        }
    }

    pub fn get_handle(&self, identifier: &str) -> Result<FileTransferHandle, FileTransferError> {
        self.capability().get_handle(identifier)
    }

    pub(crate) fn capability(&self) -> super::FileTransferCapability {
        super::FileTransferCapability::new(
            Arc::clone(&self.handles),
            Arc::clone(&self.controller),
            self.changed.clone(),
        )
    }

    pub fn transfers(&self) -> Vec<FileTransfer> {
        self.controller.transfers()
    }

    pub fn accept_incoming(
        &self,
        transfer_id: &FileTransferId,
        selected_directory: Option<&Path>,
    ) -> Result<(), FileTransferError> {
        self.controller
            .accept_incoming(transfer_id, selected_directory)
    }

    pub fn reject_incoming(&self, transfer_id: &FileTransferId) -> Result<(), FileTransferError> {
        self.controller.reject_incoming(transfer_id)
    }

    pub fn remove(&self, transfer_id: &FileTransferId) -> Result<(), FileTransferError> {
        self.controller.remove(transfer_id)
    }

    pub fn on_transfer_changed(
        &self,
        delegate: FileTransferChangedDelegate,
    ) -> FileTransferChangedSubscription {
        self.changed.subscribe(delegate)
    }
}

#[async_trait]
impl Module for FileTransferManager {
    fn name(&self) -> &'static str {
        "file-transfer"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        self.controller.start()?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        let release_error = lock(&self.handles)
            .and_then(|mut handles| handles.release_all().map_err(map_handle_error))
            .err();
        let stop_error = self.controller.stop().err();
        match release_error.or(stop_error) {
            Some(error) => Err(Box::new(error)),
            None => Ok(()),
        }
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, FileTransferError> {
    value
        .lock()
        .map_err(|_| FileTransferError::HandleSynchronizationFailed)
}

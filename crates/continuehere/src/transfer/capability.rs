use std::sync::{Arc, Mutex};

use crate::{handles::BaseHandleProvider, models::DeviceId};

use super::{
    FileTransfer, FileTransferChangedDelegate, FileTransferChangedEvent,
    FileTransferChangedSubscription, FileTransferController, FileTransferDirection,
    FileTransferError, FileTransferHandle, FileTransferId, FileTransferOperation,
    FileTransferState, handle::map_handle_error,
};

#[derive(Clone)]
pub(crate) struct FileTransferCapability {
    handles: Arc<Mutex<BaseHandleProvider<FileTransferOperation>>>,
    controller: Arc<FileTransferController>,
    changed: FileTransferChangedEvent,
}

impl FileTransferCapability {
    pub(crate) fn new(
        handles: Arc<Mutex<BaseHandleProvider<FileTransferOperation>>>,
        controller: Arc<FileTransferController>,
        changed: FileTransferChangedEvent,
    ) -> Self {
        Self {
            handles,
            controller,
            changed,
        }
    }

    pub(crate) fn get_handle(
        &self,
        identifier: &str,
    ) -> Result<FileTransferHandle, FileTransferError> {
        if !self.controller.is_running() {
            return Err(FileTransferError::ManagerUnavailable);
        }
        let controller = Arc::clone(&self.controller);
        let reference = self
            .handles
            .lock()
            .map_err(|_| FileTransferError::HandleSynchronizationFailed)?
            .get_handle(identifier, {
                let controller = Arc::clone(&controller);
                move |identifier| FileTransferOperation::new(identifier, controller)
            })
            .map_err(map_handle_error)?;
        Ok(FileTransferHandle::new(reference, controller))
    }

    pub(crate) fn on_changed(
        &self,
        delegate: FileTransferChangedDelegate,
    ) -> FileTransferChangedSubscription {
        self.changed.subscribe(delegate)
    }

    pub(crate) fn completed_incoming(
        &self,
        id: &FileTransferId,
        peer: &DeviceId,
    ) -> Option<FileTransfer> {
        self.controller.transfers().into_iter().find(|transfer| {
            transfer.id() == id
                && transfer.peer_device_id() == peer
                && transfer.direction() == FileTransferDirection::Incoming
                && transfer.state() == FileTransferState::Completed
        })
    }
}

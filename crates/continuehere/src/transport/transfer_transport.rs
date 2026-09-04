use std::sync::{Arc, Mutex, MutexGuard, Weak, mpsc as standard_mpsc};

use tokio::sync::mpsc;

use crate::models::DeviceId;

use super::{SupervisorCommand, TransportError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TransferTransportMessage {
    Offer { file_name: String, file_size: u64 },
    Chunk { offset: u64, bytes: Vec<u8> },
    Finish { digest: [u8; 32] },
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransferRejection {
    Invalid,
    Busy,
    Unavailable,
    Declined,
    DestinationConflict,
    Integrity,
    FileSystem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransferDisposition {
    Accepted,
    Rejected(TransferRejection),
}

pub(crate) struct InboundTransfer {
    id: [u8; 16],
    sender_device_id: DeviceId,
    message: TransferTransportMessage,
}

impl InboundTransfer {
    pub(crate) fn new(
        id: [u8; 16],
        sender_device_id: DeviceId,
        message: TransferTransportMessage,
    ) -> Self {
        Self {
            id,
            sender_device_id,
            message,
        }
    }

    pub(crate) fn into_parts(self) -> ([u8; 16], DeviceId, TransferTransportMessage) {
        (self.id, self.sender_device_id, self.message)
    }
}

pub(crate) trait InboundTransferHandler: Send + Sync {
    fn receive(&self, transfer: InboundTransfer) -> TransferDisposition;
}

#[derive(Clone, Default)]
pub(crate) struct TransferTransportCapability {
    inner: Arc<TransferTransportInner>,
}

#[derive(Default)]
struct TransferTransportInner {
    commands: Mutex<Option<mpsc::Sender<SupervisorCommand>>>,
    handler: Mutex<Option<Weak<dyn InboundTransferHandler>>>,
}

impl TransferTransportCapability {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set_commands(&self, commands: Option<mpsc::Sender<SupervisorCommand>>) {
        *self
            .inner
            .commands
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = commands;
    }

    pub(crate) fn set_handler(&self, handler: Weak<dyn InboundTransferHandler>) {
        *self
            .inner
            .handler
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(handler);
    }

    pub(crate) fn clear_handler(&self) {
        *self
            .inner
            .handler
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
    }

    pub(crate) fn is_supported(&self) -> bool {
        lock(&self.inner.handler)
            .map(|handler| handler.as_ref().and_then(Weak::upgrade).is_some())
            .unwrap_or(false)
    }

    pub(crate) fn send(
        &self,
        device_id: DeviceId,
        transfer_id: [u8; 16],
        message: TransferTransportMessage,
    ) -> Result<standard_mpsc::Receiver<Result<TransferDisposition, TransportError>>, TransportError>
    {
        let commands = lock(&self.inner.commands)?
            .clone()
            .ok_or(TransportError::ManagerUnavailable)?;
        let (response, result) = standard_mpsc::channel();
        commands
            .blocking_send(SupervisorCommand::SendTransfer {
                device_id,
                transfer_id,
                message,
                response,
            })
            .map_err(|_| TransportError::CommandUnavailable)?;
        Ok(result)
    }

    pub(crate) async fn receive(&self, transfer: InboundTransfer) -> TransferDisposition {
        let handler = match lock(&self.inner.handler) {
            Ok(handler) => handler.as_ref().and_then(Weak::upgrade),
            Err(_) => return TransferDisposition::Rejected(TransferRejection::Unavailable),
        };
        let Some(handler) = handler else {
            return TransferDisposition::Rejected(TransferRejection::Unavailable);
        };
        tokio::task::spawn_blocking(move || handler.receive(transfer))
            .await
            .unwrap_or(TransferDisposition::Rejected(
                TransferRejection::Unavailable,
            ))
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, TransportError> {
    value
        .lock()
        .map_err(|_| TransportError::SynchronizationFailed)
}

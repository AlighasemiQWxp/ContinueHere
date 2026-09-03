use std::sync::{Arc, Mutex, MutexGuard, Weak, mpsc as standard_mpsc};

use tokio::sync::mpsc;

use crate::models::DeviceId;

use super::{SupervisorCommand, TransportError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UrlHandoffRejection {
    Invalid,
    Busy,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UrlHandoffDisposition {
    Accepted,
    Rejected(UrlHandoffRejection),
}

pub(crate) struct InboundUrlHandoff {
    id: [u8; 16],
    sender_device_id: DeviceId,
    url: String,
}

impl InboundUrlHandoff {
    pub(crate) fn new(id: [u8; 16], sender_device_id: DeviceId, url: String) -> Self {
        Self {
            id,
            sender_device_id,
            url,
        }
    }

    pub(crate) fn into_parts(self) -> ([u8; 16], DeviceId, String) {
        (self.id, self.sender_device_id, self.url)
    }
}

pub(crate) trait InboundUrlHandoffHandler: Send + Sync {
    fn receive(&self, handoff: InboundUrlHandoff) -> UrlHandoffDisposition;
}

#[derive(Clone, Default)]
pub(crate) struct HandoffTransportCapability {
    inner: Arc<HandoffTransportInner>,
}

#[derive(Default)]
struct HandoffTransportInner {
    commands: Mutex<Option<mpsc::Sender<SupervisorCommand>>>,
    handler: Mutex<Option<Weak<dyn InboundUrlHandoffHandler>>>,
}

impl HandoffTransportCapability {
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

    pub(crate) fn set_handler(&self, handler: Weak<dyn InboundUrlHandoffHandler>) {
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

    pub(crate) fn send_url(
        &self,
        device_id: DeviceId,
        handoff_id: [u8; 16],
        url: String,
    ) -> Result<
        standard_mpsc::Receiver<Result<UrlHandoffDisposition, TransportError>>,
        TransportError,
    > {
        let commands = lock(&self.inner.commands)?
            .clone()
            .ok_or(TransportError::ManagerUnavailable)?;
        let (response, result) = standard_mpsc::channel();
        commands
            .try_send(SupervisorCommand::SendUrlHandoff {
                device_id,
                handoff_id,
                url,
                response,
            })
            .map_err(|_| TransportError::CommandUnavailable)?;
        Ok(result)
    }

    pub(crate) async fn receive_url(&self, handoff: InboundUrlHandoff) -> UrlHandoffDisposition {
        let handler = match lock(&self.inner.handler) {
            Ok(handler) => handler.as_ref().and_then(Weak::upgrade),
            Err(_) => return UrlHandoffDisposition::Rejected(UrlHandoffRejection::Unavailable),
        };
        let Some(handler) = handler else {
            return UrlHandoffDisposition::Rejected(UrlHandoffRejection::Unavailable);
        };
        tokio::task::spawn_blocking(move || handler.receive(handoff))
            .await
            .unwrap_or(UrlHandoffDisposition::Rejected(
                UrlHandoffRejection::Unavailable,
            ))
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, TransportError> {
    value
        .lock()
        .map_err(|_| TransportError::SynchronizationFailed)
}

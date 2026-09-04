use std::sync::{Arc, Mutex, MutexGuard, Weak, mpsc as standard_mpsc};

use tokio::sync::mpsc;

use crate::models::{Capability, DeviceId};

use super::{SupervisorCommand, TransportError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HandoffTransportPayload {
    Url(String),
    YouTube {
        video_id: String,
        playback_position_millis: u64,
    },
}

impl HandoffTransportPayload {
    pub(crate) const fn capability(&self) -> Capability {
        match self {
            Self::Url(_) => Capability::UrlHandoff,
            Self::YouTube { .. } => Capability::PlaybackPositionHandoff,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandoffRejection {
    Invalid,
    Busy,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandoffDisposition {
    Accepted,
    Rejected(HandoffRejection),
}

pub(crate) struct InboundHandoff {
    id: [u8; 16],
    sender_device_id: DeviceId,
    payload: HandoffTransportPayload,
}

impl InboundHandoff {
    pub(crate) fn new(
        id: [u8; 16],
        sender_device_id: DeviceId,
        payload: HandoffTransportPayload,
    ) -> Self {
        Self {
            id,
            sender_device_id,
            payload,
        }
    }

    pub(crate) fn into_parts(self) -> ([u8; 16], DeviceId, HandoffTransportPayload) {
        (self.id, self.sender_device_id, self.payload)
    }
}

pub(crate) trait InboundHandoffHandler: Send + Sync {
    fn receive(&self, handoff: InboundHandoff) -> HandoffDisposition;
}

#[derive(Clone, Default)]
pub(crate) struct HandoffTransportCapability {
    inner: Arc<HandoffTransportInner>,
}

#[derive(Default)]
struct HandoffTransportInner {
    commands: Mutex<Option<mpsc::Sender<SupervisorCommand>>>,
    handler: Mutex<Option<Weak<dyn InboundHandoffHandler>>>,
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

    pub(crate) fn set_handler(&self, handler: Weak<dyn InboundHandoffHandler>) {
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
        handoff_id: [u8; 16],
        payload: HandoffTransportPayload,
    ) -> Result<standard_mpsc::Receiver<Result<HandoffDisposition, TransportError>>, TransportError>
    {
        let commands = lock(&self.inner.commands)?
            .clone()
            .ok_or(TransportError::ManagerUnavailable)?;
        let (response, result) = standard_mpsc::channel();
        commands
            .try_send(SupervisorCommand::SendHandoff {
                device_id,
                handoff_id,
                payload,
                response,
            })
            .map_err(|_| TransportError::CommandUnavailable)?;
        Ok(result)
    }

    pub(crate) async fn receive(&self, handoff: InboundHandoff) -> HandoffDisposition {
        let handler = match lock(&self.inner.handler) {
            Ok(handler) => handler.as_ref().and_then(Weak::upgrade),
            Err(_) => return HandoffDisposition::Rejected(HandoffRejection::Unavailable),
        };
        let Some(handler) = handler else {
            return HandoffDisposition::Rejected(HandoffRejection::Unavailable);
        };
        tokio::task::spawn_blocking(move || handler.receive(handoff))
            .await
            .unwrap_or(HandoffDisposition::Rejected(HandoffRejection::Unavailable))
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, TransportError> {
    value
        .lock()
        .map_err(|_| TransportError::SynchronizationFailed)
}

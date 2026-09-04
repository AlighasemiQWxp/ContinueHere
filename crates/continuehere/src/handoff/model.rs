use std::fmt;

use uuid::Uuid;

use crate::models::DeviceId;

use super::{UrlHandoff, YouTubeHandoff};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandoffId {
    value: String,
    bytes: [u8; 16],
}

impl HandoffId {
    pub(crate) fn new() -> Self {
        Self::from_uuid(Uuid::new_v4())
    }

    pub(crate) fn from_bytes(bytes: [u8; 16]) -> Self {
        Self::from_uuid(Uuid::from_bytes(bytes))
    }

    pub(crate) const fn bytes(&self) -> [u8; 16] {
        self.bytes
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    fn from_uuid(uuid: Uuid) -> Self {
        Self {
            value: uuid.hyphenated().to_string(),
            bytes: *uuid.as_bytes(),
        }
    }
}

impl fmt::Display for HandoffId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HandoffState {
    Sending,
    Delivered,
    Rejected,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HandoffFailure {
    NotConnected,
    Unsupported,
    Invalid,
    Busy,
    TimedOut,
    Transport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HandoffPayload {
    Url(UrlHandoff),
    YouTube(YouTubeHandoff),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    id: HandoffId,
    destination_device_id: DeviceId,
    payload: HandoffPayload,
    state: HandoffState,
    failure: Option<HandoffFailure>,
}

impl Handoff {
    pub(crate) fn new(id: HandoffId, config: HandoffConfig) -> Self {
        Self {
            id,
            destination_device_id: config.device_id,
            payload: config.payload,
            state: HandoffState::Sending,
            failure: None,
        }
    }

    pub fn id(&self) -> &HandoffId {
        &self.id
    }

    pub fn destination_device_id(&self) -> &DeviceId {
        &self.destination_device_id
    }

    pub fn payload(&self) -> &HandoffPayload {
        &self.payload
    }

    pub const fn state(&self) -> HandoffState {
        self.state
    }

    pub const fn failure(&self) -> Option<HandoffFailure> {
        self.failure
    }

    pub(crate) fn deliver(&mut self) {
        self.state = HandoffState::Delivered;
        self.failure = None;
    }

    pub(crate) fn reject(&mut self, failure: HandoffFailure) {
        self.state = HandoffState::Rejected;
        self.failure = Some(failure);
    }

    pub(crate) fn fail(&mut self, failure: HandoffFailure) {
        self.state = HandoffState::Failed;
        self.failure = Some(failure);
    }

    pub(crate) fn cancel(&mut self) {
        self.state = HandoffState::Cancelled;
        self.failure = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingHandoff {
    id: HandoffId,
    sender_device_id: DeviceId,
    payload: HandoffPayload,
}

impl IncomingHandoff {
    pub(crate) fn new(id: HandoffId, sender_device_id: DeviceId, payload: HandoffPayload) -> Self {
        Self {
            id,
            sender_device_id,
            payload,
        }
    }

    pub fn id(&self) -> &HandoffId {
        &self.id
    }

    pub fn sender_device_id(&self) -> &DeviceId {
        &self.sender_device_id
    }

    pub fn payload(&self) -> &HandoffPayload {
        &self.payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HandoffChange {
    Added(Handoff),
    Updated(Handoff),
    Removed(Handoff),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IncomingHandoffChange {
    Added(IncomingHandoff),
    Removed(IncomingHandoff),
}

#[derive(Clone)]
pub(crate) struct HandoffConfig {
    device_id: DeviceId,
    payload: HandoffPayload,
}

impl HandoffConfig {
    pub(crate) fn url(device_id: DeviceId, payload: UrlHandoff) -> Self {
        Self {
            device_id,
            payload: HandoffPayload::Url(payload),
        }
    }

    pub(crate) fn youtube(device_id: DeviceId, payload: YouTubeHandoff) -> Self {
        Self {
            device_id,
            payload: HandoffPayload::YouTube(payload),
        }
    }

    pub(crate) fn into_parts(self) -> (DeviceId, HandoffPayload) {
        (self.device_id, self.payload)
    }
}

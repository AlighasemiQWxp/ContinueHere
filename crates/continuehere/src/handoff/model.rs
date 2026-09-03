use std::fmt;

use url::Url;
use uuid::Uuid;

use crate::{models::DeviceId, transport::MAX_URL_SIZE};

use super::HandoffError;

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
pub struct UrlHandoff {
    url: String,
}

impl UrlHandoff {
    pub fn new(url: &str) -> Result<Self, HandoffError> {
        Ok(Self {
            url: validate_url(url)?,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HandoffPayload {
    Url(UrlHandoff),
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
    pub(crate) fn new(id: HandoffId, config: UrlHandoffConfig) -> Self {
        Self {
            id,
            destination_device_id: config.device_id,
            payload: HandoffPayload::Url(config.payload),
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
pub(crate) struct UrlHandoffConfig {
    device_id: DeviceId,
    payload: UrlHandoff,
}

impl UrlHandoffConfig {
    pub(crate) fn new(device_id: DeviceId, url: &str) -> Result<Self, HandoffError> {
        Ok(Self {
            device_id,
            payload: UrlHandoff::new(url)?,
        })
    }

    pub(crate) fn into_parts(self) -> (DeviceId, UrlHandoff) {
        (self.device_id, self.payload)
    }
}

pub(crate) fn validate_url(value: &str) -> Result<String, HandoffError> {
    if value.is_empty() {
        return Err(HandoffError::EmptyUrl);
    }
    if value.trim() != value {
        return Err(HandoffError::InvalidUrl);
    }
    if value.len() > MAX_URL_SIZE {
        return Err(HandoffError::UrlTooLarge);
    }
    let parsed = Url::parse(value).map_err(|_| HandoffError::InvalidUrl)?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(HandoffError::UnsupportedUrlScheme);
    }
    if parsed.host().is_none() {
        return Err(HandoffError::InvalidUrl);
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(HandoffError::UrlContainsCredentials);
    }
    let normalized = parsed.to_string();
    if normalized.len() > MAX_URL_SIZE {
        return Err(HandoffError::UrlTooLarge);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{HandoffError, validate_url};

    #[test]
    fn accepts_and_normalizes_http_urls() {
        assert_eq!(
            validate_url("https://example.com").expect("URL should be valid"),
            "https://example.com/"
        );
    }

    #[test]
    fn rejects_dangerous_schemes_and_credentials() {
        assert_eq!(
            validate_url("javascript:alert(1)"),
            Err(HandoffError::UnsupportedUrlScheme)
        );
        assert_eq!(
            validate_url("https://user:secret@example.com"),
            Err(HandoffError::UrlContainsCredentials)
        );
    }
}

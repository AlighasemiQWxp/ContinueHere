mod codec;

use crate::models::{Capability, DeviceId, LocalDeviceIdentity, Platform, ProtocolVersion};

use super::{HandoffRejection, HandoffTransportPayload, TransportError};

pub(crate) const MAX_CONTROL_FRAME_SIZE: usize = 64 * 1024;
pub(crate) const MAX_IN_FLIGHT_REQUESTS: usize = 64;
pub(crate) const IDLE_TIMEOUT_SECONDS: u16 = 120;

#[derive(Clone)]
pub(crate) struct ApplicationHello {
    device_id: DeviceId,
    display_name: String,
    platform: Platform,
    protocol_major: u16,
    minimum_minor: u16,
    maximum_minor: u16,
    capabilities: Vec<Capability>,
    nonce: [u8; 32],
    limits: ProtocolLimits,
}

#[derive(Clone, Copy)]
pub(crate) struct ProtocolLimits {
    maximum_frame_size: u32,
    maximum_in_flight_requests: u16,
    idle_timeout_seconds: u16,
}

pub(crate) struct NegotiatedProtocol {
    version: ProtocolVersion,
    limits: ProtocolLimits,
}

pub(crate) struct ProtocolEnvelope {
    request_id: u64,
    message: ProtocolMessage,
}

pub(crate) enum ProtocolMessage {
    Hello(ApplicationHello),
    Ping(u64),
    Pong(u64),
    Handoff {
        handoff_id: [u8; 16],
        payload: HandoffTransportPayload,
    },
    HandoffAccepted([u8; 16]),
    HandoffRejected {
        handoff_id: [u8; 16],
        reason: HandoffRejection,
    },
    Close,
}

impl ApplicationHello {
    pub(crate) fn local(
        identity: &LocalDeviceIdentity,
        mut capabilities: Vec<Capability>,
    ) -> Result<Self, TransportError> {
        let mut nonce = [0_u8; 32];
        getrandom::fill(&mut nonce).map_err(|_| TransportError::ConnectionFailed)?;
        capabilities.sort_unstable();
        capabilities.dedup();
        Ok(Self {
            device_id: identity.id().clone(),
            display_name: identity.display_name().to_owned(),
            platform: identity.platform(),
            protocol_major: ProtocolVersion::CURRENT.major(),
            minimum_minor: 0,
            maximum_minor: ProtocolVersion::CURRENT.minor(),
            capabilities,
            nonce,
            limits: ProtocolLimits::local(),
        })
    }

    pub(crate) fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.display_name
    }

    pub(crate) const fn platform(&self) -> Platform {
        self.platform
    }

    pub(crate) fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    pub(crate) const fn nonce(&self) -> &[u8; 32] {
        &self.nonce
    }

    pub(crate) fn negotiate(&self) -> Result<NegotiatedProtocol, TransportError> {
        if self.protocol_major != ProtocolVersion::CURRENT.major()
            || self.minimum_minor > self.maximum_minor
            || self.minimum_minor > ProtocolVersion::CURRENT.minor()
        {
            return Err(TransportError::IncompatibleProtocol);
        }
        let minor = self.maximum_minor.min(ProtocolVersion::CURRENT.minor());
        Ok(NegotiatedProtocol {
            version: ProtocolVersion::new(self.protocol_major, minor),
            limits: ProtocolLimits {
                maximum_frame_size: self
                    .limits
                    .maximum_frame_size
                    .min(MAX_CONTROL_FRAME_SIZE as u32),
                maximum_in_flight_requests: self
                    .limits
                    .maximum_in_flight_requests
                    .min(MAX_IN_FLIGHT_REQUESTS as u16),
                idle_timeout_seconds: self.limits.idle_timeout_seconds.min(IDLE_TIMEOUT_SECONDS),
            },
        })
    }
}

impl ProtocolLimits {
    const fn local() -> Self {
        Self {
            maximum_frame_size: MAX_CONTROL_FRAME_SIZE as u32,
            maximum_in_flight_requests: MAX_IN_FLIGHT_REQUESTS as u16,
            idle_timeout_seconds: IDLE_TIMEOUT_SECONDS,
        }
    }

    pub(crate) const fn maximum_frame_size(self) -> usize {
        self.maximum_frame_size as usize
    }

    pub(crate) const fn maximum_in_flight_requests(self) -> usize {
        self.maximum_in_flight_requests as usize
    }

    pub(crate) const fn idle_timeout_seconds(self) -> u16 {
        self.idle_timeout_seconds
    }
}

impl NegotiatedProtocol {
    pub(crate) const fn version(&self) -> ProtocolVersion {
        self.version
    }

    pub(crate) const fn limits(&self) -> ProtocolLimits {
        self.limits
    }
}

impl ProtocolEnvelope {
    pub(crate) fn hello(hello: ApplicationHello) -> Self {
        Self {
            request_id: 0,
            message: ProtocolMessage::Hello(hello),
        }
    }

    pub(crate) fn ping(request_id: u64, nonce: u64) -> Result<Self, TransportError> {
        Self::request(request_id, ProtocolMessage::Ping(nonce))
    }

    pub(crate) fn pong(request_id: u64, nonce: u64) -> Result<Self, TransportError> {
        Self::request(request_id, ProtocolMessage::Pong(nonce))
    }

    pub(crate) fn handoff(
        request_id: u64,
        handoff_id: [u8; 16],
        payload: HandoffTransportPayload,
    ) -> Result<Self, TransportError> {
        Self::request(
            request_id,
            ProtocolMessage::Handoff {
                handoff_id,
                payload,
            },
        )
    }

    pub(crate) fn handoff_accepted(
        request_id: u64,
        handoff_id: [u8; 16],
    ) -> Result<Self, TransportError> {
        Self::request(request_id, ProtocolMessage::HandoffAccepted(handoff_id))
    }

    pub(crate) fn handoff_rejected(
        request_id: u64,
        handoff_id: [u8; 16],
        reason: HandoffRejection,
    ) -> Result<Self, TransportError> {
        Self::request(
            request_id,
            ProtocolMessage::HandoffRejected { handoff_id, reason },
        )
    }

    pub(crate) const fn close() -> Self {
        Self {
            request_id: 0,
            message: ProtocolMessage::Close,
        }
    }

    pub(crate) const fn request_id(&self) -> u64 {
        self.request_id
    }

    pub(crate) fn message(&self) -> &ProtocolMessage {
        &self.message
    }

    fn request(request_id: u64, message: ProtocolMessage) -> Result<Self, TransportError> {
        if request_id == 0 {
            return Err(TransportError::ProtocolViolation);
        }
        Ok(Self {
            request_id,
            message,
        })
    }
}

impl ProtocolMessage {
    pub(crate) fn hello(&self) -> Option<&ApplicationHello> {
        match self {
            Self::Hello(hello) => Some(hello),
            _ => None,
        }
    }
}

pub(crate) use codec::{MAX_URL_SIZE, decode, encode};

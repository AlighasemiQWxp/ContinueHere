use crate::models::{Capability, DeviceId, Platform, ProtocolVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectionDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedConnection {
    device_id: DeviceId,
    display_name: String,
    platform: Platform,
    protocol_version: ProtocolVersion,
    capabilities: Vec<Capability>,
    direction: ConnectionDirection,
}

impl AuthenticatedConnection {
    pub(crate) fn new(
        device_id: DeviceId,
        display_name: String,
        platform: Platform,
        protocol_version: ProtocolVersion,
        capabilities: Vec<Capability>,
        direction: ConnectionDirection,
    ) -> Self {
        Self {
            device_id,
            display_name,
            platform,
            protocol_version,
            capabilities,
            direction,
        }
    }

    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn platform(&self) -> Platform {
        self.platform
    }

    pub const fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    pub const fn direction(&self) -> ConnectionDirection {
        self.direction
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectionChange {
    Added(AuthenticatedConnection),
    Removed(AuthenticatedConnection),
}

use super::{Capability, DeviceId, Platform, ProtocolVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DeviceState {
    Offline,
    Available,
    Connected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    id: DeviceId,
    display_name: String,
    platform: Platform,
    protocol_version: ProtocolVersion,
    capabilities: Vec<Capability>,
    state: DeviceState,
}

impl Device {
    pub fn new<I>(
        id: DeviceId,
        display_name: impl Into<String>,
        platform: Platform,
        protocol_version: ProtocolVersion,
        capabilities: I,
        state: DeviceState,
    ) -> Self
    where
        I: IntoIterator<Item = Capability>,
    {
        let mut unique_capabilities = Vec::new();
        for capability in capabilities {
            if !unique_capabilities.contains(&capability) {
                unique_capabilities.push(capability);
            }
        }

        Self {
            id,
            display_name: display_name.into(),
            platform,
            protocol_version,
            capabilities: unique_capabilities,
            state,
        }
    }

    pub fn id(&self) -> &DeviceId {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    pub fn supports(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn state(&self) -> DeviceState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::{Capability, Device, DeviceId, DeviceState, Platform, ProtocolVersion};

    #[test]
    fn device_exposes_its_shared_description() {
        let id = DeviceId::new("device-1").expect("identifier should be valid");
        let device = Device::new(
            id,
            "Desktop",
            Platform::Windows,
            ProtocolVersion::CURRENT,
            [
                Capability::UrlHandoff,
                Capability::FileTransfer,
                Capability::UrlHandoff,
            ],
            DeviceState::Available,
        );

        assert_eq!(device.id().as_str(), "device-1");
        assert_eq!(device.display_name(), "Desktop");
        assert_eq!(device.platform(), Platform::Windows);
        assert_eq!(device.protocol_version(), ProtocolVersion::CURRENT);
        assert_eq!(
            device.capabilities(),
            &[Capability::UrlHandoff, Capability::FileTransfer]
        );
        assert!(device.supports(Capability::FileTransfer));
        assert!(!device.supports(Capability::PlaybackPositionHandoff));
        assert_eq!(device.state(), DeviceState::Available);
    }
}

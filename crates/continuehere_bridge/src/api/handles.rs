use std::{path::PathBuf, time::Duration};

use continuehere::{
    DeviceId, DiscoveryEndpoint, DiscoveryHandle, DiscoveryMode, FileTransferHandle, HandoffHandle,
    PairingHandle, PairingMode,
};

use super::{
    UiBridgeError, UiFileTransfer, UiHandoff, UiPairingSession,
    models::{file_transfer, handoff, pairing_session},
};

#[flutter_rust_bridge::frb(opaque)]
pub struct UiDiscoveryHandle {
    inner: DiscoveryHandle,
}

impl UiDiscoveryHandle {
    pub(crate) fn new(inner: DiscoveryHandle) -> Self {
        Self { inner }
    }

    pub fn browse_local(&self) -> Result<(), UiBridgeError> {
        self.inner
            .configure(DiscoveryMode::LocalBrowse)
            .map_err(UiBridgeError::operation)
    }

    pub fn use_manual_endpoint(&self, host: String, port: u16) -> Result<(), UiBridgeError> {
        let endpoint = DiscoveryEndpoint::new(host, port).map_err(UiBridgeError::invalid_input)?;
        self.inner
            .configure(DiscoveryMode::ManualEndpoint(endpoint))
            .map_err(UiBridgeError::operation)
    }

    pub fn advertise(&self, host: String, port: u16) -> Result<(), UiBridgeError> {
        let endpoint = DiscoveryEndpoint::new(host, port).map_err(UiBridgeError::invalid_input)?;
        self.inner
            .configure(DiscoveryMode::AdvertiseEndpoint(endpoint))
            .map_err(UiBridgeError::operation)
    }

    pub fn use_handle(&self) -> Result<(), UiBridgeError> {
        self.inner.use_handle().map_err(UiBridgeError::operation)
    }

    pub fn release(&self) -> Result<bool, UiBridgeError> {
        self.inner.release().map_err(UiBridgeError::operation)
    }
}

#[flutter_rust_bridge::frb(opaque)]
pub struct UiPairingHandle {
    inner: PairingHandle,
}

impl UiPairingHandle {
    pub(crate) fn new(inner: PairingHandle) -> Self {
        Self { inner }
    }

    pub fn prepare_initiator(&self, host: String, port: u16) -> Result<(), UiBridgeError> {
        let endpoint = DiscoveryEndpoint::new(host, port).map_err(UiBridgeError::invalid_input)?;
        self.inner
            .configure(PairingMode::Initiate(endpoint))
            .map_err(UiBridgeError::operation)
    }

    pub fn prepare_receiver(&self) -> Result<(), UiBridgeError> {
        self.inner
            .configure(PairingMode::Receive)
            .map_err(UiBridgeError::operation)
    }

    pub fn use_handle(&self) -> Result<(), UiBridgeError> {
        self.inner.use_handle().map_err(UiBridgeError::operation)
    }

    pub fn session(&self) -> Option<UiPairingSession> {
        self.inner.session().map(pairing_session)
    }

    pub fn approve(&self) -> Result<(), UiBridgeError> {
        self.inner.approve().map_err(UiBridgeError::operation)
    }

    pub fn reject(&self) -> Result<(), UiBridgeError> {
        self.inner.reject().map_err(UiBridgeError::operation)
    }

    pub fn release(&self) -> Result<bool, UiBridgeError> {
        self.inner.release().map_err(UiBridgeError::operation)
    }
}

#[flutter_rust_bridge::frb(opaque)]
pub struct UiHandoffHandle {
    inner: HandoffHandle,
}

impl UiHandoffHandle {
    pub(crate) fn new(inner: HandoffHandle) -> Self {
        Self { inner }
    }

    pub fn prepare_url(&self, device_id: String, url: String) -> Result<(), UiBridgeError> {
        let device_id = parse_device_id(device_id)?;
        self.inner
            .configure_url(device_id, &url)
            .map_err(UiBridgeError::operation)
    }

    pub fn prepare_youtube(
        &self,
        device_id: String,
        url: String,
        playback_position_millis: u64,
    ) -> Result<(), UiBridgeError> {
        let device_id = parse_device_id(device_id)?;
        self.inner
            .configure_youtube(
                device_id,
                &url,
                Duration::from_millis(playback_position_millis),
            )
            .map_err(UiBridgeError::operation)
    }

    pub fn prepare_local_video(
        &self,
        device_id: String,
        source: String,
        playback_position_millis: u64,
    ) -> Result<(), UiBridgeError> {
        let device_id = parse_device_id(device_id)?;
        self.inner
            .configure_local_video(
                device_id,
                PathBuf::from(source),
                Duration::from_millis(playback_position_millis),
            )
            .map_err(UiBridgeError::operation)
    }

    pub fn use_handle(&self) -> Result<(), UiBridgeError> {
        self.inner.use_handle().map_err(UiBridgeError::operation)
    }

    pub fn handoff(&self) -> Option<UiHandoff> {
        self.inner.handoff().map(handoff)
    }

    pub fn release(&self) -> Result<bool, UiBridgeError> {
        self.inner.release().map_err(UiBridgeError::operation)
    }
}

#[flutter_rust_bridge::frb(opaque)]
pub struct UiFileTransferHandle {
    inner: FileTransferHandle,
}

impl UiFileTransferHandle {
    pub(crate) fn new(inner: FileTransferHandle) -> Self {
        Self { inner }
    }

    pub fn prepare(&self, device_id: String, source: String) -> Result<(), UiBridgeError> {
        let device_id = parse_device_id(device_id)?;
        self.inner
            .configure(device_id, PathBuf::from(source))
            .map_err(UiBridgeError::operation)
    }

    pub fn use_handle(&self) -> Result<(), UiBridgeError> {
        self.inner.use_handle().map_err(UiBridgeError::operation)
    }

    pub fn transfer(&self) -> Option<UiFileTransfer> {
        self.inner.transfer().map(file_transfer)
    }

    pub fn release(&self) -> Result<bool, UiBridgeError> {
        self.inner.release().map_err(UiBridgeError::operation)
    }
}

pub(crate) fn parse_device_id(value: String) -> Result<DeviceId, UiBridgeError> {
    DeviceId::new(value).map_err(UiBridgeError::invalid_input)
}

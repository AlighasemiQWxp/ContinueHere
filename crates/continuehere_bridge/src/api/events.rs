use continuehere::{
    ConnectionChangedDelegate, ConnectionChangedSubscription, ContinueHere,
    DeviceIdentityChangedDelegate, DeviceIdentityChangedSubscription, DirectoryChangedDelegate,
    DirectoryChangedSubscription, DiscoveryChangedDelegate, DiscoveryChangedSubscription,
    DiscoveryStatusChangedDelegate, DiscoveryStatusChangedSubscription,
    FileTransferChangedDelegate, FileTransferChangedSubscription, HandoffChangedDelegate,
    HandoffChangedSubscription, IncomingHandoffChangedDelegate, IncomingHandoffChangedSubscription,
    LanguageChangedDelegate, LanguageChangedSubscription, PairingSessionChangedDelegate,
    PairingSessionChangedSubscription, TrustedDeviceChangedDelegate,
    TrustedDeviceChangedSubscription,
};

use crate::frb_generated::StreamSink;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicesUiEvent {
    DiscoveryChanged,
    DiscoveryStatusChanged,
    TrustedDevicesChanged,
    ConnectionsChanged,
    IdentityChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingUiEvent {
    SessionsChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffUiEvent {
    OutgoingChanged,
    IncomingChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferUiEvent {
    TransfersChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsUiEvent {
    DirectoryChanged,
    LanguageChanged,
    IdentityChanged,
}

#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct DevicesUiSubscription {
    _discovery: DiscoveryChangedSubscription,
    _status: DiscoveryStatusChangedSubscription,
    _trusted: TrustedDeviceChangedSubscription,
    _connections: ConnectionChangedSubscription,
    _identity: DeviceIdentityChangedSubscription,
}

impl DevicesUiSubscription {
    pub(crate) fn new(app: &ContinueHere, sink: StreamSink<DevicesUiEvent>) -> Self {
        let discovery_sink = sink.clone();
        let status_sink = sink.clone();
        let trusted_sink = sink.clone();
        let connection_sink = sink.clone();
        Self {
            _discovery: app
                .discovery()
                .on_changed(DiscoveryChangedDelegate::new(move |_| {
                    let _ = discovery_sink.add(DevicesUiEvent::DiscoveryChanged);
                })),
            _status: app
                .discovery()
                .on_status_changed(DiscoveryStatusChangedDelegate::new(move |_| {
                    let _ = status_sink.add(DevicesUiEvent::DiscoveryStatusChanged);
                })),
            _trusted: app
                .pairing()
                .on_trusted_device_changed(TrustedDeviceChangedDelegate::new(move |_| {
                    let _ = trusted_sink.add(DevicesUiEvent::TrustedDevicesChanged);
                })),
            _connections: app
                .transport()
                .on_connection_changed(ConnectionChangedDelegate::new(move |_| {
                    let _ = connection_sink.add(DevicesUiEvent::ConnectionsChanged);
                })),
            _identity: app
                .devices()
                .on_identity_changed(DeviceIdentityChangedDelegate::new(move |_| {
                    let _ = sink.add(DevicesUiEvent::IdentityChanged);
                })),
        }
    }
}

#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct PairingUiSubscription {
    _sessions: PairingSessionChangedSubscription,
}

impl PairingUiSubscription {
    pub(crate) fn new(app: &ContinueHere, sink: StreamSink<PairingUiEvent>) -> Self {
        Self {
            _sessions: app
                .pairing()
                .on_session_changed(PairingSessionChangedDelegate::new(move |_| {
                    let _ = sink.add(PairingUiEvent::SessionsChanged);
                })),
        }
    }
}

#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct HandoffUiSubscription {
    _outgoing: HandoffChangedSubscription,
    _incoming: IncomingHandoffChangedSubscription,
}

impl HandoffUiSubscription {
    pub(crate) fn new(app: &ContinueHere, sink: StreamSink<HandoffUiEvent>) -> Self {
        let outgoing_sink = sink.clone();
        Self {
            _outgoing: app
                .handoff()
                .on_handoff_changed(HandoffChangedDelegate::new(move |_| {
                    let _ = outgoing_sink.add(HandoffUiEvent::OutgoingChanged);
                })),
            _incoming: app
                .handoff()
                .on_incoming_changed(IncomingHandoffChangedDelegate::new(move |_| {
                    let _ = sink.add(HandoffUiEvent::IncomingChanged);
                })),
        }
    }
}

#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct TransferUiSubscription {
    _transfers: FileTransferChangedSubscription,
}

impl TransferUiSubscription {
    pub(crate) fn new(app: &ContinueHere, sink: StreamSink<TransferUiEvent>) -> Self {
        Self {
            _transfers: app
                .file_transfers()
                .on_transfer_changed(FileTransferChangedDelegate::new(move |_| {
                    let _ = sink.add(TransferUiEvent::TransfersChanged);
                })),
        }
    }
}

#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct SettingsUiSubscription {
    _directory: DirectoryChangedSubscription,
    _language: LanguageChangedSubscription,
    _identity: DeviceIdentityChangedSubscription,
}

impl SettingsUiSubscription {
    pub(crate) fn new(app: &ContinueHere, sink: StreamSink<SettingsUiEvent>) -> Self {
        let directory_sink = sink.clone();
        let language_sink = sink.clone();
        Self {
            _directory: app
                .directories()
                .on_directory_changed(DirectoryChangedDelegate::new(move |_| {
                    let _ = directory_sink.add(SettingsUiEvent::DirectoryChanged);
                })),
            _language: app
                .localization()
                .on_language_changed(LanguageChangedDelegate::new(move |_| {
                    let _ = language_sink.add(SettingsUiEvent::LanguageChanged);
                })),
            _identity: app
                .devices()
                .on_identity_changed(DeviceIdentityChangedDelegate::new(move |_| {
                    let _ = sink.add(SettingsUiEvent::IdentityChanged);
                })),
        }
    }
}

#[derive(Default)]
#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct UiSubscriptions {
    pub(crate) devices: Option<DevicesUiSubscription>,
    pub(crate) pairing: Option<PairingUiSubscription>,
    pub(crate) handoff: Option<HandoffUiSubscription>,
    pub(crate) transfer: Option<TransferUiSubscription>,
    pub(crate) settings: Option<SettingsUiSubscription>,
}

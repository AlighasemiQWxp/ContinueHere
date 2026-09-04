mod activities;
mod app;
mod backends;
mod controllers;
mod core;
mod directories;
mod discovery;
#[allow(
    dead_code,
    unused_imports,
    reason = "feature modules use only the handle lifecycle capabilities they require"
)]
mod handles;
mod handoff;
mod locales;
mod managers;
mod models;
mod pairing;
mod security;
mod settings;
mod transfer;
mod transport;
mod utils;

pub use app::{ContinueHere, ContinueHereBuilder};
pub use core::error::Error;
pub use directories::{DirectoryChangedDelegate, DirectoryChangedSubscription, DirectoryManager};
pub use discovery::{
    DiscoveryCandidate, DiscoveryCandidateId, DiscoveryChange, DiscoveryChangedDelegate,
    DiscoveryChangedSubscription, DiscoveryEndpoint, DiscoveryError, DiscoveryHandle,
    DiscoveryManager, DiscoveryMode, DiscoverySource, DiscoveryStatus,
    DiscoveryStatusChangedDelegate, DiscoveryStatusChangedSubscription,
};
pub use handoff::{
    Handoff, HandoffChange, HandoffChangedDelegate, HandoffChangedSubscription, HandoffError,
    HandoffFailure, HandoffHandle, HandoffId, HandoffManager, HandoffPayload, HandoffState,
    IncomingHandoff, IncomingHandoffChange, IncomingHandoffChangedDelegate,
    IncomingHandoffChangedSubscription, PlaybackPosition, UrlHandoff, YouTubeHandoff,
};
pub use locales::{
    Language, LanguageChangedDelegate, LanguageChangedSubscription, LocalizationKey,
    LocalizationManager, TextDirection,
};
pub use managers::{
    DeviceIdentityChangedDelegate, DeviceIdentityChangedSubscription, DeviceManager,
};
pub use models::{
    Capability, Device, DeviceId, DeviceIdError, DeviceState, LocalDeviceIdentity, Platform,
    ProtocolVersion,
};
pub use pairing::{
    PairingError, PairingFailure, PairingHandle, PairingManager, PairingMode, PairingRole,
    PairingSession, PairingSessionChange, PairingSessionChangedDelegate,
    PairingSessionChangedSubscription, PairingSessionId, PairingState, PairingVerification,
    TrustedDevice, TrustedDeviceChange, TrustedDeviceChangedDelegate,
    TrustedDeviceChangedSubscription,
};
pub use settings::{DirectorySettings, LocalizationSettings, SettingsManager};
pub use transfer::{
    FileTransfer, FileTransferChange, FileTransferChangedDelegate, FileTransferChangedSubscription,
    FileTransferDirection, FileTransferError, FileTransferFailure, FileTransferHandle,
    FileTransferId, FileTransferManager, FileTransferState,
};
pub use transport::{
    AuthenticatedConnection, ConnectionChange, ConnectionChangedDelegate,
    ConnectionChangedSubscription, ConnectionDirection, TransportError, TransportManager,
};

pub type Result<T> = std::result::Result<T, Error>;

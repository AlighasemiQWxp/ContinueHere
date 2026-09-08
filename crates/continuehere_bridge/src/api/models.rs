use continuehere::{
    AuthenticatedConnection, ConnectionDirection, DiscoveryCandidate, DiscoverySource,
    DiscoveryStatus, FileTransfer, FileTransferDirection, FileTransferFailure, FileTransferState,
    Handoff, HandoffFailure, HandoffPayload, HandoffState, IncomingHandoff, Language,
    LocalDeviceIdentity, PairingFailure, PairingRole, PairingSession, PairingState, Platform,
    TextDirection, TrustedDevice,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPlatform {
    Windows,
    Linux,
    MacOs,
    Android,
    Ios,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLocalDevice {
    pub id: String,
    pub display_name: String,
    pub platform: UiPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiEndpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiDiscoverySource {
    Local,
    Manual,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiDiscoveryStatus {
    Idle,
    Active,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiDiscoveryCandidate {
    pub id: String,
    pub endpoints: Vec<UiEndpoint>,
    pub source: UiDiscoverySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiTrustedDevice {
    pub id: String,
    pub display_name: String,
    pub platform: UiPlatform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiConnectionDirection {
    Incoming,
    Outgoing,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiConnection {
    pub device_id: String,
    pub display_name: String,
    pub platform: UiPlatform,
    pub direction: UiConnectionDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiDevicesSnapshot {
    pub local_device: UiLocalDevice,
    pub discovery_status: UiDiscoveryStatus,
    pub candidates: Vec<UiDiscoveryCandidate>,
    pub trusted_devices: Vec<UiTrustedDevice>,
    pub connections: Vec<UiConnection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPairingRole {
    Initiator,
    Receiver,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPairingState {
    Connecting,
    ExchangingIdentity,
    AwaitingVerification,
    PersistingTrust,
    Trusted,
    Rejected,
    Cancelled,
    Expired,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPairingFailure {
    SecurityUnavailable,
    ConnectionFailed,
    ProtocolMismatch,
    InvalidPeer,
    TimedOut,
    PersistenceFailed,
    Internal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPairingSession {
    pub id: String,
    pub role: UiPairingRole,
    pub state: UiPairingState,
    pub peer_device_id: Option<String>,
    pub peer_display_name: Option<String>,
    pub manual_code: Option<String>,
    pub failure: Option<UiPairingFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPairingSnapshot {
    pub listening_endpoint: UiEndpoint,
    pub sessions: Vec<UiPairingSession>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiFileTransferDirection {
    Outgoing,
    Incoming,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiFileTransferState {
    Preparing,
    WaitingForAcceptance,
    Offered,
    Transferring,
    Verifying,
    Completed,
    Rejected,
    Cancelled,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiFileTransferFailure {
    NotConnected,
    Unsupported,
    Invalid,
    Busy,
    Declined,
    TimedOut,
    DestinationConflict,
    Integrity,
    FileSystem,
    Transport,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFileTransfer {
    pub id: String,
    pub peer_device_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub transferred_bytes: u64,
    pub direction: UiFileTransferDirection,
    pub state: UiFileTransferState,
    pub failure: Option<UiFileTransferFailure>,
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiHandoffState {
    Sending,
    Delivered,
    Rejected,
    Cancelled,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiHandoffFailure {
    NotConnected,
    Unsupported,
    Invalid,
    Busy,
    TimedOut,
    Transport,
    FileTransfer,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiHandoffKind {
    Url,
    YouTube,
    LocalVideo,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiHandoffPayload {
    pub kind: UiHandoffKind,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub playback_position_millis: u64,
    pub transfer_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiHandoff {
    pub id: String,
    pub destination_device_id: String,
    pub payload: UiHandoffPayload,
    pub state: UiHandoffState,
    pub failure: Option<UiHandoffFailure>,
    pub transfer: Option<UiFileTransfer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiIncomingHandoff {
    pub id: String,
    pub sender_device_id: String,
    pub payload: UiHandoffPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiHandoffSnapshot {
    pub outgoing: Vec<UiHandoff>,
    pub incoming: Vec<UiIncomingHandoff>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLanguage {
    English,
    Persian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTextDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiSettingsSnapshot {
    pub display_name: String,
    pub default_transfer_directory: String,
    pub language: UiLanguage,
    pub text_direction: UiTextDirection,
}

pub(crate) fn local_device(identity: LocalDeviceIdentity) -> UiLocalDevice {
    UiLocalDevice {
        id: identity.id().as_str().to_owned(),
        display_name: identity.display_name().to_owned(),
        platform: platform(identity.platform()),
    }
}

pub(crate) fn candidate(candidate: DiscoveryCandidate) -> UiDiscoveryCandidate {
    UiDiscoveryCandidate {
        id: candidate.id().as_str().to_owned(),
        endpoints: candidate
            .endpoints()
            .iter()
            .map(|endpoint| UiEndpoint {
                host: endpoint.host().to_owned(),
                port: endpoint.port(),
            })
            .collect(),
        source: match candidate.source() {
            DiscoverySource::Local => UiDiscoverySource::Local,
            DiscoverySource::Manual => UiDiscoverySource::Manual,
            _ => UiDiscoverySource::Unknown,
        },
    }
}

pub(crate) fn discovery_status(status: DiscoveryStatus) -> UiDiscoveryStatus {
    match status {
        DiscoveryStatus::Idle => UiDiscoveryStatus::Idle,
        DiscoveryStatus::Active => UiDiscoveryStatus::Active,
        DiscoveryStatus::Unavailable => UiDiscoveryStatus::Unavailable,
        _ => UiDiscoveryStatus::Unknown,
    }
}

pub(crate) fn trusted_device(device: TrustedDevice) -> UiTrustedDevice {
    UiTrustedDevice {
        id: device.device_id().as_str().to_owned(),
        display_name: device.display_name().to_owned(),
        platform: platform(device.platform()),
    }
}

pub(crate) fn connection(connection: AuthenticatedConnection) -> UiConnection {
    UiConnection {
        device_id: connection.device_id().as_str().to_owned(),
        display_name: connection.display_name().to_owned(),
        platform: platform(connection.platform()),
        direction: match connection.direction() {
            ConnectionDirection::Incoming => UiConnectionDirection::Incoming,
            ConnectionDirection::Outgoing => UiConnectionDirection::Outgoing,
            _ => UiConnectionDirection::Unknown,
        },
    }
}

pub(crate) fn pairing_session(session: PairingSession) -> UiPairingSession {
    UiPairingSession {
        id: session.id().as_str().to_owned(),
        role: match session.role() {
            PairingRole::Initiator => UiPairingRole::Initiator,
            PairingRole::Receiver => UiPairingRole::Receiver,
            _ => UiPairingRole::Unknown,
        },
        state: pairing_state(session.state()),
        peer_device_id: session
            .peer_device_id()
            .map(|identifier| identifier.as_str().to_owned()),
        peer_display_name: session.peer_display_name().map(str::to_owned),
        manual_code: session.verification().map(|value| value.manual_code()),
        failure: session.failure().map(pairing_failure),
    }
}

pub(crate) fn file_transfer(transfer: FileTransfer) -> UiFileTransfer {
    UiFileTransfer {
        id: transfer.id().as_str().to_owned(),
        peer_device_id: transfer.peer_device_id().as_str().to_owned(),
        file_name: transfer.file_name().to_owned(),
        file_size: transfer.file_size(),
        transferred_bytes: transfer.transferred_bytes(),
        direction: match transfer.direction() {
            FileTransferDirection::Outgoing => UiFileTransferDirection::Outgoing,
            FileTransferDirection::Incoming => UiFileTransferDirection::Incoming,
            _ => UiFileTransferDirection::Unknown,
        },
        state: file_transfer_state(transfer.state()),
        failure: transfer.failure().map(file_transfer_failure),
        destination: transfer
            .destination()
            .map(|path| path.to_string_lossy().into_owned()),
    }
}

pub(crate) fn handoff(handoff: Handoff) -> UiHandoff {
    UiHandoff {
        id: handoff.id().as_str().to_owned(),
        destination_device_id: handoff.destination_device_id().as_str().to_owned(),
        payload: handoff_payload(handoff.payload()),
        state: handoff_state(handoff.state()),
        failure: handoff.failure().map(handoff_failure),
        transfer: handoff.transfer().cloned().map(file_transfer),
    }
}

pub(crate) fn incoming_handoff(handoff: IncomingHandoff) -> UiIncomingHandoff {
    UiIncomingHandoff {
        id: handoff.id().as_str().to_owned(),
        sender_device_id: handoff.sender_device_id().as_str().to_owned(),
        payload: handoff_payload(handoff.payload()),
    }
}

pub(crate) fn language(language: Language) -> UiLanguage {
    match language {
        Language::English => UiLanguage::English,
        Language::Persian => UiLanguage::Persian,
        _ => UiLanguage::English,
    }
}

pub(crate) fn text_direction(direction: TextDirection) -> UiTextDirection {
    match direction {
        TextDirection::LeftToRight => UiTextDirection::LeftToRight,
        TextDirection::RightToLeft => UiTextDirection::RightToLeft,
        _ => UiTextDirection::LeftToRight,
    }
}

pub(crate) fn core_language(language: UiLanguage) -> Language {
    match language {
        UiLanguage::English => Language::English,
        UiLanguage::Persian => Language::Persian,
    }
}

pub(crate) fn platform(value: Platform) -> UiPlatform {
    match value {
        Platform::Windows => UiPlatform::Windows,
        Platform::Linux => UiPlatform::Linux,
        Platform::MacOs => UiPlatform::MacOs,
        Platform::Android => UiPlatform::Android,
        Platform::Ios => UiPlatform::Ios,
        Platform::Unknown => UiPlatform::Unknown,
        _ => UiPlatform::Unknown,
    }
}

fn pairing_state(state: PairingState) -> UiPairingState {
    match state {
        PairingState::Connecting => UiPairingState::Connecting,
        PairingState::ExchangingIdentity => UiPairingState::ExchangingIdentity,
        PairingState::AwaitingVerification => UiPairingState::AwaitingVerification,
        PairingState::PersistingTrust => UiPairingState::PersistingTrust,
        PairingState::Trusted => UiPairingState::Trusted,
        PairingState::Rejected => UiPairingState::Rejected,
        PairingState::Cancelled => UiPairingState::Cancelled,
        PairingState::Expired => UiPairingState::Expired,
        PairingState::Failed => UiPairingState::Failed,
        _ => UiPairingState::Unknown,
    }
}

fn pairing_failure(failure: PairingFailure) -> UiPairingFailure {
    match failure {
        PairingFailure::SecurityUnavailable => UiPairingFailure::SecurityUnavailable,
        PairingFailure::ConnectionFailed => UiPairingFailure::ConnectionFailed,
        PairingFailure::ProtocolMismatch => UiPairingFailure::ProtocolMismatch,
        PairingFailure::InvalidPeer => UiPairingFailure::InvalidPeer,
        PairingFailure::TimedOut => UiPairingFailure::TimedOut,
        PairingFailure::PersistenceFailed => UiPairingFailure::PersistenceFailed,
        PairingFailure::Internal => UiPairingFailure::Internal,
        _ => UiPairingFailure::Unknown,
    }
}

fn file_transfer_state(state: FileTransferState) -> UiFileTransferState {
    match state {
        FileTransferState::Preparing => UiFileTransferState::Preparing,
        FileTransferState::WaitingForAcceptance => UiFileTransferState::WaitingForAcceptance,
        FileTransferState::Offered => UiFileTransferState::Offered,
        FileTransferState::Transferring => UiFileTransferState::Transferring,
        FileTransferState::Verifying => UiFileTransferState::Verifying,
        FileTransferState::Completed => UiFileTransferState::Completed,
        FileTransferState::Rejected => UiFileTransferState::Rejected,
        FileTransferState::Cancelled => UiFileTransferState::Cancelled,
        FileTransferState::Failed => UiFileTransferState::Failed,
        _ => UiFileTransferState::Unknown,
    }
}

fn file_transfer_failure(failure: FileTransferFailure) -> UiFileTransferFailure {
    match failure {
        FileTransferFailure::NotConnected => UiFileTransferFailure::NotConnected,
        FileTransferFailure::Unsupported => UiFileTransferFailure::Unsupported,
        FileTransferFailure::Invalid => UiFileTransferFailure::Invalid,
        FileTransferFailure::Busy => UiFileTransferFailure::Busy,
        FileTransferFailure::Declined => UiFileTransferFailure::Declined,
        FileTransferFailure::TimedOut => UiFileTransferFailure::TimedOut,
        FileTransferFailure::DestinationConflict => UiFileTransferFailure::DestinationConflict,
        FileTransferFailure::Integrity => UiFileTransferFailure::Integrity,
        FileTransferFailure::FileSystem => UiFileTransferFailure::FileSystem,
        FileTransferFailure::Transport => UiFileTransferFailure::Transport,
        _ => UiFileTransferFailure::Unknown,
    }
}

fn handoff_state(state: HandoffState) -> UiHandoffState {
    match state {
        HandoffState::Sending => UiHandoffState::Sending,
        HandoffState::Delivered => UiHandoffState::Delivered,
        HandoffState::Rejected => UiHandoffState::Rejected,
        HandoffState::Cancelled => UiHandoffState::Cancelled,
        HandoffState::Failed => UiHandoffState::Failed,
        _ => UiHandoffState::Unknown,
    }
}

fn handoff_failure(failure: HandoffFailure) -> UiHandoffFailure {
    match failure {
        HandoffFailure::NotConnected => UiHandoffFailure::NotConnected,
        HandoffFailure::Unsupported => UiHandoffFailure::Unsupported,
        HandoffFailure::Invalid => UiHandoffFailure::Invalid,
        HandoffFailure::Busy => UiHandoffFailure::Busy,
        HandoffFailure::TimedOut => UiHandoffFailure::TimedOut,
        HandoffFailure::Transport => UiHandoffFailure::Transport,
        HandoffFailure::FileTransfer => UiHandoffFailure::FileTransfer,
        _ => UiHandoffFailure::Unknown,
    }
}

fn handoff_payload(payload: &HandoffPayload) -> UiHandoffPayload {
    match payload {
        HandoffPayload::Url(value) => UiHandoffPayload {
            kind: UiHandoffKind::Url,
            url: Some(value.url().to_owned()),
            file_path: None,
            playback_position_millis: 0,
            transfer_id: None,
        },
        HandoffPayload::YouTube(value) => UiHandoffPayload {
            kind: UiHandoffKind::YouTube,
            url: Some(value.resume_url()),
            file_path: None,
            playback_position_millis: value.playback_position().as_millis(),
            transfer_id: None,
        },
        HandoffPayload::LocalVideo(value) => UiHandoffPayload {
            kind: UiHandoffKind::LocalVideo,
            url: None,
            file_path: Some(value.file_path().to_string_lossy().into_owned()),
            playback_position_millis: value.playback_position().as_millis(),
            transfer_id: value
                .transfer_id()
                .map(|identifier| identifier.as_str().to_owned()),
        },
        _ => UiHandoffPayload {
            kind: UiHandoffKind::Unknown,
            url: None,
            file_path: None,
            playback_position_millis: 0,
            transfer_id: None,
        },
    }
}

use std::fmt;

use zeroize::Zeroize;

use crate::{
    discovery::DiscoveryEndpoint,
    models::{DeviceId, Platform},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PairingSessionId(String);

impl PairingSessionId {
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PairingSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PairingMode {
    Initiate(DiscoveryEndpoint),
    Receive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PairingRole {
    Initiator,
    Receiver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PairingState {
    Connecting,
    ExchangingIdentity,
    AwaitingVerification,
    PersistingTrust,
    Trusted,
    Rejected,
    Cancelled,
    Expired,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PairingFailure {
    SecurityUnavailable,
    ConnectionFailed,
    ProtocolMismatch,
    InvalidPeer,
    TimedOut,
    PersistenceFailed,
    Internal,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PairingVerification {
    value: [u8; 32],
}

impl PairingVerification {
    pub(crate) const fn new(value: [u8; 32]) -> Self {
        Self { value }
    }

    pub fn qr_value(&self) -> &[u8; 32] {
        &self.value
    }

    pub fn manual_code(&self) -> String {
        let numeric = u64::from_be_bytes([
            self.value[0],
            self.value[1],
            self.value[2],
            self.value[3],
            self.value[4],
            self.value[5],
            self.value[6],
            self.value[7],
        ]) % 10_000_000_000;
        let digits = format!("{numeric:010}");
        format!("{} {}", &digits[..5], &digits[5..])
    }
}

impl fmt::Debug for PairingVerification {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingVerification([redacted])")
    }
}

impl Drop for PairingVerification {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingSession {
    id: PairingSessionId,
    role: PairingRole,
    state: PairingState,
    peer_device_id: Option<DeviceId>,
    peer_display_name: Option<String>,
    verification: Option<PairingVerification>,
    failure: Option<PairingFailure>,
}

impl PairingSession {
    pub(crate) fn new(id: PairingSessionId, role: PairingRole, state: PairingState) -> Self {
        Self {
            id,
            role,
            state,
            peer_device_id: None,
            peer_display_name: None,
            verification: None,
            failure: None,
        }
    }

    pub fn id(&self) -> &PairingSessionId {
        &self.id
    }

    pub const fn role(&self) -> PairingRole {
        self.role
    }

    pub const fn state(&self) -> PairingState {
        self.state
    }

    pub fn peer_device_id(&self) -> Option<&DeviceId> {
        self.peer_device_id.as_ref()
    }

    pub fn peer_display_name(&self) -> Option<&str> {
        self.peer_display_name.as_deref()
    }

    pub fn verification(&self) -> Option<&PairingVerification> {
        self.verification.as_ref()
    }

    pub const fn failure(&self) -> Option<PairingFailure> {
        self.failure
    }

    pub(crate) fn set_state(&mut self, state: PairingState) {
        self.state = state;
        if state != PairingState::AwaitingVerification {
            self.verification = None;
        }
    }

    pub(crate) fn set_peer(&mut self, device_id: DeviceId, display_name: String) {
        self.peer_device_id = Some(device_id);
        self.peer_display_name = Some(display_name);
    }

    pub(crate) fn set_verification(&mut self, verification: PairingVerification) {
        self.state = PairingState::AwaitingVerification;
        self.verification = Some(verification);
    }

    pub(crate) fn fail(&mut self, failure: PairingFailure) {
        self.state = PairingState::Failed;
        self.verification = None;
        self.failure = Some(failure);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedDevice {
    device_id: DeviceId,
    public_key_fingerprint: [u8; 32],
    display_name: String,
    platform: Platform,
}

impl TrustedDevice {
    pub(crate) fn new(
        device_id: DeviceId,
        public_key_fingerprint: [u8; 32],
        display_name: String,
        platform: Platform,
    ) -> Self {
        Self {
            device_id,
            public_key_fingerprint,
            display_name,
            platform,
        }
    }

    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub const fn public_key_fingerprint(&self) -> &[u8; 32] {
        &self.public_key_fingerprint
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn platform(&self) -> Platform {
        self.platform
    }
}

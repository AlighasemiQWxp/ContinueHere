mod controller;
mod error;
mod event;
mod handle;
mod manager;
mod model;
mod store;
mod trust;

pub use error::PairingError;
pub use event::{
    PairingSessionChange, PairingSessionChangedDelegate, PairingSessionChangedSubscription,
    TrustedDeviceChange, TrustedDeviceChangedDelegate, TrustedDeviceChangedSubscription,
};
pub use handle::PairingHandle;
pub use manager::PairingManager;
pub use model::{
    PairingFailure, PairingMode, PairingRole, PairingSession, PairingSessionId, PairingState,
    PairingVerification, TrustedDevice,
};

pub(crate) use controller::PairingController;
pub(crate) use event::{PairingSessionChangedEvent, TrustedDeviceChangedEvent};
pub(crate) use handle::PairingOperation;
pub(crate) use store::TrustedDeviceStore;
pub(crate) use trust::{TrustMutation, TrustedDeviceRegistry, TrustedPeerLookup};

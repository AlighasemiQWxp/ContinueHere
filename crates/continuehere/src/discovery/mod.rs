mod backend;
mod candidate;
mod error;
mod event;
mod handle;
mod manager;
mod runtime;
mod store;

pub use candidate::{
    DiscoveryCandidate, DiscoveryCandidateId, DiscoveryEndpoint, DiscoveryMode, DiscoverySource,
};
pub use error::DiscoveryError;
pub use event::{
    DiscoveryChange, DiscoveryChangedDelegate, DiscoveryChangedSubscription, DiscoveryStatus,
    DiscoveryStatusChangedDelegate, DiscoveryStatusChangedSubscription,
};
pub use handle::DiscoveryHandle;
pub use manager::DiscoveryManager;

pub(crate) use backend::{DiscoveryBackend, DiscoveryBackendEvent, MdnsDiscoveryBackend};
pub(crate) use candidate::MAX_DISCOVERY_IDENTIFIER_SIZE;
pub(crate) use event::{DiscoveryChangedEvent, DiscoveryStatusChangedEvent};
pub(crate) use handle::DiscoveryOperation;
pub(crate) use runtime::DiscoveryRuntime;
pub(crate) use store::CandidateStore;

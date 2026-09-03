mod controller;
mod error;
mod event;
mod handle;
mod manager;
mod model;

pub use error::HandoffError;
pub use event::{
    HandoffChangedDelegate, HandoffChangedSubscription, IncomingHandoffChangedDelegate,
    IncomingHandoffChangedSubscription,
};
pub use handle::HandoffHandle;
pub use manager::HandoffManager;
pub use model::{
    Handoff, HandoffChange, HandoffFailure, HandoffId, HandoffPayload, HandoffState,
    IncomingHandoff, IncomingHandoffChange, UrlHandoff,
};

pub(crate) use controller::HandoffController;
pub(crate) use event::{HandoffChangedEvent, IncomingHandoffChangedEvent};
pub(crate) use handle::HandoffOperation;
pub(crate) use model::UrlHandoffConfig;

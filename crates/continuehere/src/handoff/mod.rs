mod controller;
mod error;
mod event;
mod handle;
mod local_video;
mod manager;
mod model;
mod position;
mod video_operation;

#[cfg(test)]
mod local_video_tests;
mod url;
mod youtube;

pub use error::HandoffError;
pub use event::{
    HandoffChangedDelegate, HandoffChangedSubscription, IncomingHandoffChangedDelegate,
    IncomingHandoffChangedSubscription,
};
pub use handle::HandoffHandle;
pub use local_video::LocalVideoHandoff;
pub(crate) use manager::HandoffCapability;
pub use manager::HandoffManager;
pub use model::{
    Handoff, HandoffChange, HandoffFailure, HandoffId, HandoffPayload, HandoffState,
    IncomingHandoff, IncomingHandoffChange,
};
pub use position::PlaybackPosition;
pub use url::UrlHandoff;
pub use youtube::YouTubeHandoff;

pub(crate) use controller::HandoffController;
pub(crate) use event::{HandoffChangedEvent, IncomingHandoffChangedEvent};
pub(crate) use handle::HandoffOperation;
pub(crate) use model::HandoffConfig;

mod authenticated;
mod channel;
mod error;
mod event;
mod handoff_transport;
mod manager;
mod message;
mod model;
mod pairing_transport;
mod protocol;
mod supervisor;
mod tls;
mod verifier;

pub use error::TransportError;
pub use event::{ConnectionChangedDelegate, ConnectionChangedSubscription};
pub use manager::TransportManager;
pub use model::{AuthenticatedConnection, ConnectionChange, ConnectionDirection};

pub(crate) use event::ConnectionChangedEvent;
pub(crate) use handoff_transport::{
    HandoffTransportCapability, InboundUrlHandoff, InboundUrlHandoffHandler, UrlHandoffDisposition,
    UrlHandoffRejection,
};

pub(crate) use channel::PairingChannel;
pub(crate) use message::{PairingHello, PairingMessage};
pub(crate) use pairing_transport::PairingTransportCapability;
pub(crate) use protocol::MAX_URL_SIZE;
pub(crate) use supervisor::SupervisorCommand;

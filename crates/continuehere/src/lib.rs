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
    reason = "the handle foundation is activated by later feature modules"
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
pub use settings::{DirectorySettings, LocalizationSettings, SettingsManager};

pub type Result<T> = std::result::Result<T, Error>;

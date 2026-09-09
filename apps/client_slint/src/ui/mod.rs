mod devices;
mod handoff;
mod history;
mod image_preview;
mod manager;
mod pairing;
mod preview;
mod settings;
mod startup;
mod support;
mod transfers;

#[allow(
    clippy::todo,
    reason = "Slint generates unreachable Rust-component embedding stubs with todo macros"
)]
mod generated {
    slint::include_modules!();
}

pub(crate) use generated::{ContentRow, DeviceRow, MainWindow};
pub(crate) use manager::UiManager;
pub(crate) use startup::StartupUiController;

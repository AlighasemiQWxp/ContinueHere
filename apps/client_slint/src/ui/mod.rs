mod devices;
mod manager;
mod settings;

#[allow(
    clippy::todo,
    reason = "Slint generates unreachable Rust-component embedding stubs with todo macros"
)]
mod generated {
    slint::include_modules!();
}

pub(crate) use generated::{DeviceRow, MainWindow};
pub(crate) use manager::UiManager;

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod application;
mod platform;
mod ui;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    application::run()
}

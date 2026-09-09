#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod platform;
mod ui;

use slint::ComponentHandle;
use std::error::Error;

use ui::{MainWindow, StartupUiController};

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let window = MainWindow::new()?;
    let _runtime_context = runtime.enter();
    let startup = StartupUiController::start(&runtime, &window)?;

    let window_result = window.run();
    drop(window);

    startup.shutdown(&runtime)?;
    window_result?;
    Ok(())
}

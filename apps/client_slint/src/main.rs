#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod platform;
mod ui;

use std::{error::Error, rc::Rc};

use continuehere::ContinueHere;
use slint::ComponentHandle;

use ui::{MainWindow, UiManager};

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let window = MainWindow::new()?;
    let manager = match platform::project_directory() {
        Ok(project_directory) => {
            match runtime.block_on(ContinueHere::builder(project_directory).build()) {
                Ok(core) => Some(UiManager::start(Rc::new(core), &window)),
                Err(error) => {
                    window.set_error_message(error.to_string().into());
                    None
                }
            }
        }
        Err(error) => {
            window.set_error_message(error.to_string().into());
            None
        }
    };

    window.run()?;
    drop(window);

    if let Some(manager) = manager {
        manager.shutdown(&runtime)?;
    }
    Ok(())
}

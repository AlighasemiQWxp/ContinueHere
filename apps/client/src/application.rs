use slint::ComponentHandle;
use std::error::Error;

use crate::ui::{MainWindow, StartupUiController};

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let window = MainWindow::new()?;
    let _runtime_context = runtime.enter();
    let startup = StartupUiController::start(&runtime, &window)?;

    let window_result = window.run();
    drop(window);

    startup.shutdown(&runtime)?;
    #[cfg(target_os = "android")]
    crate::platform::shutdown_android()?;
    window_result?;
    Ok(())
}

use std::{cell::RefCell, error::Error, rc::Rc};

use continuehere::ContinueHere;
use tokio::runtime::Runtime;

use super::{MainWindow, devices::DevicesUiController, settings::SettingsUiController};

pub(crate) struct UiManager {
    core: Rc<ContinueHere>,
    devices: Rc<RefCell<DevicesUiController>>,
    settings: Rc<SettingsUiController>,
}

impl UiManager {
    pub(crate) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Self {
        let devices = DevicesUiController::start(Rc::clone(&core), window);
        let settings = SettingsUiController::start(Rc::clone(&core), window);
        Self {
            core,
            devices,
            settings,
        }
    }

    pub(crate) fn shutdown(self, runtime: &Runtime) -> Result<(), Box<dyn Error>> {
        let Self {
            core,
            devices,
            settings,
        } = self;
        drop(devices);
        drop(settings);
        let core = Rc::try_unwrap(core).map_err(|_| "UI core ownership was not released")?;
        runtime.block_on(core.shutdown())?;
        Ok(())
    }
}

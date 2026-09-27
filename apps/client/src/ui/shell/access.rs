use std::rc::Rc;

use continuehere::{AppearanceSettings, ContinueHere, LocalizationManager};

pub(in crate::ui) struct ShellAccess {
    core: Rc<ContinueHere>,
}

impl ShellAccess {
    pub(in crate::ui) fn new(core: Rc<ContinueHere>) -> Self {
        Self { core }
    }

    pub(super) fn appearance(&self) -> &AppearanceSettings {
        self.core.settings().appearance()
    }

    pub(super) fn localization(&self) -> &LocalizationManager {
        self.core.localization()
    }
}

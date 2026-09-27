use std::rc::Rc;

use continuehere::{
    AppearanceSettings, ContinueHere, DeviceManager, DirectorySettings, LocalizationSettings,
};

pub(in crate::ui) struct SettingsAccess {
    core: Rc<ContinueHere>,
}

impl SettingsAccess {
    pub(in crate::ui) fn new(core: Rc<ContinueHere>) -> Self {
        Self { core }
    }

    pub(super) fn devices(&self) -> &DeviceManager {
        self.core.devices()
    }

    pub(super) fn directories(&self) -> &DirectorySettings {
        self.core.settings().directories()
    }

    pub(super) fn localization(&self) -> &LocalizationSettings {
        self.core.settings().localization()
    }

    pub(super) fn appearance(&self) -> &AppearanceSettings {
        self.core.settings().appearance()
    }
}

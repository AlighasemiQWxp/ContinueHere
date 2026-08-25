use crate::{locales::LocalizationManager, managers::DeviceManager, settings::SettingsManager};

pub(crate) struct CoreModules {
    settings: SettingsManager,
    localization: LocalizationManager,
    devices: DeviceManager,
}

impl CoreModules {
    pub(crate) fn new() -> Self {
        Self {
            settings: SettingsManager::new(),
            localization: LocalizationManager::new(),
            devices: DeviceManager::new(),
        }
    }

    pub(crate) fn settings(&self) -> &SettingsManager {
        &self.settings
    }

    pub(crate) fn localization(&self) -> &LocalizationManager {
        &self.localization
    }

    pub(crate) fn devices(&self) -> &DeviceManager {
        &self.devices
    }
}

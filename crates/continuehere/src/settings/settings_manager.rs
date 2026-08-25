use async_trait::async_trait;

use crate::{core::error::ModuleError, core::module::Module, locales::LocalizationManager};

use super::modules::SettingsModules;

pub struct SettingsManager {
    modules: SettingsModules,
}

impl SettingsManager {
    pub(crate) fn new() -> Self {
        Self {
            modules: SettingsModules::new(),
        }
    }

    pub fn localization(&self) -> &LocalizationManager {
        self.modules.localization()
    }
}

#[async_trait]
impl Module for SettingsManager {
    fn name(&self) -> &'static str {
        "settings"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        self.modules.start_all().await
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        self.modules.stop_all().await
    }
}

#[cfg(test)]
mod tests {
    use crate::core::module::Module;

    use super::SettingsManager;

    #[tokio::test]
    async fn manager_controls_its_child_module_lifecycle() {
        let mut manager = SettingsManager::new();

        manager.start().await.expect("settings should start");
        assert!(manager.modules.localization_started());

        manager.stop().await.expect("settings should stop");
        assert!(!manager.modules.localization_started());
    }
}

use crate::{core::error::ModuleError, core::module::Module, locales::LocalizationManager};

pub(super) struct SettingsModules {
    localization: LocalizationManager,
    localization_started: bool,
}

impl SettingsModules {
    pub(super) fn new() -> Self {
        Self {
            localization: LocalizationManager::new(),
            localization_started: false,
        }
    }

    pub(super) fn localization(&self) -> &LocalizationManager {
        &self.localization
    }

    pub(super) async fn start_all(&mut self) -> Result<(), ModuleError> {
        self.localization.start().await?;
        self.localization_started = true;
        Ok(())
    }

    pub(super) async fn stop_all(&mut self) -> Result<(), ModuleError> {
        if !self.localization_started {
            return Ok(());
        }

        let result = self.localization.stop().await;
        self.localization_started = false;
        result
    }

    #[cfg(test)]
    pub(super) fn localization_started(&self) -> bool {
        self.localization_started
    }
}

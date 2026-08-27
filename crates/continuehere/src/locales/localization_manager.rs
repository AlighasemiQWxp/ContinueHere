use async_trait::async_trait;

use crate::{
    core::{error::ModuleError, module::Module},
    settings::LocalizationSettings,
};

use super::{
    Language, LanguageChangedDelegate, LanguageChangedSubscription, LocalizationKey, TextDirection,
    catalog,
};

pub struct LocalizationManager {
    settings: LocalizationSettings,
}

impl LocalizationManager {
    pub(crate) fn new(settings: LocalizationSettings) -> Self {
        Self { settings }
    }

    pub fn language(&self) -> Language {
        self.settings.language()
    }

    pub fn text_direction(&self) -> TextDirection {
        self.language().text_direction()
    }

    pub fn is_rtl(&self) -> bool {
        self.language().is_rtl()
    }

    pub fn text(&self, key: LocalizationKey) -> &'static str {
        catalog::text(self.language(), key)
    }

    pub fn on_language_changed(
        &self,
        delegate: LanguageChangedDelegate,
    ) -> LanguageChangedSubscription {
        self.settings.on_language_changed(delegate)
    }
}

#[async_trait]
impl Module for LocalizationManager {
    fn name(&self) -> &'static str {
        "localization"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tempfile::tempdir;

    use super::LocalizationManager;
    use crate::{
        locales::{Language, LanguageChangedDelegate, LocalizationKey, TextDirection},
        settings::SettingsManager,
    };

    #[test]
    fn manager_reads_language_direction_and_text_from_settings() {
        let project = tempdir().expect("temporary project directory should be available");
        let settings = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be created");
        let manager = LocalizationManager::new(settings.localization().shared());

        assert_eq!(manager.language(), Language::English);
        assert_eq!(manager.text_direction(), TextDirection::LeftToRight);
        assert!(!manager.is_rtl());
        assert_eq!(manager.text(LocalizationKey::LanguagePersian), "Persian");

        settings
            .localization()
            .set_language(Language::Persian)
            .expect("language should save");

        assert_eq!(manager.language(), Language::Persian);
        assert_eq!(manager.text_direction(), TextDirection::RightToLeft);
        assert!(manager.is_rtl());
        assert_eq!(manager.text(LocalizationKey::LanguagePersian), "فارسی");
    }

    #[test]
    fn main_system_exposes_its_language_changed_event() {
        let project = tempdir().expect("temporary project directory should be available");
        let settings = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be created");
        let manager = LocalizationManager::new(settings.localization().shared());
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded_changes = Arc::clone(&changes);
        let _subscription =
            manager.on_language_changed(LanguageChangedDelegate::new(move |language| {
                recorded_changes
                    .lock()
                    .expect("recorded changes should be available")
                    .push(language);
            }));

        settings
            .localization()
            .set_language(Language::Persian)
            .expect("language should save");

        assert_eq!(
            *changes
                .lock()
                .expect("recorded changes should be available"),
            vec![Language::Persian]
        );
    }
}

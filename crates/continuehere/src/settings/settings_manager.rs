use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::core::{error::ModuleError, module::Module};

use super::{
    directory_settings::{DirectorySettings, validate_directory_path},
    error::SettingsError,
    localization_settings::LocalizationSettings,
    store::SettingsStore,
};

pub struct SettingsManager {
    store: Arc<Mutex<SettingsStore>>,
    directories: DirectorySettings,
    localization: LocalizationSettings,
}

impl SettingsManager {
    pub(crate) fn new(project_directory: PathBuf) -> Result<Self, SettingsError> {
        validate_directory_path(&project_directory).map_err(|_| {
            SettingsError::InvalidProjectDirectory {
                path: project_directory.clone(),
            }
        })?;
        if project_directory.exists() && !project_directory.is_dir() {
            return Err(SettingsError::InvalidProjectDirectory {
                path: project_directory,
            });
        }

        let store = Arc::new(Mutex::new(SettingsStore::new(
            project_directory.join("settings.bin"),
        )));
        let directories = DirectorySettings::new(Arc::clone(&store), project_directory);
        let localization = LocalizationSettings::new(Arc::clone(&store));
        Ok(Self {
            store,
            directories,
            localization,
        })
    }

    pub fn directories(&self) -> &DirectorySettings {
        &self.directories
    }

    pub fn localization(&self) -> &LocalizationSettings {
        &self.localization
    }
}

#[async_trait]
impl Module for SettingsManager {
    fn name(&self) -> &'static str {
        "settings"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        self.store
            .lock()
            .map_err(|_| SettingsError::StateUnavailable)?
            .load()?;
        self.directories.load()?;
        self.localization.load()?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use crate::{core::module::Module, locales::Language};

    use super::SettingsManager;

    #[tokio::test]
    async fn manager_loads_the_saved_default_transfer_directory() {
        let project = tempdir().expect("temporary project directory should be available");
        let saved = project.path().join("received");
        let mut manager = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be created");
        manager.start().await.expect("settings should start");
        manager
            .directories()
            .set_default_transfer_directory(saved.clone())
            .expect("directory should save");
        manager.stop().await.expect("settings should stop");

        let mut loaded = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be recreated");
        loaded.start().await.expect("saved settings should load");

        assert_eq!(loaded.directories().default_transfer_directory(), saved);
    }

    #[tokio::test]
    async fn manager_loads_the_saved_language() {
        let project = tempdir().expect("temporary project directory should be available");
        let mut manager = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be created");
        manager.start().await.expect("settings should start");
        manager
            .localization()
            .set_language(Language::Persian)
            .expect("language should save");
        manager.stop().await.expect("settings should stop");

        let mut loaded = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be recreated");
        loaded.start().await.expect("saved settings should load");

        assert_eq!(loaded.localization().language(), Language::Persian);
    }

    #[test]
    fn manager_rejects_a_relative_project_directory() {
        assert!(SettingsManager::new(PathBuf::from("relative")).is_err());
    }

    #[test]
    fn manager_rejects_a_project_path_that_is_a_file() {
        let project = tempdir().expect("temporary project directory should be available");
        let path = project.path().join("not_a_directory");
        fs::write(&path, b"file").expect("file fixture should be written");

        assert!(SettingsManager::new(path).is_err());
    }

    #[tokio::test]
    async fn manager_preserves_a_malformed_settings_file() {
        let project = tempdir().expect("temporary project directory should be available");
        let path = project.path().join("settings.bin");
        fs::write(&path, b"malformed").expect("malformed fixture should be written");
        let mut manager = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be created");

        assert!(manager.start().await.is_err());
        assert_eq!(
            fs::read(path).expect("malformed fixture should remain"),
            b"malformed"
        );
    }
}

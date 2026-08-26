use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::{
    Error,
    core::{error::ModuleError, module::Module},
    directories::{DirectoryChangedDelegate, DirectoryChangedSubscription},
    settings::{DirectorySettings, validate_directory_path},
};

pub struct DirectoryManager {
    settings: DirectorySettings,
}

impl DirectoryManager {
    pub(crate) fn new(settings: DirectorySettings) -> Self {
        Self { settings }
    }

    pub fn default_transfer_directory(&self) -> PathBuf {
        self.settings.default_transfer_directory()
    }

    pub fn on_directory_changed(
        &self,
        delegate: DirectoryChangedDelegate,
    ) -> DirectoryChangedSubscription {
        self.settings.on_directory_changed(delegate)
    }

    pub fn resolve_transfer_directory(
        &self,
        selected_directory: Option<&Path>,
    ) -> crate::Result<PathBuf> {
        let Some(selected_directory) = selected_directory else {
            return Ok(self.default_transfer_directory());
        };

        validate_directory_path(selected_directory).map_err(Error::settings)?;
        Ok(selected_directory.to_path_buf())
    }
}

#[async_trait]
impl Module for DirectoryManager {
    fn name(&self) -> &'static str {
        "directories"
    }

    async fn start(&mut self) -> std::result::Result<(), ModuleError> {
        Ok(())
    }

    async fn stop(&mut self) -> std::result::Result<(), ModuleError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tempfile::tempdir;

    use super::DirectoryManager;
    use crate::{directories::DirectoryChangedDelegate, settings::SettingsManager};

    #[test]
    fn selected_transfer_directory_is_a_temporary_override() {
        let project = tempdir().expect("temporary project directory should be available");
        let selected = project.path().join("selected");
        let settings = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be created");
        let manager = DirectoryManager::new(settings.directories().shared());
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded_changes = Arc::clone(&changes);
        let _subscription =
            manager.on_directory_changed(DirectoryChangedDelegate::new(move |directory| {
                recorded_changes
                    .lock()
                    .expect("recorded changes should be available")
                    .push(directory.to_path_buf());
            }));

        assert_eq!(
            manager
                .resolve_transfer_directory(Some(&selected))
                .expect("selected directory should resolve"),
            selected
        );
        assert_eq!(manager.default_transfer_directory(), project.path());
        assert!(
            changes
                .lock()
                .expect("recorded changes should be available")
                .is_empty()
        );
    }

    #[test]
    fn missing_transfer_override_uses_the_saved_default() {
        let project = tempdir().expect("temporary project directory should be available");
        let saved = project.path().join("saved");
        let settings = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be created");
        settings
            .directories()
            .set_default_transfer_directory(saved.clone())
            .expect("default directory should save");
        let manager = DirectoryManager::new(settings.directories().shared());

        assert_eq!(
            manager
                .resolve_transfer_directory(None)
                .expect("default directory should resolve"),
            saved
        );
    }

    #[test]
    fn main_system_exposes_its_directory_changed_event() {
        let project = tempdir().expect("temporary project directory should be available");
        let saved = project.path().join("saved");
        let settings = SettingsManager::new(project.path().to_path_buf())
            .expect("settings manager should be created");
        let manager = DirectoryManager::new(settings.directories().shared());
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded_changes = Arc::clone(&changes);
        let _subscription =
            manager.on_directory_changed(DirectoryChangedDelegate::new(move |directory| {
                recorded_changes
                    .lock()
                    .expect("recorded changes should be available")
                    .push(directory.to_path_buf());
            }));

        settings
            .directories()
            .set_default_transfer_directory(saved.clone())
            .expect("default directory should save");

        assert_eq!(
            *changes
                .lock()
                .expect("recorded changes should be available"),
            vec![saved]
        );
    }
}

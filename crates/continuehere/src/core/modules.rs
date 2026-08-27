use std::path::PathBuf;

use crate::{
    Error, Result, directories::DirectoryManager, locales::LocalizationManager,
    managers::DeviceManager, settings::SettingsManager,
};

use super::{module::Module, registry::ModuleRegistry};

pub(crate) struct CoreModules {
    settings: SettingsManager,
    directories: DirectoryManager,
    localization: LocalizationManager,
    devices: DeviceManager,
    optional: ModuleRegistry,
    settings_started: bool,
    directories_started: bool,
    localization_started: bool,
    devices_started: bool,
}

impl CoreModules {
    pub(crate) fn new(optional: ModuleRegistry, project_directory: PathBuf) -> Result<Self> {
        let settings = SettingsManager::new(project_directory).map_err(Error::settings)?;
        let directories = DirectoryManager::new(settings.directories().shared());
        let localization = LocalizationManager::new(settings.localization().shared());
        Ok(Self {
            settings,
            directories,
            localization,
            devices: DeviceManager::new(),
            optional,
            settings_started: false,
            directories_started: false,
            localization_started: false,
            devices_started: false,
        })
    }

    pub(crate) fn settings(&self) -> &SettingsManager {
        &self.settings
    }

    pub(crate) fn directories(&self) -> &DirectoryManager {
        &self.directories
    }

    pub(crate) fn localization(&self) -> &LocalizationManager {
        &self.localization
    }

    pub(crate) fn devices(&self) -> &DeviceManager {
        &self.devices
    }

    pub(crate) async fn start_all(&mut self) -> Result<()> {
        start_module(&mut self.settings).await?;
        self.settings_started = true;

        if let Err(error) = start_module(&mut self.directories).await {
            self.rollback_main_systems().await;
            return Err(error);
        }
        self.directories_started = true;

        if let Err(error) = start_module(&mut self.localization).await {
            self.rollback_main_systems().await;
            return Err(error);
        }
        self.localization_started = true;

        if let Err(error) = start_module(&mut self.devices).await {
            self.rollback_main_systems().await;
            return Err(error);
        }
        self.devices_started = true;

        if let Err(error) = self.optional.start_all().await {
            self.rollback_main_systems().await;
            return Err(error);
        }

        Ok(())
    }

    pub(crate) async fn stop_all(&mut self) -> Result<()> {
        let mut first_error = self.optional.stop_all().await.err();

        if self.devices_started {
            let result = stop_module(&mut self.devices).await;
            self.devices_started = false;
            keep_first_error(&mut first_error, result);
        }

        if self.localization_started {
            let result = stop_module(&mut self.localization).await;
            self.localization_started = false;
            keep_first_error(&mut first_error, result);
        }

        if self.directories_started {
            let result = stop_module(&mut self.directories).await;
            self.directories_started = false;
            keep_first_error(&mut first_error, result);
        }

        if self.settings_started {
            let result = stop_module(&mut self.settings).await;
            self.settings_started = false;
            keep_first_error(&mut first_error, result);
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    async fn rollback_main_systems(&mut self) {
        if self.devices_started {
            let _ = self.devices.stop().await;
            self.devices_started = false;
        }

        if self.localization_started {
            let _ = self.localization.stop().await;
            self.localization_started = false;
        }

        if self.directories_started {
            let _ = self.directories.stop().await;
            self.directories_started = false;
        }

        if self.settings_started {
            let _ = self.settings.stop().await;
            self.settings_started = false;
        }
    }
}

async fn start_module<M>(module: &mut M) -> Result<()>
where
    M: Module,
{
    let name = module.name();
    module
        .start()
        .await
        .map_err(|source| Error::ModuleStart { name, source })
}

async fn stop_module<M>(module: &mut M) -> Result<()>
where
    M: Module,
{
    let name = module.name();
    module
        .stop()
        .await
        .map_err(|source| Error::ModuleStop { name, source })
}

fn keep_first_error(first_error: &mut Option<Error>, result: Result<()>) {
    if first_error.is_none() {
        if let Err(error) = result {
            *first_error = Some(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use crate::core::error::ModuleError;
    use tempfile::tempdir;

    use super::{CoreModules, Module, ModuleRegistry};

    struct FailingOptionalModule;

    #[async_trait]
    impl Module for FailingOptionalModule {
        fn name(&self) -> &'static str {
            "failing-optional"
        }

        async fn start(&mut self) -> Result<(), ModuleError> {
            Err(std::io::Error::other("start failed").into())
        }

        async fn stop(&mut self) -> Result<(), ModuleError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn core_starts_and_stops_all_main_systems() {
        let project = tempdir().expect("temporary project directory should be available");
        let mut modules = CoreModules::new(ModuleRegistry::default(), project.path().to_path_buf())
            .expect("core should be created");

        modules.start_all().await.expect("core should start");
        assert!(modules.settings_started);
        assert!(modules.directories_started);
        assert!(modules.localization_started);
        assert!(modules.devices_started);

        modules.stop_all().await.expect("core should stop");
        assert!(!modules.settings_started);
        assert!(!modules.directories_started);
        assert!(!modules.localization_started);
        assert!(!modules.devices_started);
    }

    #[tokio::test]
    async fn optional_start_failure_rolls_back_main_systems() {
        let project = tempdir().expect("temporary project directory should be available");
        let mut optional = ModuleRegistry::default();
        optional
            .register(FailingOptionalModule)
            .expect("optional module should register");
        let mut modules = CoreModules::new(optional, project.path().to_path_buf())
            .expect("core should be created");

        let result = modules.start_all().await;

        assert!(result.is_err());
        assert!(!modules.settings_started);
        assert!(!modules.directories_started);
        assert!(!modules.localization_started);
        assert!(!modules.devices_started);
    }
}

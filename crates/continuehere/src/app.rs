use std::path::PathBuf;

use crate::{
    Result,
    core::{module::Module, modules::CoreModules, registry::ModuleRegistry},
    directories::DirectoryManager,
    discovery::DiscoveryManager,
    locales::LocalizationManager,
    managers::DeviceManager,
    pairing::PairingManager,
    settings::SettingsManager,
    transport::TransportManager,
};

pub struct ContinueHere {
    modules: CoreModules,
}

impl ContinueHere {
    pub fn builder(project_directory: impl Into<PathBuf>) -> ContinueHereBuilder {
        ContinueHereBuilder::new(project_directory)
    }

    pub fn settings(&self) -> &SettingsManager {
        self.modules.settings()
    }

    pub fn directories(&self) -> &DirectoryManager {
        self.modules.directories()
    }

    pub fn localization(&self) -> &LocalizationManager {
        self.modules.localization()
    }

    pub fn devices(&self) -> &DeviceManager {
        self.modules.devices()
    }

    pub fn discovery(&self) -> &DiscoveryManager {
        self.modules.discovery()
    }

    pub fn pairing(&self) -> &PairingManager {
        self.modules.pairing()
    }

    pub fn transport(&self) -> &TransportManager {
        self.modules.transport()
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.modules.stop_all().await
    }

    fn new(modules: CoreModules) -> Self {
        Self { modules }
    }
}

pub struct ContinueHereBuilder {
    project_directory: PathBuf,
    optional_modules: ModuleRegistry,
}

impl ContinueHereBuilder {
    pub fn new(project_directory: impl Into<PathBuf>) -> Self {
        Self {
            project_directory: project_directory.into(),
            optional_modules: ModuleRegistry::default(),
        }
    }

    pub async fn build(self) -> Result<ContinueHere> {
        let mut modules = CoreModules::new(self.optional_modules, self.project_directory)?;
        modules.start_all().await?;
        Ok(ContinueHere::new(modules))
    }

    #[allow(
        dead_code,
        reason = "optional modules will be registered when their features are implemented"
    )]
    pub(crate) fn register_optional<M>(&mut self, module: M) -> Result<()>
    where
        M: Module + 'static,
    {
        self.optional_modules.register(module)
    }
}

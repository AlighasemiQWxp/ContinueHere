use crate::{
    Result,
    core::{module::Module, modules::CoreModules, registry::ModuleRegistry},
    locales::LocalizationManager,
    managers::DeviceManager,
    settings::SettingsManager,
};

pub struct ContinueHere {
    core: CoreModules,
    optional_modules: ModuleRegistry,
}

impl ContinueHere {
    pub fn builder() -> ContinueHereBuilder {
        ContinueHereBuilder::new()
    }

    pub fn settings(&self) -> &SettingsManager {
        self.core.settings()
    }

    pub fn localization(&self) -> &LocalizationManager {
        self.core.localization()
    }

    pub fn devices(&self) -> &DeviceManager {
        self.core.devices()
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.optional_modules.stop_all().await
    }

    fn new(core: CoreModules, optional_modules: ModuleRegistry) -> Self {
        Self {
            core,
            optional_modules,
        }
    }
}

#[derive(Default)]
pub struct ContinueHereBuilder {
    optional_modules: ModuleRegistry,
}

impl ContinueHereBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn build(mut self) -> Result<ContinueHere> {
        let core = CoreModules::new();
        let start_result = self.optional_modules.start_all().await;

        match start_result {
            Ok(()) => Ok(ContinueHere::new(core, self.optional_modules)),
            Err(error) => Err(error),
        }
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

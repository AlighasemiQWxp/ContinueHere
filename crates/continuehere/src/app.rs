use crate::{
    Result,
    core::{module::Module, modules::CoreModules, registry::ModuleRegistry},
    locales::LocalizationManager,
    managers::DeviceManager,
    settings::SettingsManager,
};

pub struct ContinueHere {
    modules: CoreModules,
}

impl ContinueHere {
    pub fn builder() -> ContinueHereBuilder {
        ContinueHereBuilder::new()
    }

    pub fn settings(&self) -> &SettingsManager {
        self.modules.settings()
    }

    pub fn localization(&self) -> &LocalizationManager {
        self.modules.localization()
    }

    pub fn devices(&self) -> &DeviceManager {
        self.modules.devices()
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.modules.stop_all().await
    }

    fn new(modules: CoreModules) -> Self {
        Self { modules }
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

    pub async fn build(self) -> Result<ContinueHere> {
        let mut modules = CoreModules::new(self.optional_modules);
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

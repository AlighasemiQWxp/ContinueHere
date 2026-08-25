use async_trait::async_trait;

use crate::core::{error::ModuleError, module::Module};

pub struct LocalizationManager {
    _private: (),
}

impl LocalizationManager {
    pub(crate) fn new() -> Self {
        Self { _private: () }
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

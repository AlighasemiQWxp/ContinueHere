use async_trait::async_trait;

use crate::core::{error::ModuleError, module::Module};

pub struct DeviceManager {
    _private: (),
}

impl DeviceManager {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

#[async_trait]
impl Module for DeviceManager {
    fn name(&self) -> &'static str {
        "devices"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        Ok(())
    }
}

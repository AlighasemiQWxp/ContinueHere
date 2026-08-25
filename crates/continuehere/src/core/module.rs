use async_trait::async_trait;

use super::error::ModuleError;

#[async_trait]
pub(crate) trait Module: Send {
    fn name(&self) -> &'static str;

    async fn start(&mut self) -> Result<(), ModuleError>;

    async fn stop(&mut self) -> Result<(), ModuleError>;
}

use super::{Handle, HandleError};

pub(crate) trait UsageHandle: Handle {
    type Config;

    fn on_configure(&mut self, config: Self::Config) -> Result<(), HandleError>;
}

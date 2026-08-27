use std::error::Error as StdError;

use thiserror::Error;

pub(crate) type ModuleError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("a module named `{name}` is already registered")]
    DuplicateModule { name: &'static str },

    #[error("module `{name}` failed to start")]
    ModuleStart {
        name: &'static str,
        #[source]
        source: Box<dyn StdError + Send + Sync + 'static>,
    },

    #[error("module `{name}` failed to stop")]
    ModuleStop {
        name: &'static str,
        #[source]
        source: Box<dyn StdError + Send + Sync + 'static>,
    },

    #[error("settings operation failed: {source}")]
    Settings {
        #[source]
        source: Box<dyn StdError + Send + Sync + 'static>,
    },

    #[error("device identity operation failed: {source}")]
    DeviceIdentity {
        #[source]
        source: Box<dyn StdError + Send + Sync + 'static>,
    },
}

impl Error {
    pub(crate) fn settings<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Settings {
            source: Box::new(source),
        }
    }

    pub(crate) fn device_identity<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::DeviceIdentity {
            source: Box::new(source),
        }
    }
}

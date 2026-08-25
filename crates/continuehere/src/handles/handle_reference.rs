use std::sync::{Arc, Mutex, Weak};

use super::{Handle, HandleError, HandleState, UsageHandle, handle_provider::ProviderEntries};

pub(crate) struct HandleReference<H>
where
    H: Handle,
{
    identifier: String,
    inner: Arc<Mutex<H>>,
    provider: Weak<Mutex<ProviderEntries<H>>>,
}

impl<H> Clone for HandleReference<H>
where
    H: Handle,
{
    fn clone(&self) -> Self {
        Self {
            identifier: self.identifier.clone(),
            inner: Arc::clone(&self.inner),
            provider: Weak::clone(&self.provider),
        }
    }
}

impl<H> HandleReference<H>
where
    H: Handle,
{
    pub(super) fn new(
        identifier: String,
        inner: Arc<Mutex<H>>,
        provider: Weak<Mutex<ProviderEntries<H>>>,
    ) -> Self {
        Self {
            identifier,
            inner,
            provider,
        }
    }

    pub(crate) fn identifier(&self) -> &str {
        &self.identifier
    }

    pub(crate) fn state(&self) -> Result<HandleState, HandleError> {
        match self.inner.lock() {
            Ok(handle) => Ok(handle.base().state()),
            Err(_) => Err(HandleError::SynchronizationFailed),
        }
    }

    pub(crate) fn release(&self) -> Result<bool, HandleError> {
        self.release_internal(false)
    }

    pub(crate) fn invalidate(&self) -> Result<bool, HandleError> {
        self.release_internal(true)
    }

    pub(crate) fn same_handle(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    fn release_internal(&self, force: bool) -> Result<bool, HandleError> {
        {
            let mut handle = match self.inner.lock() {
                Ok(handle) => handle,
                Err(_) => return Err(HandleError::SynchronizationFailed),
            };

            let state = handle.base().state();
            if state == HandleState::Releasing || state == HandleState::Released {
                return Ok(false);
            }

            if !force && !handle.can_release() {
                return Ok(false);
            }

            if state == HandleState::Using {
                handle.on_use_end();
            }

            handle.base_mut().set_state(HandleState::Releasing);
            handle.on_release_beginning();
            handle.on_usage_releasing();
        }

        let unregister_result = self.unregister();

        {
            let mut handle = match self.inner.lock() {
                Ok(handle) => handle,
                Err(_) => return Err(HandleError::SynchronizationFailed),
            };

            handle.on_release_finished();
            handle.base_mut().finish_release();
            handle.on_released();
        }

        match unregister_result {
            Ok(()) => Ok(true),
            Err(error) => Err(error),
        }
    }

    fn unregister(&self) -> Result<(), HandleError> {
        let provider = match self.provider.upgrade() {
            Some(provider) => provider,
            None => return Ok(()),
        };

        let mut entries = match provider.lock() {
            Ok(entries) => entries,
            Err(_) => return Err(HandleError::SynchronizationFailed),
        };

        let should_remove = match entries.handles.get(&self.identifier) {
            Some(registered) => Arc::ptr_eq(registered, &self.inner),
            None => false,
        };

        if should_remove {
            entries.handles.remove(&self.identifier);
        }

        Ok(())
    }
}

impl<H> HandleReference<H>
where
    H: UsageHandle,
{
    pub(crate) fn configure(&self, config: H::Config) -> Result<(), HandleError> {
        let mut handle = match self.inner.lock() {
            Ok(handle) => handle,
            Err(_) => return Err(HandleError::SynchronizationFailed),
        };

        let state = handle.base().state();
        if state == HandleState::Using
            || state == HandleState::Releasing
            || state == HandleState::Released
        {
            return Err(HandleError::InvalidState {
                identifier: self.identifier.clone(),
                operation: "configure",
                state,
            });
        }

        let configure_result = handle.on_configure(config);
        match configure_result {
            Ok(()) => {
                handle.base_mut().mark_configured();
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn use_handle(&self) -> Result<(), HandleError> {
        let mut handle = match self.inner.lock() {
            Ok(handle) => handle,
            Err(_) => return Err(HandleError::SynchronizationFailed),
        };

        let previous_state = handle.base().state();
        if previous_state != HandleState::Idle && previous_state != HandleState::Used {
            return Err(HandleError::InvalidState {
                identifier: self.identifier.clone(),
                operation: "use",
                state: previous_state,
            });
        }

        if !handle.base().is_configured() {
            return Err(HandleError::NotConfigured {
                identifier: self.identifier.clone(),
            });
        }

        handle.base_mut().set_state(HandleState::Using);
        let use_result = handle.on_use_begin();

        match use_result {
            Ok(()) => Ok(()),
            Err(error) => {
                handle.base_mut().set_state(previous_state);
                Err(error)
            }
        }
    }

    pub(crate) fn complete_use(&self) -> Result<(), HandleError> {
        let release_after_use = {
            let mut handle = match self.inner.lock() {
                Ok(handle) => handle,
                Err(_) => return Err(HandleError::SynchronizationFailed),
            };

            let state = handle.base().state();
            if state != HandleState::Using {
                return Err(HandleError::InvalidState {
                    identifier: self.identifier.clone(),
                    operation: "complete use of",
                    state,
                });
            }

            handle.on_use_end();
            handle.base_mut().set_state(HandleState::Used);
            handle.base().release_after_use_requested()
        };

        if release_after_use {
            let release_result = self.release();
            match release_result {
                Ok(_) => Ok(()),
                Err(error) => Err(error),
            }
        } else {
            Ok(())
        }
    }

    pub(crate) fn release_after_use(&self) -> Result<(), HandleError> {
        let mut handle = match self.inner.lock() {
            Ok(handle) => handle,
            Err(_) => return Err(HandleError::SynchronizationFailed),
        };

        let state = handle.base().state();
        if state == HandleState::Releasing || state == HandleState::Released {
            return Err(HandleError::InvalidState {
                identifier: self.identifier.clone(),
                operation: "request release after use for",
                state,
            });
        }

        handle.base_mut().request_release_after_use();
        Ok(())
    }
}

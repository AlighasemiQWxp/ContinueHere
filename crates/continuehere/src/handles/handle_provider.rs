use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::{Handle, HandleError, HandleReference};

pub(super) struct ProviderEntries<H> {
    pub(super) handles: HashMap<String, Arc<Mutex<H>>>,
}

impl<H> Default for ProviderEntries<H> {
    fn default() -> Self {
        Self {
            handles: HashMap::new(),
        }
    }
}

pub(crate) struct BaseHandleProvider<H>
where
    H: Handle,
{
    entries: Arc<Mutex<ProviderEntries<H>>>,
}

impl<H> Default for BaseHandleProvider<H>
where
    H: Handle,
{
    fn default() -> Self {
        Self {
            entries: Arc::new(Mutex::new(ProviderEntries::default())),
        }
    }
}

impl<H> BaseHandleProvider<H>
where
    H: Handle,
{
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_handle<F>(
        &mut self,
        identifier: &str,
        create: F,
    ) -> Result<HandleReference<H>, HandleError>
    where
        F: FnOnce(String) -> H,
    {
        if identifier.trim().is_empty() {
            return Err(HandleError::EmptyIdentifier);
        }

        let existing = match self.entries.lock() {
            Ok(entries) => entries.handles.get(identifier).cloned(),
            Err(_) => return Err(HandleError::SynchronizationFailed),
        };

        if let Some(inner) = existing {
            let released = match inner.lock() {
                Ok(handle) => handle.base().is_released(),
                Err(_) => return Err(HandleError::SynchronizationFailed),
            };

            if !released {
                return Ok(HandleReference::new(
                    identifier.to_owned(),
                    inner,
                    Arc::downgrade(&self.entries),
                ));
            }

            let mut entries = match self.entries.lock() {
                Ok(entries) => entries,
                Err(_) => return Err(HandleError::SynchronizationFailed),
            };

            let should_remove = match entries.handles.get(identifier) {
                Some(registered) => Arc::ptr_eq(registered, &inner),
                None => false,
            };

            if should_remove {
                entries.handles.remove(identifier);
            }
        }

        let identifier = identifier.to_owned();
        let inner = Arc::new(Mutex::new(create(identifier.clone())));

        let mut entries = match self.entries.lock() {
            Ok(entries) => entries,
            Err(_) => return Err(HandleError::SynchronizationFailed),
        };

        entries
            .handles
            .insert(identifier.clone(), Arc::clone(&inner));

        Ok(HandleReference::new(
            identifier,
            inner,
            Arc::downgrade(&self.entries),
        ))
    }

    pub(crate) fn active_handle_count(&self) -> Result<usize, HandleError> {
        match self.entries.lock() {
            Ok(entries) => Ok(entries.handles.len()),
            Err(_) => Err(HandleError::SynchronizationFailed),
        }
    }

    pub(crate) fn release_all(&mut self) -> Result<(), HandleError> {
        let handles = {
            let mut entries = match self.entries.lock() {
                Ok(entries) => entries,
                Err(_) => return Err(HandleError::SynchronizationFailed),
            };

            entries.handles.drain().collect::<Vec<_>>()
        };

        let mut first_error = None;

        for (identifier, inner) in handles {
            let handle = HandleReference::new(identifier, inner, Arc::downgrade(&self.entries));

            let release_result = handle.invalidate();
            if first_error.is_none() {
                if let Err(error) = release_result {
                    first_error = Some(error);
                }
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl<H> Drop for BaseHandleProvider<H>
where
    H: Handle,
{
    fn drop(&mut self) {
        let _release_result = self.release_all();
    }
}

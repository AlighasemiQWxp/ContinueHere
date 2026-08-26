use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone)]
pub struct DirectoryChangedDelegate {
    callback: Arc<dyn Fn(&Path) + Send + Sync + 'static>,
}

impl DirectoryChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&Path) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, directory: &Path) {
        (self.callback)(directory);
    }
}

#[derive(Clone, Default)]
pub(crate) struct DirectoryChangedEvent {
    inner: Arc<DirectoryChangedEventInner>,
}

#[derive(Default)]
struct DirectoryChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, DirectoryChangedDelegate>>,
}

impl DirectoryChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: DirectoryChangedDelegate,
    ) -> DirectoryChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock_delegates(&self.inner.delegates).insert(identifier, delegate);
        DirectoryChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, directory: &Path) {
        let delegates: Vec<_> = lock_delegates(&self.inner.delegates)
            .values()
            .cloned()
            .collect();
        for delegate in delegates {
            delegate.invoke(directory);
        }
    }
}

#[must_use = "dropping the subscription unregisters the directory delegate"]
pub struct DirectoryChangedSubscription {
    event: Weak<DirectoryChangedEventInner>,
    identifier: Option<u64>,
}

impl DirectoryChangedSubscription {
    pub fn unsubscribe(mut self) {
        self.remove();
    }

    fn remove(&mut self) {
        let Some(identifier) = self.identifier.take() else {
            return;
        };
        let Some(event) = self.event.upgrade() else {
            return;
        };
        lock_delegates(&event.delegates).remove(&identifier);
    }
}

impl Drop for DirectoryChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock_delegates(
    delegates: &Mutex<BTreeMap<u64, DirectoryChangedDelegate>>,
) -> MutexGuard<'_, BTreeMap<u64, DirectoryChangedDelegate>> {
    delegates.lock().unwrap_or_else(|error| error.into_inner())
}

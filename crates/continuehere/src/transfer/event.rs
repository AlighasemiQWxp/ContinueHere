use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use super::FileTransferChange;

#[derive(Clone)]
pub struct FileTransferChangedDelegate {
    callback: Arc<dyn Fn(FileTransferChange) + Send + Sync + 'static>,
}

impl FileTransferChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(FileTransferChange) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, change: FileTransferChange) {
        (self.callback)(change);
    }
}

#[derive(Clone, Default)]
pub(crate) struct FileTransferChangedEvent {
    inner: Arc<FileTransferChangedEventInner>,
}

#[derive(Default)]
struct FileTransferChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, FileTransferChangedDelegate>>,
}

impl FileTransferChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: FileTransferChangedDelegate,
    ) -> FileTransferChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock(&self.inner.delegates).insert(identifier, delegate);
        FileTransferChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, change: FileTransferChange) {
        let delegates = lock(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(change.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the file-transfer delegate"]
pub struct FileTransferChangedSubscription {
    event: Weak<FileTransferChangedEventInner>,
    identifier: Option<u64>,
}

impl FileTransferChangedSubscription {
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
        lock(&event.delegates).remove(&identifier);
    }
}

impl Drop for FileTransferChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

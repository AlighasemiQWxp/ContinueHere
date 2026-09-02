use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use super::ConnectionChange;

#[derive(Clone)]
pub struct ConnectionChangedDelegate {
    callback: Arc<dyn Fn(ConnectionChange) + Send + Sync + 'static>,
}

impl ConnectionChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(ConnectionChange) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, change: ConnectionChange) {
        (self.callback)(change);
    }
}

#[derive(Clone, Default)]
pub(crate) struct ConnectionChangedEvent {
    inner: Arc<ConnectionChangedEventInner>,
}

#[derive(Default)]
struct ConnectionChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, ConnectionChangedDelegate>>,
}

impl ConnectionChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: ConnectionChangedDelegate,
    ) -> ConnectionChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock(&self.inner.delegates).insert(identifier, delegate);
        ConnectionChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, change: ConnectionChange) {
        let delegates = lock(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(change.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the connection delegate"]
pub struct ConnectionChangedSubscription {
    event: Weak<ConnectionChangedEventInner>,
    identifier: Option<u64>,
}

impl ConnectionChangedSubscription {
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

impl Drop for ConnectionChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

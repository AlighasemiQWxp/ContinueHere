use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use super::ActivityChange;

#[derive(Clone)]
pub struct ActivityChangedDelegate {
    callback: Arc<dyn Fn(ActivityChange) + Send + Sync + 'static>,
}

impl ActivityChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(ActivityChange) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, change: ActivityChange) {
        (self.callback)(change);
    }
}

#[derive(Clone, Default)]
pub(crate) struct ActivityChangedEvent {
    inner: Arc<ActivityChangedEventInner>,
}

#[derive(Default)]
struct ActivityChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, ActivityChangedDelegate>>,
}

impl ActivityChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: ActivityChangedDelegate,
    ) -> ActivityChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock(&self.inner.delegates).insert(identifier, delegate);
        ActivityChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, change: ActivityChange) {
        let delegates = lock(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(change.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the activity delegate"]
pub struct ActivityChangedSubscription {
    event: Weak<ActivityChangedEventInner>,
    identifier: Option<u64>,
}

impl ActivityChangedSubscription {
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

impl Drop for ActivityChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

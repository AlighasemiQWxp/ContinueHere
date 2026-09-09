use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use super::appearance::Appearance;

#[derive(Clone)]
pub struct AppearanceChangedDelegate {
    callback: Arc<dyn Fn(Appearance) + Send + Sync + 'static>,
}

impl AppearanceChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(Appearance) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, appearance: Appearance) {
        (self.callback)(appearance);
    }
}

#[derive(Clone, Default)]
pub(crate) struct AppearanceChangedEvent {
    inner: Arc<AppearanceChangedEventInner>,
}

#[derive(Default)]
struct AppearanceChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, AppearanceChangedDelegate>>,
}

impl AppearanceChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: AppearanceChangedDelegate,
    ) -> AppearanceChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock_delegates(&self.inner.delegates).insert(identifier, delegate);
        AppearanceChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, appearance: Appearance) {
        let delegates: Vec<_> = lock_delegates(&self.inner.delegates)
            .values()
            .cloned()
            .collect();
        for delegate in delegates {
            delegate.invoke(appearance);
        }
    }
}

#[must_use = "dropping the subscription unregisters the appearance delegate"]
pub struct AppearanceChangedSubscription {
    event: Weak<AppearanceChangedEventInner>,
    identifier: Option<u64>,
}

impl AppearanceChangedSubscription {
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

impl Drop for AppearanceChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock_delegates(
    delegates: &Mutex<BTreeMap<u64, AppearanceChangedDelegate>>,
) -> MutexGuard<'_, BTreeMap<u64, AppearanceChangedDelegate>> {
    delegates.lock().unwrap_or_else(|error| error.into_inner())
}

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use super::DiscoveryCandidate;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiscoveryChange {
    Added(DiscoveryCandidate),
    Updated(DiscoveryCandidate),
    Removed(DiscoveryCandidate),
}

#[derive(Clone)]
pub struct DiscoveryChangedDelegate {
    callback: Arc<dyn Fn(DiscoveryChange) + Send + Sync + 'static>,
}

impl DiscoveryChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(DiscoveryChange) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, change: DiscoveryChange) {
        (self.callback)(change);
    }
}

#[derive(Clone, Default)]
pub(crate) struct DiscoveryChangedEvent {
    inner: Arc<DiscoveryChangedEventInner>,
}

#[derive(Default)]
struct DiscoveryChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, DiscoveryChangedDelegate>>,
}

impl DiscoveryChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: DiscoveryChangedDelegate,
    ) -> DiscoveryChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock_delegates(&self.inner.delegates).insert(identifier, delegate);
        DiscoveryChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, change: DiscoveryChange) {
        let delegates = lock_delegates(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(change.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the discovery delegate"]
pub struct DiscoveryChangedSubscription {
    event: Weak<DiscoveryChangedEventInner>,
    identifier: Option<u64>,
}

impl DiscoveryChangedSubscription {
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

impl Drop for DiscoveryChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiscoveryStatus {
    Idle,
    Active,
    Unavailable,
}

#[derive(Clone)]
pub struct DiscoveryStatusChangedDelegate {
    callback: Arc<dyn Fn(DiscoveryStatus) + Send + Sync + 'static>,
}

impl DiscoveryStatusChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(DiscoveryStatus) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, status: DiscoveryStatus) {
        (self.callback)(status);
    }
}

#[derive(Clone, Default)]
pub(crate) struct DiscoveryStatusChangedEvent {
    inner: Arc<DiscoveryStatusChangedEventInner>,
}

#[derive(Default)]
struct DiscoveryStatusChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, DiscoveryStatusChangedDelegate>>,
}

impl DiscoveryStatusChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: DiscoveryStatusChangedDelegate,
    ) -> DiscoveryStatusChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock_status_delegates(&self.inner.delegates).insert(identifier, delegate);
        DiscoveryStatusChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, status: DiscoveryStatus) {
        let delegates = lock_status_delegates(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(status);
        }
    }
}

#[must_use = "dropping the subscription unregisters the discovery status delegate"]
pub struct DiscoveryStatusChangedSubscription {
    event: Weak<DiscoveryStatusChangedEventInner>,
    identifier: Option<u64>,
}

impl DiscoveryStatusChangedSubscription {
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
        lock_status_delegates(&event.delegates).remove(&identifier);
    }
}

impl Drop for DiscoveryStatusChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock_delegates(
    delegates: &Mutex<BTreeMap<u64, DiscoveryChangedDelegate>>,
) -> MutexGuard<'_, BTreeMap<u64, DiscoveryChangedDelegate>> {
    delegates.lock().unwrap_or_else(|error| error.into_inner())
}

fn lock_status_delegates(
    delegates: &Mutex<BTreeMap<u64, DiscoveryStatusChangedDelegate>>,
) -> MutexGuard<'_, BTreeMap<u64, DiscoveryStatusChangedDelegate>> {
    delegates.lock().unwrap_or_else(|error| error.into_inner())
}

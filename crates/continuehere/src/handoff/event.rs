use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use super::{HandoffChange, IncomingHandoffChange};

#[derive(Clone)]
pub struct HandoffChangedDelegate {
    callback: Arc<dyn Fn(HandoffChange) + Send + Sync + 'static>,
}

impl HandoffChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(HandoffChange) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, change: HandoffChange) {
        (self.callback)(change);
    }
}

#[derive(Clone, Default)]
pub(crate) struct HandoffChangedEvent {
    inner: Arc<HandoffChangedEventInner>,
}

#[derive(Default)]
struct HandoffChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, HandoffChangedDelegate>>,
}

impl HandoffChangedEvent {
    pub(crate) fn subscribe(&self, delegate: HandoffChangedDelegate) -> HandoffChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock(&self.inner.delegates).insert(identifier, delegate);
        HandoffChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, change: HandoffChange) {
        let delegates = lock(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(change.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the handoff delegate"]
pub struct HandoffChangedSubscription {
    event: Weak<HandoffChangedEventInner>,
    identifier: Option<u64>,
}

impl HandoffChangedSubscription {
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

impl Drop for HandoffChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

#[derive(Clone)]
pub struct IncomingHandoffChangedDelegate {
    callback: Arc<dyn Fn(IncomingHandoffChange) + Send + Sync + 'static>,
}

impl IncomingHandoffChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(IncomingHandoffChange) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, change: IncomingHandoffChange) {
        (self.callback)(change);
    }
}

#[derive(Clone, Default)]
pub(crate) struct IncomingHandoffChangedEvent {
    inner: Arc<IncomingHandoffChangedEventInner>,
}

#[derive(Default)]
struct IncomingHandoffChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, IncomingHandoffChangedDelegate>>,
}

impl IncomingHandoffChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: IncomingHandoffChangedDelegate,
    ) -> IncomingHandoffChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock(&self.inner.delegates).insert(identifier, delegate);
        IncomingHandoffChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, change: IncomingHandoffChange) {
        let delegates = lock(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(change.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the incoming-handoff delegate"]
pub struct IncomingHandoffChangedSubscription {
    event: Weak<IncomingHandoffChangedEventInner>,
    identifier: Option<u64>,
}

impl IncomingHandoffChangedSubscription {
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

impl Drop for IncomingHandoffChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

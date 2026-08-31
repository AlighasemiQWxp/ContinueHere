use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use super::{PairingSession, TrustedDevice};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PairingSessionChange {
    Added(PairingSession),
    Updated(PairingSession),
    Removed(PairingSession),
}

#[derive(Clone)]
pub struct PairingSessionChangedDelegate {
    callback: Arc<dyn Fn(PairingSessionChange) + Send + Sync + 'static>,
}

impl PairingSessionChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(PairingSessionChange) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, change: PairingSessionChange) {
        (self.callback)(change);
    }
}

#[derive(Clone, Default)]
pub(crate) struct PairingSessionChangedEvent {
    inner: Arc<PairingSessionChangedEventInner>,
}

#[derive(Default)]
struct PairingSessionChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, PairingSessionChangedDelegate>>,
}

impl PairingSessionChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: PairingSessionChangedDelegate,
    ) -> PairingSessionChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock(&self.inner.delegates).insert(identifier, delegate);
        PairingSessionChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, change: PairingSessionChange) {
        let delegates = lock(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(change.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the pairing-session delegate"]
pub struct PairingSessionChangedSubscription {
    event: Weak<PairingSessionChangedEventInner>,
    identifier: Option<u64>,
}

impl PairingSessionChangedSubscription {
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

impl Drop for PairingSessionChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrustedDeviceChange {
    Added(TrustedDevice),
    Removed(TrustedDevice),
}

#[derive(Clone)]
pub struct TrustedDeviceChangedDelegate {
    callback: Arc<dyn Fn(TrustedDeviceChange) + Send + Sync + 'static>,
}

impl TrustedDeviceChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(TrustedDeviceChange) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, change: TrustedDeviceChange) {
        (self.callback)(change);
    }
}

#[derive(Clone, Default)]
pub(crate) struct TrustedDeviceChangedEvent {
    inner: Arc<TrustedDeviceChangedEventInner>,
}

#[derive(Default)]
struct TrustedDeviceChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, TrustedDeviceChangedDelegate>>,
}

impl TrustedDeviceChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: TrustedDeviceChangedDelegate,
    ) -> TrustedDeviceChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock(&self.inner.delegates).insert(identifier, delegate);
        TrustedDeviceChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, change: TrustedDeviceChange) {
        let delegates = lock(&self.inner.delegates)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for delegate in delegates {
            delegate.invoke(change.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the trusted-device delegate"]
pub struct TrustedDeviceChangedSubscription {
    event: Weak<TrustedDeviceChangedEventInner>,
    identifier: Option<u64>,
}

impl TrustedDeviceChangedSubscription {
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

impl Drop for TrustedDeviceChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

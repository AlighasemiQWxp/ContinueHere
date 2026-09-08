use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::models::LocalDeviceIdentity;

#[derive(Clone)]
pub struct DeviceIdentityChangedDelegate {
    callback: Arc<dyn Fn(LocalDeviceIdentity) + Send + Sync + 'static>,
}

impl DeviceIdentityChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(LocalDeviceIdentity) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, identity: LocalDeviceIdentity) {
        (self.callback)(identity);
    }
}

#[derive(Clone, Default)]
pub(super) struct DeviceIdentityChangedEvent {
    inner: Arc<DeviceIdentityChangedEventInner>,
}

#[derive(Default)]
struct DeviceIdentityChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, DeviceIdentityChangedDelegate>>,
}

impl DeviceIdentityChangedEvent {
    pub(super) fn subscribe(
        &self,
        delegate: DeviceIdentityChangedDelegate,
    ) -> DeviceIdentityChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock_delegates(&self.inner.delegates).insert(identifier, delegate);
        DeviceIdentityChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(super) fn publish(&self, identity: LocalDeviceIdentity) {
        let delegates: Vec<_> = lock_delegates(&self.inner.delegates)
            .values()
            .cloned()
            .collect();
        for delegate in delegates {
            delegate.invoke(identity.clone());
        }
    }
}

#[must_use = "dropping the subscription unregisters the device identity delegate"]
pub struct DeviceIdentityChangedSubscription {
    event: Weak<DeviceIdentityChangedEventInner>,
    identifier: Option<u64>,
}

impl DeviceIdentityChangedSubscription {
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

impl Drop for DeviceIdentityChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock_delegates(
    delegates: &Mutex<BTreeMap<u64, DeviceIdentityChangedDelegate>>,
) -> MutexGuard<'_, BTreeMap<u64, DeviceIdentityChangedDelegate>> {
    delegates.lock().unwrap_or_else(|error| error.into_inner())
}

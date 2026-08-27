use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use super::Language;

#[derive(Clone)]
pub struct LanguageChangedDelegate {
    callback: Arc<dyn Fn(Language) + Send + Sync + 'static>,
}

impl LanguageChangedDelegate {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(Language) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn invoke(&self, language: Language) {
        (self.callback)(language);
    }
}

#[derive(Clone, Default)]
pub(crate) struct LanguageChangedEvent {
    inner: Arc<LanguageChangedEventInner>,
}

#[derive(Default)]
struct LanguageChangedEventInner {
    next_identifier: AtomicU64,
    delegates: Mutex<BTreeMap<u64, LanguageChangedDelegate>>,
}

impl LanguageChangedEvent {
    pub(crate) fn subscribe(
        &self,
        delegate: LanguageChangedDelegate,
    ) -> LanguageChangedSubscription {
        let identifier = self.inner.next_identifier.fetch_add(1, Ordering::Relaxed);
        lock_delegates(&self.inner.delegates).insert(identifier, delegate);
        LanguageChangedSubscription {
            event: Arc::downgrade(&self.inner),
            identifier: Some(identifier),
        }
    }

    pub(crate) fn publish(&self, language: Language) {
        let delegates: Vec<_> = lock_delegates(&self.inner.delegates)
            .values()
            .cloned()
            .collect();
        for delegate in delegates {
            delegate.invoke(language);
        }
    }
}

#[must_use = "dropping the subscription unregisters the language delegate"]
pub struct LanguageChangedSubscription {
    event: Weak<LanguageChangedEventInner>,
    identifier: Option<u64>,
}

impl LanguageChangedSubscription {
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

impl Drop for LanguageChangedSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

fn lock_delegates(
    delegates: &Mutex<BTreeMap<u64, LanguageChangedDelegate>>,
) -> MutexGuard<'_, BTreeMap<u64, LanguageChangedDelegate>> {
    delegates.lock().unwrap_or_else(|error| error.into_inner())
}

mod base_handle;
mod handle_provider;
mod handle_reference;
mod usage_handle;

pub(crate) use base_handle::{BaseHandle, Handle, HandleError, HandleState};
pub(crate) use handle_provider::BaseHandleProvider;
pub(crate) use handle_reference::HandleReference;
pub(crate) use usage_handle::UsageHandle;

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{
        BaseHandle, BaseHandleProvider, Handle, HandleError, HandleReference, HandleState,
        UsageHandle,
    };

    struct TestHandle {
        base: BaseHandle,
        events: Arc<Mutex<Vec<&'static str>>>,
        release_allowed: bool,
    }

    impl TestHandle {
        fn new(identifier: String, events: Arc<Mutex<Vec<&'static str>>>) -> Self {
            Self {
                base: BaseHandle::new(identifier),
                events,
                release_allowed: true,
            }
        }

        fn block_release(mut self) -> Self {
            self.release_allowed = false;
            self
        }

        fn record(&self, event: &'static str) {
            let mut events = match self.events.lock() {
                Ok(events) => events,
                Err(_) => panic!("test event synchronization failed"),
            };

            events.push(event);
        }
    }

    impl Handle for TestHandle {
        fn base(&self) -> &BaseHandle {
            &self.base
        }

        fn base_mut(&mut self) -> &mut BaseHandle {
            &mut self.base
        }

        fn can_release(&mut self) -> bool {
            self.release_allowed
        }

        fn on_use_begin(&mut self) -> Result<(), HandleError> {
            self.record("use_begin");
            Ok(())
        }

        fn on_use_end(&mut self) {
            self.record("use_end");
        }

        fn on_release_beginning(&mut self) {
            self.record("release_beginning");
        }

        fn on_usage_releasing(&mut self) {
            self.record("usage_releasing");
        }

        fn on_release_finished(&mut self) {
            self.record("release_finished");
        }

        fn on_released(&mut self) {
            self.record("released");
        }
    }

    impl UsageHandle for TestHandle {
        type Config = u32;

        fn on_configure(&mut self, config: Self::Config) -> Result<(), HandleError> {
            if config == 0 {
                return Err(HandleError::Rejected(
                    "configuration must be greater than zero",
                ));
            }

            self.record("configured");
            Ok(())
        }
    }

    fn get_handle(
        provider: &mut BaseHandleProvider<TestHandle>,
        identifier: &str,
        events: &Arc<Mutex<Vec<&'static str>>>,
    ) -> HandleReference<TestHandle> {
        let events = Arc::clone(events);
        let result = provider.get_handle(identifier, move |identifier| {
            TestHandle::new(identifier, events)
        });

        match result {
            Ok(handle) => handle,
            Err(error) => panic!("failed to get test handle: {error}"),
        }
    }

    fn assert_state(handle: &HandleReference<TestHandle>, expected: HandleState) {
        let state = match handle.state() {
            Ok(state) => state,
            Err(error) => panic!("failed to read handle state: {error}"),
        };

        assert_eq!(state, expected);
    }

    fn assert_success(result: Result<(), HandleError>) {
        if let Err(error) = result {
            panic!("handle operation failed: {error}");
        }
    }

    fn active_handle_count(provider: &BaseHandleProvider<TestHandle>) -> usize {
        match provider.active_handle_count() {
            Ok(count) => count,
            Err(error) => panic!("failed to count active handles: {error}"),
        }
    }

    #[test]
    fn provider_reuses_an_active_handle_with_the_same_identifier() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut provider = BaseHandleProvider::new();
        let first = get_handle(&mut provider, "transfer", &events);
        let second = get_handle(&mut provider, "transfer", &events);

        assert!(first.same_handle(&second));
        assert_eq!(active_handle_count(&provider), 1);
    }

    #[test]
    fn provider_rejects_an_empty_identifier() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut provider = BaseHandleProvider::new();
        let result =
            provider.get_handle("   ", move |identifier| TestHandle::new(identifier, events));

        assert!(matches!(result, Err(HandleError::EmptyIdentifier)));
        assert_eq!(active_handle_count(&provider), 0);
    }

    #[test]
    fn usage_handle_follows_the_expected_lifecycle() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut provider = BaseHandleProvider::new();
        let handle = get_handle(&mut provider, "transfer", &events);

        assert_success(handle.configure(1));
        assert_success(handle.use_handle());
        assert_state(&handle, HandleState::Using);

        assert_success(handle.complete_use());
        assert_state(&handle, HandleState::Used);

        let release_result = handle.release();
        assert_eq!(release_result, Ok(true));
        assert_state(&handle, HandleState::Released);
        assert_eq!(active_handle_count(&provider), 0);

        let recorded = match events.lock() {
            Ok(events) => events.clone(),
            Err(_) => panic!("test event synchronization failed"),
        };

        assert_eq!(
            recorded,
            vec![
                "configured",
                "use_begin",
                "use_end",
                "release_beginning",
                "usage_releasing",
                "release_finished",
                "released",
            ]
        );
    }

    #[test]
    fn release_after_use_waits_until_use_is_complete() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut provider = BaseHandleProvider::new();
        let handle = get_handle(&mut provider, "handoff", &events);

        assert_success(handle.configure(1));
        assert_success(handle.use_handle());
        assert_success(handle.release_after_use());
        assert_state(&handle, HandleState::Using);

        assert_success(handle.complete_use());
        assert_state(&handle, HandleState::Released);
        assert_eq!(active_handle_count(&provider), 0);
    }

    #[test]
    fn use_requires_configuration() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut provider = BaseHandleProvider::new();
        let handle = get_handle(&mut provider, "activity", &events);

        let result = handle.use_handle();

        assert_eq!(
            result,
            Err(HandleError::NotConfigured {
                identifier: "activity".to_owned(),
            })
        );
        assert_state(&handle, HandleState::Idle);
    }

    #[test]
    fn release_is_safe_to_call_more_than_once() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut provider = BaseHandleProvider::new();
        let handle = get_handle(&mut provider, "device", &events);

        let first_release = handle.release();
        let second_release = handle.release();

        assert_eq!(first_release, Ok(true));
        assert_eq!(second_release, Ok(false));
    }

    #[test]
    fn invalidate_bypasses_a_blocked_release() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let create_events = Arc::clone(&events);
        let mut provider = BaseHandleProvider::new();
        let result = provider.get_handle("locked", move |identifier| {
            TestHandle::new(identifier, create_events).block_release()
        });
        let handle = match result {
            Ok(handle) => handle,
            Err(error) => panic!("failed to get test handle: {error}"),
        };

        assert_eq!(handle.release(), Ok(false));
        assert_state(&handle, HandleState::Idle);
        assert_eq!(active_handle_count(&provider), 1);

        assert_eq!(handle.invalidate(), Ok(true));
        assert_state(&handle, HandleState::Released);
        assert_eq!(active_handle_count(&provider), 0);
    }

    #[test]
    fn provider_creates_a_new_handle_after_release() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut provider = BaseHandleProvider::new();
        let first = get_handle(&mut provider, "transfer", &events);

        assert_eq!(first.release(), Ok(true));

        let second = get_handle(&mut provider, "transfer", &events);
        assert!(!first.same_handle(&second));
        assert_state(&second, HandleState::Idle);
        assert_eq!(active_handle_count(&provider), 1);
    }

    #[test]
    fn provider_releases_all_registered_handles() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut provider = BaseHandleProvider::new();
        let first = get_handle(&mut provider, "first", &events);
        let create_events = Arc::clone(&events);
        let result = provider.get_handle("second", move |identifier| {
            TestHandle::new(identifier, create_events).block_release()
        });
        let second = match result {
            Ok(handle) => handle,
            Err(error) => panic!("failed to get test handle: {error}"),
        };

        assert_success(provider.release_all());

        assert_state(&first, HandleState::Released);
        assert_state(&second, HandleState::Released);
        assert_eq!(active_handle_count(&provider), 0);
    }
}

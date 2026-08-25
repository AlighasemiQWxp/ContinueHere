use crate::{Error, Result};

use super::module::Module;

#[derive(Default)]
pub(crate) struct ModuleRegistry {
    modules: Vec<RegisteredModule>,
}

impl ModuleRegistry {
    #[allow(
        dead_code,
        reason = "optional modules will be registered when their features are implemented"
    )]
    pub(crate) fn register<M>(&mut self, module: M) -> Result<()>
    where
        M: Module + 'static,
    {
        let name = module.name();

        if self.modules.iter().any(|entry| entry.module.name() == name) {
            return Err(Error::DuplicateModule { name });
        }

        self.modules.push(RegisteredModule {
            module: Box::new(module),
            started: false,
        });

        Ok(())
    }

    pub(crate) async fn start_all(&mut self) -> Result<()> {
        for index in 0..self.modules.len() {
            let name = self.modules[index].module.name();
            let result = self.modules[index].module.start().await;

            if let Err(source) = result {
                self.stop_started_before(index).await;
                return Err(Error::ModuleStart { name, source });
            }

            self.modules[index].started = true;
        }

        Ok(())
    }

    pub(crate) async fn stop_all(&mut self) -> Result<()> {
        let mut first_error = None;

        for entry in self.modules.iter_mut().rev() {
            if !entry.started {
                continue;
            }

            let name = entry.module.name();
            let result = entry.module.stop().await;
            entry.started = false;

            if first_error.is_none() {
                if let Err(source) = result {
                    first_error = Some(Error::ModuleStop { name, source });
                }
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    async fn stop_started_before(&mut self, end: usize) {
        for entry in self.modules[..end].iter_mut().rev() {
            if entry.started {
                let _ = entry.module.stop().await;
                entry.started = false;
            }
        }
    }
}

struct RegisteredModule {
    module: Box<dyn Module>,
    started: bool,
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::Error;

    use super::{Module, ModuleRegistry};
    use crate::core::error::ModuleError;

    type EventLog = Arc<Mutex<Vec<&'static str>>>;

    struct TestModule {
        name: &'static str,
        start_event: &'static str,
        stop_event: &'static str,
        events: EventLog,
        fail_start: bool,
        fail_stop: bool,
    }

    impl TestModule {
        fn new(
            name: &'static str,
            start_event: &'static str,
            stop_event: &'static str,
            events: EventLog,
        ) -> Self {
            Self {
                name,
                start_event,
                stop_event,
                events,
                fail_start: false,
                fail_stop: false,
            }
        }

        fn with_start_failure(mut self) -> Self {
            self.fail_start = true;
            self
        }

        fn with_stop_failure(mut self) -> Self {
            self.fail_stop = true;
            self
        }

        fn record(&self, event: &'static str) {
            self.events
                .lock()
                .expect("the event log should be available")
                .push(event);
        }
    }

    #[async_trait]
    impl Module for TestModule {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn start(&mut self) -> Result<(), ModuleError> {
            self.record(self.start_event);

            if self.fail_start {
                return Err(std::io::Error::other("start failed").into());
            }

            Ok(())
        }

        async fn stop(&mut self) -> Result<(), ModuleError> {
            self.record(self.stop_event);

            if self.fail_stop {
                return Err(std::io::Error::other("stop failed").into());
            }

            Ok(())
        }
    }

    #[test]
    fn duplicate_module_names_are_rejected() {
        let events = EventLog::default();
        let mut registry = ModuleRegistry::default();

        registry
            .register(TestModule::new(
                "device",
                "start-device",
                "stop-device",
                Arc::clone(&events),
            ))
            .expect("the first module should be registered");

        let result = registry.register(TestModule::new(
            "device",
            "start-duplicate",
            "stop-duplicate",
            events,
        ));

        assert!(matches!(
            result,
            Err(Error::DuplicateModule { name: "device" })
        ));
    }

    #[tokio::test]
    async fn modules_start_in_order_and_stop_in_reverse_order() {
        let events = EventLog::default();
        let mut registry = ModuleRegistry::default();

        registry
            .register(TestModule::new(
                "first",
                "start-first",
                "stop-first",
                Arc::clone(&events),
            ))
            .expect("the first module should be registered");
        registry
            .register(TestModule::new(
                "second",
                "start-second",
                "stop-second",
                Arc::clone(&events),
            ))
            .expect("the second module should be registered");

        registry.start_all().await.expect("startup should succeed");
        registry.stop_all().await.expect("shutdown should succeed");

        assert_eq!(
            *events.lock().expect("the event log should be available"),
            vec!["start-first", "start-second", "stop-second", "stop-first"]
        );
    }

    #[tokio::test]
    async fn startup_failure_rolls_back_started_modules() {
        let events = EventLog::default();
        let mut registry = ModuleRegistry::default();

        registry
            .register(TestModule::new(
                "first",
                "start-first",
                "stop-first",
                Arc::clone(&events),
            ))
            .expect("the first module should be registered");
        registry
            .register(
                TestModule::new("second", "start-second", "stop-second", Arc::clone(&events))
                    .with_start_failure(),
            )
            .expect("the second module should be registered");

        let result = registry.start_all().await;

        assert!(matches!(
            result,
            Err(Error::ModuleStart { name: "second", .. })
        ));
        assert_eq!(
            *events.lock().expect("the event log should be available"),
            vec!["start-first", "start-second", "stop-first"]
        );
    }

    #[tokio::test]
    async fn shutdown_continues_after_a_module_fails() {
        let events = EventLog::default();
        let mut registry = ModuleRegistry::default();

        registry
            .register(TestModule::new(
                "first",
                "start-first",
                "stop-first",
                Arc::clone(&events),
            ))
            .expect("the first module should be registered");
        registry
            .register(
                TestModule::new("second", "start-second", "stop-second", Arc::clone(&events))
                    .with_stop_failure(),
            )
            .expect("the second module should be registered");

        registry.start_all().await.expect("startup should succeed");
        let result = registry.stop_all().await;

        assert!(matches!(
            result,
            Err(Error::ModuleStop { name: "second", .. })
        ));
        assert_eq!(
            *events.lock().expect("the event log should be available"),
            vec!["start-first", "start-second", "stop-second", "stop-first"]
        );
    }
}

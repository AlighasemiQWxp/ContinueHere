use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
};

use continuehere::ContinueHere;
use slint::ComponentHandle;
use tokio::runtime::Runtime;

use super::{
    MainWindow, UiManager,
    support::{EventTarget, UiResult},
};

type StartupResult = Arc<Mutex<Option<Result<ContinueHere, String>>>>;

pub(crate) struct StartupUiController {
    manager: Rc<RefCell<Option<UiManager>>>,
    result: StartupResult,
    worker: Option<thread::JoinHandle<()>>,
}

impl StartupUiController {
    pub(crate) fn start(runtime: &Runtime, window: &MainWindow) -> UiResult<Self> {
        let manager = Rc::new(RefCell::new(None));
        let result: StartupResult = Arc::new(Mutex::new(None));
        let completed_result = Arc::clone(&result);
        let completed_manager = Rc::downgrade(&manager);
        let view = window.as_weak();
        window.on_startup_completed(move || {
            if let (Some(manager), Some(window)) = (completed_manager.upgrade(), view.upgrade()) {
                let result = completed_result
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .take();
                match result {
                    Some(Ok(core)) => {
                        *manager.borrow_mut() = Some(UiManager::start(Rc::new(core), &window))
                    }
                    Some(Err(error)) => window.set_error_message(error.into()),
                    None => {}
                }
            }
        });
        let target = EventTarget::new(window);
        let worker_result = Arc::clone(&result);
        let runtime = runtime.handle().clone();
        let worker = thread::Builder::new()
            .name("client-startup".into())
            .spawn(move || {
                let result = match crate::platform::project_directory() {
                    Ok(path) => runtime
                        .block_on(ContinueHere::builder(path).build())
                        .map_err(|error| error.to_string()),
                    Err(error) => Err(error.to_string()),
                };
                *worker_result
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()) = Some(result);
                target.dispatch(|window| window.invoke_startup_completed());
            })?;
        Ok(Self {
            manager,
            result,
            worker: Some(worker),
        })
    }

    pub(crate) fn shutdown(mut self, runtime: &Runtime) -> UiResult {
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| "Startup worker failed.")?;
        }
        if let Some(manager) = self.manager.borrow_mut().take() {
            manager.shutdown(runtime)?;
        }
        if let Some(Ok(core)) = self
            .result
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            runtime.block_on(core.shutdown())?;
        }
        Ok(())
    }
}

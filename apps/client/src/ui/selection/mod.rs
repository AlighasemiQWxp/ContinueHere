use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use slint::ComponentHandle;

use super::{
    MainWindow,
    shared::{EventTarget, UiResult},
};

type SelectionAction = Box<dyn FnOnce(&MainWindow, crate::platform::SelectionResult) + 'static>;
type SelectionInbox = Arc<Mutex<BTreeMap<i32, crate::platform::SelectionResult>>>;

pub(super) struct ContentSelectionUiController {
    inbox: SelectionInbox,
    next_request: i32,
    pending: BTreeMap<i32, SelectionAction>,
}

impl ContentSelectionUiController {
    pub(super) fn start(window: &MainWindow) -> Rc<RefCell<Self>> {
        let controller = Rc::new(RefCell::new(Self {
            inbox: Arc::new(Mutex::new(BTreeMap::new())),
            next_request: 0,
            pending: BTreeMap::new(),
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_content_selection_completed(move |request| {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            let Some(window) = view.upgrade() else {
                return;
            };
            let (action, result) = {
                let mut controller = controller.borrow_mut();
                let action = controller.pending.remove(&request);
                let result = controller
                    .inbox
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .remove(&request);
                (action, result)
            };
            if let (Some(action), Some(result)) = (action, result) {
                action(&window, result);
            }
        });
        controller
    }

    pub(super) fn choose_file(
        &mut self,
        window: &MainWindow,
        kind: crate::platform::SelectionKind,
        action: impl FnOnce(&MainWindow, crate::platform::SelectionResult) + 'static,
    ) -> UiResult {
        self.start_selection(window, action, move |completion| {
            crate::platform::select_file(kind, completion)
        })
    }

    pub(super) fn choose_directory(
        &mut self,
        window: &MainWindow,
        action: impl FnOnce(&MainWindow, crate::platform::SelectionResult) + 'static,
    ) -> UiResult {
        self.start_selection(window, action, move |completion| {
            crate::platform::select_directory(completion)
        })
    }

    fn start_selection(
        &mut self,
        window: &MainWindow,
        action: impl FnOnce(&MainWindow, crate::platform::SelectionResult) + 'static,
        select: impl FnOnce(
            Box<dyn FnOnce(crate::platform::SelectionResult) + Send>,
        ) -> crate::platform::PlatformResult<()>,
    ) -> UiResult {
        if !self.pending.is_empty() {
            return Err("Finish the current content selection first.".into());
        }
        self.next_request = self.next_request.checked_add(1).unwrap_or(1);
        let request = self.next_request;
        self.pending.insert(request, Box::new(action));
        let inbox = Arc::clone(&self.inbox);
        let target = EventTarget::new(window);
        let result = select(Box::new(move |result| {
            inbox
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .insert(request, result);
            target.dispatch(move |window| {
                window.invoke_content_selection_completed(request);
            });
        }));
        if let Err(error) = result {
            self.pending.remove(&request);
            return Err(error);
        }
        Ok(())
    }
}

pub(super) fn selected_path(result: crate::platform::SelectionResult) -> UiResult<Option<PathBuf>> {
    result.map_err(Into::into)
}

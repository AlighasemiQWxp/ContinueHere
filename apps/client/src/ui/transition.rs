use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use slint::ComponentHandle;

use super::MainWindow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UiTransition {
    Preview = 1,
    Reconnect = 2,
    Error = 3,
}

struct UiTransitionOperation {
    identifier: String,
    transition: Option<UiTransition>,
    active: bool,
    released: bool,
}

struct UiTransitionState {
    window: slint::Weak<MainWindow>,
    handles: Vec<Rc<RefCell<UiTransitionOperation>>>,
}

pub(super) struct UiTransitionHandle {
    operation: Rc<RefCell<UiTransitionOperation>>,
    state: Weak<RefCell<UiTransitionState>>,
}

impl UiTransitionHandle {
    pub(super) fn configure(&self, transition: UiTransition) -> Result<(), &'static str> {
        let mut operation = self.operation.borrow_mut();
        if operation.active || operation.released {
            return Err("This UI transition cannot be configured now.");
        }
        operation.transition = Some(transition);
        Ok(())
    }

    pub(super) fn use_handle(&self) -> Result<(), &'static str> {
        let Some(state) = self.state.upgrade() else {
            return Err("The UI transition controller is unavailable.");
        };
        let operation = self.operation.borrow();
        if operation.released {
            return Err("This UI transition was already released.");
        }
        if operation.transition.is_none() {
            return Err("Configure the UI transition before opening it.");
        }
        drop(operation);
        self.operation.borrow_mut().active = true;
        {
            let mut state = state.borrow_mut();
            activate(&mut state.handles, &self.operation);
        }
        refresh(&state);
        Ok(())
    }

    pub(super) fn release(&self) -> bool {
        let mut operation = self.operation.borrow_mut();
        if operation.released {
            return false;
        }
        operation.active = false;
        operation.released = true;
        drop(operation);
        let Some(state) = self.state.upgrade() else {
            return true;
        };
        remove(&mut state.borrow_mut().handles, &self.operation);
        refresh(&state);
        true
    }
}

impl Drop for UiTransitionHandle {
    fn drop(&mut self) {
        self.release();
    }
}

pub(super) struct UiTransitionController {
    state: Rc<RefCell<UiTransitionState>>,
    error: RefCell<Option<UiTransitionHandle>>,
}

impl UiTransitionController {
    pub(super) fn start(window: &MainWindow) -> Rc<Self> {
        let controller = Rc::new(Self {
            state: Rc::new(RefCell::new(UiTransitionState {
                window: window.as_weak(),
                handles: Vec::new(),
            })),
            error: RefCell::new(None),
        });
        let weak = Rc::downgrade(&controller);
        window.on_show_error_requested(move |message| {
            if let Some(controller) = weak.upgrade() {
                controller.show_error(message.as_str());
            }
        });
        let weak = Rc::downgrade(&controller);
        window.on_dismiss_error_requested(move || {
            if let Some(controller) = weak.upgrade() {
                controller.dismiss_error();
            }
        });
        controller
    }

    pub(super) fn get_handle(&self, identifier: &str) -> UiTransitionHandle {
        if let Some(operation) = self
            .state
            .borrow()
            .handles
            .iter()
            .find(|operation| operation.borrow().identifier == identifier)
            .cloned()
        {
            return UiTransitionHandle {
                operation,
                state: Rc::downgrade(&self.state),
            };
        }
        let operation = Rc::new(RefCell::new(UiTransitionOperation {
            identifier: identifier.to_owned(),
            transition: None,
            active: false,
            released: false,
        }));
        UiTransitionHandle {
            operation,
            state: Rc::downgrade(&self.state),
        }
    }

    fn show_error(&self, message: &str) {
        if let Some(window) = self.state.borrow().window.upgrade() {
            window.set_error_message(message.into());
        }
        let mut current = self.error.borrow_mut();
        if let Some(handle) = current.as_ref() {
            let _result = handle.use_handle();
            return;
        }
        let handle = self.get_handle("error");
        if handle.configure(UiTransition::Error).is_ok() && handle.use_handle().is_ok() {
            *current = Some(handle);
        }
    }

    fn dismiss_error(&self) {
        if let Some(window) = self.state.borrow().window.upgrade() {
            window.set_error_message("".into());
        }
        self.error.borrow_mut().take();
    }
}

impl Drop for UiTransitionController {
    fn drop(&mut self) {
        self.error.get_mut().take();
        let window = {
            let mut state = self.state.borrow_mut();
            state.handles.clear();
            state.window.clone()
        };
        if let Some(window) = window.upgrade() {
            window.set_active_transition(0);
        }
    }
}

fn refresh(state: &Rc<RefCell<UiTransitionState>>) {
    let (window, transition) = {
        let state = state.borrow();
        let transition = active_transition(&state.handles);
        (state.window.clone(), transition)
    };
    if let Some(window) = window.upgrade() {
        let value = transition.map_or(0, |transition| transition as i32);
        window.set_active_transition(value);
    }
}

fn activate(
    handles: &mut Vec<Rc<RefCell<UiTransitionOperation>>>,
    operation: &Rc<RefCell<UiTransitionOperation>>,
) {
    let identifier = operation.borrow().identifier.clone();
    handles.retain(|handle| handle.borrow().identifier != identifier);
    handles.push(Rc::clone(operation));
}

fn remove(
    handles: &mut Vec<Rc<RefCell<UiTransitionOperation>>>,
    operation: &Rc<RefCell<UiTransitionOperation>>,
) {
    handles.retain(|handle| !Rc::ptr_eq(handle, operation));
}

fn active_transition(handles: &[Rc<RefCell<UiTransitionOperation>>]) -> Option<UiTransition> {
    handles.last().and_then(|handle| {
        let operation = handle.borrow();
        if operation.active {
            operation.transition
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Rc, RefCell, UiTransition, UiTransitionOperation, activate, active_transition, remove,
    };

    #[test]
    fn releasing_the_top_transition_reveals_the_previous_surface() {
        let preview = operation("preview", UiTransition::Preview);
        let error = operation("error", UiTransition::Error);
        let mut handles = Vec::new();

        activate(&mut handles, &preview);
        activate(&mut handles, &error);
        assert_eq!(active_transition(&handles), Some(UiTransition::Error));

        remove(&mut handles, &error);
        assert_eq!(active_transition(&handles), Some(UiTransition::Preview));

        remove(&mut handles, &preview);
        assert_eq!(active_transition(&handles), None);
    }

    #[test]
    fn reopening_an_identifier_moves_one_handle_to_the_top() {
        let preview = operation("preview", UiTransition::Preview);
        let reconnect = operation("reconnect", UiTransition::Reconnect);
        let mut handles = Vec::new();

        activate(&mut handles, &preview);
        activate(&mut handles, &reconnect);
        activate(&mut handles, &preview);

        assert_eq!(handles.len(), 2);
        assert_eq!(active_transition(&handles), Some(UiTransition::Preview));
    }

    fn operation(identifier: &str, transition: UiTransition) -> Rc<RefCell<UiTransitionOperation>> {
        Rc::new(RefCell::new(UiTransitionOperation {
            identifier: identifier.to_owned(),
            transition: Some(transition),
            active: true,
            released: false,
        }))
    }
}

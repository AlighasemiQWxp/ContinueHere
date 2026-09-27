use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseChange<P> {
    pub previous: P,
    pub current: P,
}

pub struct PhaseChangedDelegate<P> {
    callback: Rc<dyn Fn(PhaseChange<P>)>,
}

impl<P> PhaseChangedDelegate<P> {
    pub fn new(callback: impl Fn(PhaseChange<P>) + 'static) -> Self {
        Self {
            callback: Rc::new(callback),
        }
    }
}

type Listeners<P> = RefCell<Vec<Weak<PhaseChangedDelegate<P>>>>;

#[must_use = "dropping the subscription unregisters the phase-change delegate"]
pub struct PhaseChangedSubscription<P> {
    delegate: Rc<PhaseChangedDelegate<P>>,
    listeners: Weak<Listeners<P>>,
}

impl<P> Drop for PhaseChangedSubscription<P> {
    fn drop(&mut self) {
        if let Some(listeners) = self.listeners.upgrade() {
            let delegate = Rc::downgrade(&self.delegate);
            listeners
                .borrow_mut()
                .retain(|item| !item.ptr_eq(&delegate));
        }
    }
}

pub struct PhaseController<P> {
    phase: P,
    listeners: Rc<Listeners<P>>,
}

impl<P: Clone + PartialEq> PhaseController<P> {
    pub fn new(initial: P) -> Self {
        Self {
            phase: initial,
            listeners: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn phase(&self) -> &P {
        &self.phase
    }

    pub fn transition_to(&mut self, next: P) -> bool {
        if self.phase == next {
            return false;
        }
        let previous = std::mem::replace(&mut self.phase, next);
        let change = PhaseChange {
            previous,
            current: self.phase.clone(),
        };
        let listeners = self.listeners.borrow().clone();
        for listener in listeners {
            if let Some(delegate) = listener.upgrade() {
                (delegate.callback)(change.clone());
            }
        }
        true
    }

    pub fn on_phase_changed(
        &self,
        delegate: PhaseChangedDelegate<P>,
    ) -> PhaseChangedSubscription<P> {
        let delegate = Rc::new(delegate);
        self.listeners.borrow_mut().push(Rc::downgrade(&delegate));
        PhaseChangedSubscription {
            delegate,
            listeners: Rc::downgrade(&self.listeners),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_notify_once_and_dropping_subscription_stops_notifications() {
        let mut phases = PhaseController::new(0);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let received = Rc::clone(&changes);
        let subscription = phases.on_phase_changed(PhaseChangedDelegate::new(move |change| {
            received.borrow_mut().push(change);
        }));
        assert!(!phases.transition_to(0));
        assert!(phases.transition_to(1));
        assert!(!phases.transition_to(1));
        assert_eq!(*phases.phase(), 1);
        assert_eq!(
            *changes.borrow(),
            vec![PhaseChange {
                previous: 0,
                current: 1,
            }]
        );
        drop(subscription);
        assert!(phases.transition_to(2));
        assert_eq!(changes.borrow().len(), 1);
    }

    #[test]
    fn listener_can_unsubscribe_another_listener_during_notification() {
        let mut phases = PhaseController::new(false);
        let later = Rc::new(RefCell::new(None));
        let removed = Rc::clone(&later);
        let _first = phases.on_phase_changed(PhaseChangedDelegate::new(move |_| {
            removed.borrow_mut().take();
        }));
        let called = Rc::new(RefCell::new(false));
        let observed = Rc::clone(&called);
        *later.borrow_mut() = Some(phases.on_phase_changed(PhaseChangedDelegate::new(move |_| {
            *observed.borrow_mut() = true;
        })));
        phases.transition_to(true);
        assert!(!*called.borrow());
    }
}

mod access;
mod presentation;

pub(in crate::ui) use access::ShellAccess;

use std::{cell::RefCell, rc::Rc, time::Duration};

use continuehere::{
    AppearanceChangedDelegate, AppearanceChangedSubscription, LanguageChangedDelegate,
    LanguageChangedSubscription,
};
use slint::ComponentHandle;

use super::{MainWindow, history::HistoryNavigation, shared::EventTarget};

pub(super) struct ShellController {
    access: ShellAccess,
    history: RefCell<Option<HistoryNavigation>>,
    _appearance_changed: AppearanceChangedSubscription,
    _language_changed: LanguageChangedSubscription,
}

impl ShellController {
    pub(super) fn start(access: ShellAccess, window: &MainWindow) -> Rc<Self> {
        let target = EventTarget::new(window);
        let language_target = target.clone();
        let appearance_changed = access
            .appearance()
            .on_changed(AppearanceChangedDelegate::new(move |_| {
                target.dispatch(|window| window.invoke_refresh_presentation_requested());
            }));
        let language_changed =
            access
                .localization()
                .on_language_changed(LanguageChangedDelegate::new(move |_| {
                    language_target
                        .dispatch(|window| window.invoke_refresh_presentation_requested());
                }));
        let controller = Rc::new(Self {
            access,
            history: RefCell::new(None),
            _appearance_changed: appearance_changed,
            _language_changed: language_changed,
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_refresh_presentation_requested(move || {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                controller.refresh(&window);
            }
        });
        let page_controller = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_page_selected(move |page| {
            let Some(window) = view.upgrade() else {
                return;
            };
            if ![0, 1, 3, 4].contains(&page) || window.get_active_transition() != 0 {
                return;
            }
            window.set_reduce_motion(crate::platform::reduce_motion());
            window.set_page(page);
            match page {
                0 => window.set_devices_unread(false),
                1 => window.set_send_unread(false),
                3 => {
                    window.set_history_unread(false);
                }
                _ => {}
            }
            if let Some(controller) = page_controller.upgrade()
                && let Some(history) = controller.history.borrow().clone()
            {
                if page == 3 {
                    history.show_overview();
                } else {
                    history.refresh();
                }
            }
            window.set_content_opacity(0.0);
            let view = window.as_weak();
            slint::Timer::single_shot(Duration::from_millis(16), move || {
                if let Some(window) = view.upgrade() {
                    window.set_content_opacity(1.0);
                }
            });
        });
        controller.refresh(window);
        controller
    }

    pub(super) fn set_history_navigation(&self, history: HistoryNavigation) {
        *self.history.borrow_mut() = Some(history);
    }

    fn refresh(&self, window: &MainWindow) {
        presentation::apply(&self.access, window);
    }
}

mod snapshot;
use snapshot::{apply_snapshot, snapshot};

mod bindings;

mod access;
pub(super) use access::SettingsAccess;

use crate::ui::shared::EventTarget;

use std::{cell::RefCell, rc::Rc};

use continuehere::{
    DeviceIdentityChangedDelegate, DeviceIdentityChangedSubscription, DirectoryChangedDelegate,
    DirectoryChangedSubscription, Language, LanguageChangedDelegate, LanguageChangedSubscription,
};

use super::{MainWindow, selection::ContentSelectionUiController};

pub(super) struct SettingsUiController {
    access: SettingsAccess,
    _identity_changed: DeviceIdentityChangedSubscription,
    _directory_changed: DirectoryChangedSubscription,
    _language_changed: LanguageChangedSubscription,
}

impl SettingsUiController {
    pub(super) fn start(
        access: SettingsAccess,
        window: &MainWindow,
        selection: Rc<RefCell<ContentSelectionUiController>>,
    ) -> Rc<Self> {
        let event_target = EventTarget::new(window);
        let identity_changed =
            access
                .devices()
                .on_identity_changed(DeviceIdentityChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let directory_changed =
            access
                .directories()
                .on_directory_changed(DirectoryChangedDelegate::new(directory_refresh_delegate(
                    event_target.clone(),
                )));
        let language_changed = access
            .localization()
            .on_language_changed(LanguageChangedDelegate::new(refresh_delegate(event_target)));
        let controller = Rc::new(Self {
            access,
            _identity_changed: identity_changed,
            _directory_changed: directory_changed,
            _language_changed: language_changed,
        });
        Self::bind_callbacks(Rc::downgrade(&controller), window, selection);
        controller.refresh(window);
        controller
    }

    fn refresh(&self, window: &MainWindow) {
        apply_snapshot(window, snapshot(&self.access));
    }
}

fn refresh_delegate<T>(event_target: EventTarget) -> impl Fn(T) + Send + Sync + 'static
where
    T: 'static,
{
    move |_| {
        event_target.dispatch(|window| window.invoke_refresh_settings_requested());
    }
}

fn directory_refresh_delegate(
    event_target: EventTarget,
) -> impl Fn(&std::path::Path) + Send + Sync + 'static {
    move |_| {
        event_target.dispatch(|window| window.invoke_refresh_settings_requested());
    }
}

fn show_result(result: continuehere::Result<()>, window: &slint::Weak<MainWindow>) {
    let Some(window) = window.upgrade() else {
        return;
    };
    if let Err(error) = result {
        window.invoke_show_error_requested(error.to_string().into());
    }
}

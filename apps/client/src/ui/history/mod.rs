mod access;
mod snapshot;

pub(in crate::ui) use access::HistoryAccess;

use snapshot::{activity_row, activity_time, can_open, timestamp};

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use continuehere::{
    Activity, ActivityChangedDelegate, ActivityChangedSubscription, ActivityDirection,
    ActivityKind, ActivityStatus, LanguageChangedDelegate, LanguageChangedSubscription,
};
use slint::ComponentHandle;

use super::{
    ContentRow, MainWindow,
    preview::PreviewOpener,
    shared::{EventTarget, UiResult, model, notify, show_result},
};

pub(super) struct HistoryUiController {
    access: HistoryAccess,
    preview: PreviewOpener,
    revisions: BTreeMap<String, u64>,
    unread: BTreeSet<String>,
    initialized: bool,
    _changed: ActivityChangedSubscription,
    _language_changed: LanguageChangedSubscription,
}

#[derive(Clone, Copy)]
enum HistoryAction {
    SelectDevice,
    Back,
    Clear,
    Remove,
    Retry,
    Open,
}

impl HistoryAction {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "device" => Some(Self::SelectDevice),
            "back" => Some(Self::Back),
            "clear" => Some(Self::Clear),
            "remove" => Some(Self::Remove),
            "retry" => Some(Self::Retry),
            "open" => Some(Self::Open),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(super) struct HistoryNavigation {
    controller: std::rc::Weak<RefCell<HistoryUiController>>,
    window: slint::Weak<MainWindow>,
}

impl HistoryNavigation {
    pub(super) fn refresh(&self) {
        let (Some(controller), Some(window)) = (self.controller.upgrade(), self.window.upgrade())
        else {
            return;
        };
        controller.borrow_mut().refresh(&window);
    }

    pub(super) fn show_overview(&self) {
        let (Some(controller), Some(window)) = (self.controller.upgrade(), self.window.upgrade())
        else {
            return;
        };
        controller.borrow_mut().show_overview(&window);
    }
}

impl HistoryUiController {
    pub(super) fn start(
        access: HistoryAccess,
        preview: PreviewOpener,
        window: &MainWindow,
    ) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let changed = access
            .activity()
            .on_changed(ActivityChangedDelegate::new(move |_| {
                target.dispatch(|window| window.invoke_refresh_history_requested());
            }));
        let language_target = EventTarget::new(window);
        let language_changed =
            access
                .localization()
                .on_language_changed(LanguageChangedDelegate::new(move |_| {
                    language_target.dispatch(|window| window.invoke_refresh_history_requested());
                }));
        let controller = Rc::new(RefCell::new(Self {
            access,
            preview,
            revisions: BTreeMap::new(),
            unread: BTreeSet::new(),
            initialized: false,
            _changed: changed,
            _language_changed: language_changed,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_history_action(move |id, action| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = match HistoryAction::parse(action.as_str()) {
                    Some(action) => controller.borrow_mut().act(id.as_str(), action, &window),
                    None => Err("Unknown history action.".into()),
                };
                show_result(&window, result);
                controller.borrow_mut().refresh(&window);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_refresh_history_requested(move || {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                controller.borrow_mut().refresh(&window);
            }
        });
        controller.borrow_mut().refresh(window);
        controller
    }

    pub(super) fn navigation(
        controller: &Rc<RefCell<Self>>,
        window: &MainWindow,
    ) -> HistoryNavigation {
        HistoryNavigation {
            controller: Rc::downgrade(controller),
            window: window.as_weak(),
        }
    }

    fn show_overview(&mut self, window: &MainWindow) {
        window.set_history_device("".into());
        self.refresh(window);
    }

    fn act(&mut self, id: &str, action: HistoryAction, window: &MainWindow) -> UiResult {
        match action {
            HistoryAction::SelectDevice => {
                self.unread.remove(id);
                window.set_history_device(id.into());
            }
            HistoryAction::Back => window.set_history_device("".into()),
            HistoryAction::Clear => self.access.activity().clear()?,
            HistoryAction::Remove => self.access.activity().remove(id)?,
            HistoryAction::Retry => self.access.activity().retry(id)?,
            HistoryAction::Open => {
                let snapshot = self.access.activity().activities()?;
                let item = snapshot
                    .entries()
                    .iter()
                    .find(|item| item.id() == id)
                    .ok_or("This activity is unavailable.")?;
                if !can_open(item) {
                    return Err("This activity cannot be opened.".into());
                }
                if let Some(path) = item.file_path() {
                    if let Some(continuation) = item.document_continuation() {
                        crate::platform::open_document(path, continuation)?;
                    } else {
                        self.preview
                            .open(path.to_path_buf(), item.position_millis())?;
                    }
                } else if let Some(url) = item.url() {
                    crate::platform::open_url(url)?;
                }
            }
        }
        Ok(())
    }

    fn refresh(&mut self, window: &MainWindow) {
        let snapshot = match self.access.activity().activities() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                window.set_history_error(error.to_string().into());
                return;
            }
        };
        window.set_history_error(snapshot.storage_error().unwrap_or("").into());
        let selected = window.get_history_device();
        let mut devices: BTreeMap<&str, Vec<&Activity>> = BTreeMap::new();
        for item in snapshot.entries() {
            if self.initialized
                && self
                    .revisions
                    .get(item.id())
                    .is_none_or(|revision| *revision < item.revision())
                && !(window.get_page() == 3 && selected.as_str() == item.device_id())
            {
                self.unread.insert(item.device_id().to_owned());
                notify(window, 3);
            }
            devices.entry(item.device_id()).or_default().push(item);
        }
        self.revisions = snapshot
            .entries()
            .iter()
            .map(|item| (item.id().to_owned(), item.revision()))
            .collect();
        self.initialized = true;
        self.unread.retain(|id| devices.contains_key(id.as_str()));
        if self.unread.is_empty() {
            window.set_history_unread(false);
        }
        let mut groups: Vec<_> = devices.into_values().collect();
        for group in &mut groups {
            group.sort_by_key(|item| std::cmp::Reverse((activity_time(item), item.revision())));
        }
        groups.sort_by_key(|group| std::cmp::Reverse(activity_time(group[0])));
        let rtl = window.get_rtl();
        let entries = snapshot.entries();
        window.set_sent_activities(model(
            entries
                .iter()
                .filter(|item| {
                    item.direction() == ActivityDirection::Outgoing
                        && item.kind() != ActivityKind::Session
                })
                .take(100)
                .map(|item| activity_row(item, &self.access, rtl))
                .collect(),
        ));
        window.set_received_activities(model(
            entries
                .iter()
                .filter(|item| {
                    item.direction() == ActivityDirection::Incoming
                        && item.kind() != ActivityKind::Session
                        && item.status() != ActivityStatus::Active
                })
                .take(100)
                .map(|item| activity_row(item, &self.access, rtl))
                .collect(),
        ));
        window.set_history_devices(model(
            groups
                .iter()
                .map(|group| {
                    let item = group[0];
                    ContentRow {
                        connected: self
                            .access
                            .transport()
                            .connections()
                            .iter()
                            .any(|connection| connection.device_id().as_str() == item.device_id()),
                        id: item.device_id().into(),
                        title: item.device_name().into(),
                        detail: format!(
                            "{} · {}\n{}",
                            super::devices::platform_name(item.platform()),
                            group.len(),
                            timestamp(activity_time(item))
                        )
                        .into(),
                        unread: self.unread.contains(item.device_id()),
                        ..Default::default()
                    }
                })
                .collect(),
        ));
        let entries = groups
            .into_iter()
            .find(|group| group[0].device_id() == selected.as_str())
            .unwrap_or_default();
        window.set_history_device_name(
            entries
                .first()
                .map(|item| item.device_name())
                .unwrap_or("")
                .into(),
        );
        window.set_history_entries(model(
            entries
                .into_iter()
                .map(|item| activity_row(item, &self.access, rtl))
                .collect(),
        ));
    }
}

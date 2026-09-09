use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use continuehere::{
    Activity, ActivityChangedDelegate, ActivityChangedSubscription, ActivityDirection,
    ActivityKind, ActivityStatus, ContinueHere,
};
use slint::ComponentHandle;

use super::{
    ContentRow, MainWindow,
    support::{EventTarget, UiResult, model, notify, show_result, text},
};

pub(super) struct HistoryUiController {
    core: Rc<ContinueHere>,
    revisions: BTreeMap<String, u64>,
    unread: BTreeSet<String>,
    initialized: bool,
    _changed: ActivityChangedSubscription,
}

impl HistoryUiController {
    pub(super) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let changed = core
            .activity()
            .on_changed(ActivityChangedDelegate::new(move |_| {
                target.dispatch(|window| window.invoke_refresh_history_requested());
            }));
        let controller = Rc::new(RefCell::new(Self {
            core,
            revisions: BTreeMap::new(),
            unread: BTreeSet::new(),
            initialized: false,
            _changed: changed,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_history_action(move |id, action| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = controller
                    .borrow_mut()
                    .act(id.as_str(), action.as_str(), &window);
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

    fn act(&mut self, id: &str, action: &str, window: &MainWindow) -> UiResult {
        match action {
            "device" => {
                self.unread.remove(id);
                window.set_history_device(id.into());
            }
            "back" => window.set_history_device("".into()),
            "clear" => self.core.activity().clear()?,
            "remove" => self.core.activity().remove(id)?,
            "retry" => self.core.activity().retry(id)?,
            "open" => {
                let snapshot = self.core.activity().activities()?;
                let item = snapshot
                    .entries()
                    .iter()
                    .find(|item| item.id() == id)
                    .ok_or("This activity is unavailable.")?;
                if !can_open(item) {
                    return Err("This activity cannot be opened.".into());
                }
                if let Some(path) = item.file_path() {
                    window.invoke_open_file(
                        path.to_string_lossy().as_ref().into(),
                        item.position_millis().to_string().into(),
                    );
                } else if let Some(url) = item.url() {
                    crate::platform::open_url(url)?;
                }
            }
            _ => return Err("Unknown history action.".into()),
        }
        Ok(())
    }

    fn refresh(&mut self, window: &MainWindow) {
        let snapshot = match self.core.activity().activities() {
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
                .map(|item| activity_row(item, &self.core, rtl))
                .collect(),
        ));
        window.set_received_activities(model(
            entries
                .iter()
                .filter(|item| {
                    item.direction() == ActivityDirection::Incoming
                        && item.kind() != ActivityKind::Session
                })
                .take(100)
                .map(|item| activity_row(item, &self.core, rtl))
                .collect(),
        ));
        window.set_history_devices(model(
            groups
                .iter()
                .map(|group| {
                    let item = group[0];
                    ContentRow {
                        connected: self
                            .core
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
                .map(|item| activity_row(item, &self.core, rtl))
                .collect(),
        ));
    }
}

fn activity_time(item: &Activity) -> u64 {
    [
        Some(item.started_at()),
        item.ended_at(),
        item.completed_at(),
        item.file_completed_at(),
        item.disconnected_at(),
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(item.started_at())
}

fn timestamp(value: u64) -> String {
    i64::try_from(value)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|time| {
            time.with_timezone(&chrono::Local)
                .format("%Y-%m-%d  %H:%M:%S%.3f UTC%:z")
                .to_string()
        })
        .unwrap_or_else(|| value.to_string())
}

fn can_open(item: &Activity) -> bool {
    matches!(
        item.status(),
        ActivityStatus::Completed | ActivityStatus::Delivered
    ) && (item.file_path().is_some() || item.url().is_some())
}

fn activity_row(item: &Activity, core: &ContinueHere, rtl: bool) -> ContentRow {
    let session = item.kind() == ActivityKind::Session;
    let direction = match item.direction() {
        ActivityDirection::Outgoing => text(rtl, "Sent", "ارسالی"),
        ActivityDirection::Incoming => text(rtl, "Received", "دریافتی"),
        ActivityDirection::Connection => text(rtl, "Connection", "اتصال"),
    };
    let mut detail = format!(
        "{}\n{}: {}",
        direction,
        text(rtl, "Started", "شروع"),
        timestamp(item.started_at())
    );
    for (label, value) in [
        (
            text(rtl, "File completed", "تکمیل فایل"),
            item.file_completed_at(),
        ),
        (
            text(rtl, "Disconnected", "قطع اتصال"),
            item.disconnected_at(),
        ),
        (text(rtl, "Ended", "پایان"), item.ended_at()),
    ] {
        if let Some(value) = value {
            detail.push_str(&format!("\n{label}: {}", timestamp(value)));
        }
    }
    if item.status() == ActivityStatus::Interrupted && item.ended_at().is_none() {
        detail.push_str(text(
            rtl,
            "\nEnd time unknown after unexpected exit",
            "\nزمان پایان پس از خروج غیرمنتظره نامشخص است",
        ));
    }
    if item.retry_of().is_some() {
        detail.push_str(text(rtl, "\nRetry attempt", "\nتلاش دوباره"));
    }
    if let Some(failure) = item.failure() {
        detail.push_str(&format!("\n{failure}"));
    }
    let status = match item.status() {
        ActivityStatus::Active => text(rtl, "Active", "فعال"),
        ActivityStatus::Delivered => text(rtl, "Delivered", "تحویل داده شد"),
        ActivityStatus::Completed => text(rtl, "Completed", "تکمیل شد"),
        ActivityStatus::Rejected => text(rtl, "Rejected", "رد شد"),
        ActivityStatus::Cancelled => text(rtl, "Cancelled", "لغو شد"),
        ActivityStatus::Failed => text(rtl, "Failed", "ناموفق"),
        ActivityStatus::Disconnected => text(rtl, "Disconnected", "قطع شده"),
        ActivityStatus::Interrupted => text(rtl, "Interrupted", "متوقف شده"),
    };
    ContentRow {
        id: item.id().into(),
        title: if session {
            text(rtl, "Connection session", "نشست اتصال").into()
        } else {
            item.title().into()
        },
        detail: detail.into(),
        status: status.into(),
        can_open: can_open(item),
        retryable: item.can_retry(),
        can_retry: item.can_retry()
            && core
                .transport()
                .connections()
                .iter()
                .any(|connection| connection.device_id().as_str() == item.device_id()),
        active: item.status() == ActivityStatus::Active,
        ..Default::default()
    }
}

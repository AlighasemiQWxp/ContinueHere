use super::models::{UiPlatform, platform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiActivityEvent {
    Changed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiActivityKind {
    Url,
    YouTube,
    File,
    LocalVideo,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiActivityDirection {
    Outgoing,
    Incoming,
    Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiActivityStatus {
    Active,
    Delivered,
    Completed,
    Rejected,
    Cancelled,
    Failed,
    Disconnected,
    Interrupted,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiActivity {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub platform: UiPlatform,
    pub kind: UiActivityKind,
    pub direction: UiActivityDirection,
    pub status: UiActivityStatus,
    pub title: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub disconnected_at: Option<u64>,
    pub file_completed_at: Option<u64>,
    pub failure: Option<String>,
    pub url: Option<String>,
    pub position_millis: u64,
    pub retry_of: Option<String>,
    pub session_id: Option<String>,
    pub revision: u64,
    pub file_path: Option<String>,
    pub can_retry: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiActivitySnapshot {
    pub entries: Vec<UiActivity>,
    pub storage_error: Option<String>,
}
pub(crate) fn activity(value: &continuehere::Activity) -> UiActivity {
    UiActivity {
        id: value.id().to_owned(),
        device_id: value.device_id().to_owned(),
        device_name: value.device_name().to_owned(),
        platform: platform(value.platform()),
        kind: match value.kind() {
            continuehere::ActivityKind::Url => UiActivityKind::Url,
            continuehere::ActivityKind::YouTube => UiActivityKind::YouTube,
            continuehere::ActivityKind::File => UiActivityKind::File,
            continuehere::ActivityKind::LocalVideo => UiActivityKind::LocalVideo,
            continuehere::ActivityKind::Session => UiActivityKind::Session,
        },
        direction: match value.direction() {
            continuehere::ActivityDirection::Outgoing => UiActivityDirection::Outgoing,
            continuehere::ActivityDirection::Incoming => UiActivityDirection::Incoming,
            continuehere::ActivityDirection::Connection => UiActivityDirection::Connection,
        },
        status: match value.status() {
            continuehere::ActivityStatus::Active => UiActivityStatus::Active,
            continuehere::ActivityStatus::Delivered => UiActivityStatus::Delivered,
            continuehere::ActivityStatus::Completed => UiActivityStatus::Completed,
            continuehere::ActivityStatus::Rejected => UiActivityStatus::Rejected,
            continuehere::ActivityStatus::Cancelled => UiActivityStatus::Cancelled,
            continuehere::ActivityStatus::Failed => UiActivityStatus::Failed,
            continuehere::ActivityStatus::Disconnected => UiActivityStatus::Disconnected,
            continuehere::ActivityStatus::Interrupted => UiActivityStatus::Interrupted,
        },
        title: value.title().to_owned(),
        started_at: value.started_at(),
        ended_at: value.ended_at(),
        completed_at: value.completed_at(),
        disconnected_at: value.disconnected_at(),
        file_completed_at: value.file_completed_at(),
        failure: value.failure().map(str::to_owned),
        url: value.url().map(str::to_owned),
        position_millis: value.position_millis(),
        retry_of: value.retry_of().map(str::to_owned),
        session_id: value.session_id().map(str::to_owned),
        revision: value.revision(),
        file_path: value
            .file_path()
            .map(|path| path.to_string_lossy().into_owned()),
        can_retry: value.can_retry(),
    }
}

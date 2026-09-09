use crate::models::Platform;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityChange {
    device_id: Option<crate::models::DeviceId>,
}
impl ActivityChange {
    pub fn device_id(&self) -> Option<&crate::models::DeviceId> {
        self.device_id.as_ref()
    }
    pub(super) fn new(device_id: &str) -> Self {
        Self {
            device_id: crate::models::DeviceId::new(device_id).ok(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Folder,
    Url,
    YouTube,
    File,
    LocalVideo,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityDirection {
    Outgoing,
    Incoming,
    Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityStatus {
    Active,
    Delivered,
    Completed,
    Rejected,
    Cancelled,
    Failed,
    Disconnected,
    Interrupted,
}

#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("History storage failed: {0}")]
    Storage(#[from] std::io::Error),
    #[error("History data is invalid or uses an unsupported version")]
    InvalidData,
    #[error("This activity is unavailable")]
    NotFound,
    #[error("Only unsuccessful outgoing activities can be retried")]
    NotRetryable,
    #[error("A retry for this activity is already running")]
    RetryInProgress,
    #[error("Reconnect to the trusted device before retrying")]
    NotConnected,
    #[error("The source file is missing or has changed; select it again from Send")]
    SourceChanged,
    #[error("History is not running")]
    NotRunning,
    #[error("History synchronization failed")]
    Synchronization,
    #[error("Retry failed: {0}")]
    Retry(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    pub(super) id: String,
    pub(super) device_id: String,
    pub(super) device_name: String,
    pub(super) platform: Platform,
    pub(super) kind: ActivityKind,
    pub(super) direction: ActivityDirection,
    pub(super) status: ActivityStatus,
    pub(super) title: String,
    pub(super) started_at: u64,
    pub(super) ended_at: Option<u64>,
    pub(super) completed_at: Option<u64>,
    pub(super) disconnected_at: Option<u64>,
    pub(super) file_completed_at: Option<u64>,
    pub(super) failure: Option<String>,
    pub(super) url: Option<String>,
    pub(super) path: Option<PathBuf>,
    pub(super) position_millis: u64,
    pub(super) retry_of: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) transfer_id: Option<String>,
    pub(super) revision: u64,
    pub(super) source_size: Option<u64>,
    pub(super) source_modified: Option<u64>,
}

impl Activity {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
    pub fn device_name(&self) -> &str {
        &self.device_name
    }
    pub fn platform(&self) -> Platform {
        self.platform
    }
    pub fn kind(&self) -> ActivityKind {
        self.kind
    }
    pub fn direction(&self) -> ActivityDirection {
        self.direction
    }
    pub fn status(&self) -> ActivityStatus {
        self.status
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn started_at(&self) -> u64 {
        self.started_at
    }
    pub fn ended_at(&self) -> Option<u64> {
        self.ended_at
    }
    pub fn completed_at(&self) -> Option<u64> {
        self.completed_at
    }
    pub fn disconnected_at(&self) -> Option<u64> {
        self.disconnected_at
    }
    pub fn file_completed_at(&self) -> Option<u64> {
        self.file_completed_at
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn position_millis(&self) -> u64 {
        self.position_millis
    }
    pub fn retry_of(&self) -> Option<&str> {
        self.retry_of.as_deref()
    }
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
    pub fn file_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
    pub fn can_retry(&self) -> bool {
        self.direction == ActivityDirection::Outgoing
            && matches!(
                self.status,
                ActivityStatus::Failed
                    | ActivityStatus::Rejected
                    | ActivityStatus::Cancelled
                    | ActivityStatus::Interrupted
            )
            && self.kind != ActivityKind::Session
    }
    pub(super) fn is_finished(&self) -> bool {
        self.status != ActivityStatus::Active
    }
}

#[derive(Debug, Clone)]
pub struct ActivitySnapshot {
    pub(super) entries: Vec<Activity>,
    pub(super) storage_error: Option<String>,
}
impl ActivitySnapshot {
    pub fn entries(&self) -> &[Activity] {
        &self.entries
    }
    pub fn storage_error(&self) -> Option<&str> {
        self.storage_error.as_deref()
    }
}

pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

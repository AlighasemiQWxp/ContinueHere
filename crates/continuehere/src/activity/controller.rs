use super::{
    Activity, ActivityDirection, ActivityError, ActivityKind, ActivitySnapshot, ActivityStatus,
    event::ActivityChangedEvent,
    model::now,
    store::{ActivityStore, MAX_ENTRIES},
};
use crate::{
    handoff::{HandoffChange, HandoffPayload, HandoffState, IncomingHandoffChange},
    models::{DeviceId, Platform},
    pairing::TrustedPeerLookup,
    transfer::{FileTransfer, FileTransferChange, FileTransferDirection, FileTransferState},
    transport::ConnectionChange,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

pub(super) struct ActivityController {
    state: Mutex<State>,
    store: ActivityStore,
    trusted: TrustedPeerLookup,
    changed: ActivityChangedEvent,
}

struct State {
    running: bool,
    entries: Vec<Activity>,
    sessions: HashMap<String, String>,
    revision: u64,
    storage_error: Option<String>,
}

impl ActivityController {
    pub(super) fn new(
        directory: PathBuf,
        trusted: TrustedPeerLookup,
        changed: ActivityChangedEvent,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                running: false,
                entries: Vec::new(),
                sessions: HashMap::new(),
                revision: 0,
                storage_error: None,
            }),
            store: ActivityStore::new(directory),
            trusted,
            changed,
        })
    }

    pub(super) fn start(&self) -> Result<(), ActivityError> {
        let mut state = self.lock()?;
        let mut entries = self.store.load()?;
        recover(&mut entries);
        self.store.save(&entries)?;
        state.revision = entries
            .iter()
            .map(|entry| entry.revision)
            .max()
            .unwrap_or(0);
        state.entries = entries;
        state.running = true;
        Ok(())
    }

    pub(super) fn stop(&self) -> Result<(), ActivityError> {
        let mut state = self.lock()?;
        state.running = false;
        recover(&mut state.entries);
        state.sessions.clear();
        self.store.save(&state.entries)
    }

    pub(super) fn snapshot(&self) -> Result<ActivitySnapshot, ActivityError> {
        let state = self.lock()?;
        if !state.running {
            return Err(ActivityError::NotRunning);
        }
        let mut entries = state.entries.clone();
        entries.sort_by_key(|entry| std::cmp::Reverse((entry.started_at, entry.revision)));
        Ok(ActivitySnapshot {
            entries,
            storage_error: state.storage_error.clone(),
        })
    }

    pub(super) fn find(&self, id: &str) -> Result<Activity, ActivityError> {
        let state = self.lock()?;
        if !state.running {
            return Err(ActivityError::NotRunning);
        }
        state
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .cloned()
            .ok_or(ActivityError::NotFound)
    }

    pub(super) fn remove(&self, id: Option<&str>) -> Result<(), ActivityError> {
        let device = {
            let mut state = self.lock()?;
            if !state.running {
                return Err(ActivityError::NotRunning);
            }
            let mut entries = state.entries.clone();
            let mut device = String::new();
            if let Some(id) = id {
                let entry = entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .ok_or(ActivityError::NotFound)?;
                if !entry.is_finished() {
                    return Err(ActivityError::RetryInProgress);
                }
                device = entry.device_id.clone();
                entries.retain(|entry| entry.id != id);
            } else {
                entries.retain(|entry| !entry.is_finished());
            }
            self.store.save(&entries)?;
            state.entries = entries;
            state.storage_error = None;
            device
        };
        self.changed.publish(super::ActivityChange::new(&device));
        Ok(())
    }

    pub(super) fn link_retry(&self, id: &str, original: &str) {
        self.mutate(|state| {
            let entry = state.entries.iter_mut().find(|entry| entry.id == id)?;
            entry.retry_of = Some(original.to_owned());
            entry.revision = 0;
            Some(entry.device_id.clone())
        });
    }

    pub(super) fn handoff(&self, change: HandoffChange) {
        let (handoff, added, removed) = match change {
            HandoffChange::Added(value) => (value, true, false),
            HandoffChange::Updated(value) => (value, false, false),
            HandoffChange::Removed(value) => (value, false, true),
        };
        let id = format!("handoff.out.{}", handoff.id());
        if !added && !removed && handoff.state() == HandoffState::Sending {
            let transfer_id = handoff.transfer().map(|value| value.id().as_str());
            let Ok(state) = self.lock() else {
                return;
            };
            if state
                .entries
                .iter()
                .any(|entry| entry.id == id && entry.transfer_id.as_deref() == transfer_id)
            {
                return;
            }
        }
        let mut record = self.record(
            id,
            handoff.destination_device_id(),
            ActivityDirection::Outgoing,
        );
        payload(&mut record, handoff.payload());
        record.status = match handoff.state() {
            HandoffState::Sending => ActivityStatus::Active,
            HandoffState::Delivered => ActivityStatus::Delivered,
            HandoffState::Rejected => ActivityStatus::Rejected,
            HandoffState::Cancelled => ActivityStatus::Cancelled,
            HandoffState::Failed => ActivityStatus::Failed,
        };
        record.failure = handoff.failure().map(|failure| format!("{failure:?}"));
        if let Some(transfer) = handoff.transfer() {
            record.transfer_id = Some(transfer.id().as_str().to_owned());
        }
        self.upsert(record, added, removed);
    }

    pub(super) fn incoming(&self, change: IncomingHandoffChange) {
        let IncomingHandoffChange::Added(handoff) = change else {
            return;
        };
        let mut record = self.record(
            format!("handoff.in.{}", handoff.id()),
            handoff.sender_device_id(),
            ActivityDirection::Incoming,
        );
        payload(&mut record, handoff.payload());
        record.status = ActivityStatus::Delivered;
        self.upsert(record, true, false);
    }

    pub(super) fn transfer(&self, change: FileTransferChange) {
        let (transfer, added, removed) = match change {
            FileTransferChange::Added(value) => (value, true, false),
            FileTransferChange::Updated(value) => (value, false, false),
            FileTransferChange::Removed(value) => (value, false, true),
        };
        if !added
            && !removed
            && !matches!(
                transfer.state(),
                FileTransferState::Completed
                    | FileTransferState::Rejected
                    | FileTransferState::Cancelled
                    | FileTransferState::Failed
            )
        {
            let Ok(state) = self.lock() else {
                return;
            };
            let unchanged = state.entries.iter().any(|entry| {
                entry.transfer_id.as_deref() == Some(transfer.id().as_str())
                    && entry.device_id == transfer.peer_device_id().as_str()
                    && (entry.kind == ActivityKind::LocalVideo
                        || entry.path.as_deref()
                            == transfer.source().or_else(|| transfer.destination()))
            });
            if unchanged {
                return;
            }
        }
        let mut record = self.record(
            transfer_key(&transfer),
            transfer.peer_device_id(),
            match transfer.direction() {
                FileTransferDirection::Outgoing => ActivityDirection::Outgoing,
                FileTransferDirection::Incoming => ActivityDirection::Incoming,
            },
        );
        record.kind = if transfer.is_folder() {
            ActivityKind::Folder
        } else {
            ActivityKind::File
        };
        record.title = transfer.file_name().to_owned();
        record.transfer_id = Some(transfer.id().as_str().to_owned());
        record.path = transfer
            .source()
            .or_else(|| transfer.destination())
            .map(PathBuf::from);
        record.status = match transfer.state() {
            FileTransferState::Completed => ActivityStatus::Completed,
            FileTransferState::Rejected => ActivityStatus::Rejected,
            FileTransferState::Cancelled => ActivityStatus::Cancelled,
            FileTransferState::Failed => ActivityStatus::Failed,
            _ => ActivityStatus::Active,
        };
        record.failure = transfer.failure().map(|failure| format!("{failure:?}"));
        self.upsert(record, added, removed);
    }

    pub(super) fn connection(&self, change: ConnectionChange) {
        let (connection, connected) = match change {
            ConnectionChange::Added(value) => (value, true),
            ConnectionChange::Removed(value) => (value, false),
        };
        let peer = connection.device_id().as_str().to_owned();
        let mut record = self.record(
            uuid::Uuid::new_v4().to_string(),
            connection.device_id(),
            ActivityDirection::Connection,
        );
        record.device_name = connection.display_name().to_owned();
        record.platform = connection.platform();
        record.kind = ActivityKind::Session;
        self.mutate(|state| {
            if connected {
                if state.sessions.contains_key(&peer) {
                    return None;
                }
                state.sessions.insert(peer.clone(), record.id.clone());
                state.entries.push(record);
            } else {
                let id = state.sessions.remove(&peer)?;
                let entry = state.entries.iter_mut().find(|entry| entry.id == id)?;
                entry.status = ActivityStatus::Disconnected;
                let time = now();
                entry.disconnected_at = Some(time);
                entry.ended_at = Some(time);
                entry.revision = 0;
            }
            Some(peer)
        });
    }

    fn upsert(&self, mut record: Activity, added: bool, removed: bool) {
        self.mutate(|state| {
            let time = now();
            if record.kind == ActivityKind::File
                && let Some(parent) = state.entries.iter_mut().find(|entry| {
                    entry.kind == ActivityKind::LocalVideo
                        && entry.transfer_id == record.transfer_id
                        && entry.device_id == record.device_id
                        && entry.direction == record.direction
                })
            {
                if record.status == ActivityStatus::Completed && parent.file_completed_at.is_none()
                {
                    parent.file_completed_at = Some(time);
                    parent.revision = 0;
                    return Some(parent.device_id.clone());
                }
                return None;
            }
            let index = state.entries.iter().position(|entry| entry.id == record.id);
            if index.is_none() && !added {
                return None;
            }
            if let Some(index) = index {
                let old = &state.entries[index];
                if old.is_finished() {
                    return None;
                }
                record.started_at = old.started_at;
                record.session_id = old.session_id.clone();
                record.retry_of = old.retry_of.clone();
                record.source_size = old.source_size;
                record.source_modified = old.source_modified;
                record.file_completed_at = old.file_completed_at;
                if record.path.is_none() {
                    record.path = old.path.clone();
                }
                if removed && record.status == ActivityStatus::Active {
                    record.status = ActivityStatus::Cancelled;
                }
                if record.status == old.status
                    && record.transfer_id == old.transfer_id
                    && record.path == old.path
                    && record.failure == old.failure
                    && !removed
                {
                    return None;
                }
            } else {
                record.session_id = state.sessions.get(&record.device_id).cloned();
                if let Some(session) = state
                    .entries
                    .iter()
                    .find(|entry| Some(&entry.id) == record.session_id.as_ref())
                {
                    record.device_name = session.device_name.clone();
                    record.platform = session.platform;
                }
                if record.direction == ActivityDirection::Outgoing {
                    capture_source(&mut record);
                }
            }
            if record.status != ActivityStatus::Active {
                record.ended_at = Some(time);
                if matches!(
                    record.status,
                    ActivityStatus::Completed | ActivityStatus::Delivered
                ) {
                    record.completed_at = Some(time);
                }
                if matches!(record.kind, ActivityKind::File | ActivityKind::Folder)
                    && record.status == ActivityStatus::Completed
                {
                    record.file_completed_at = Some(time);
                }
            }
            if record.kind == ActivityKind::LocalVideo && record.transfer_id.is_some() {
                if let Some(child) = state.entries.iter().find(|entry| {
                    entry.kind == ActivityKind::File
                        && entry.transfer_id == record.transfer_id
                        && entry.device_id == record.device_id
                        && entry.direction == record.direction
                }) {
                    record.file_completed_at = child.file_completed_at;
                    record.started_at = record.started_at.min(child.started_at);
                    if record.session_id.is_none() {
                        record.session_id = child.session_id.clone();
                    }
                }
                state.entries.retain(|entry| {
                    !(entry.kind == ActivityKind::File
                        && entry.transfer_id == record.transfer_id
                        && entry.device_id == record.device_id
                        && entry.direction == record.direction)
                });
            }
            let peer = record.device_id.clone();
            if let Some(entry) = state.entries.iter_mut().find(|entry| entry.id == record.id) {
                *entry = record;
            } else {
                state.entries.push(record);
            }
            Some(peer)
        });
    }

    fn record(&self, id: String, device_id: &DeviceId, direction: ActivityDirection) -> Activity {
        let peer = self.trusted.get(device_id).ok().flatten();
        Activity {
            id,
            device_id: device_id.as_str().to_owned(),
            device_name: peer
                .as_ref()
                .map(|peer| peer.display_name().to_owned())
                .unwrap_or_else(|| device_id.as_str().to_owned()),
            platform: peer
                .as_ref()
                .map(|peer| peer.platform())
                .unwrap_or(Platform::Unknown),
            kind: ActivityKind::Url,
            direction,
            status: ActivityStatus::Active,
            title: String::new(),
            started_at: now(),
            ended_at: None,
            completed_at: None,
            disconnected_at: None,
            file_completed_at: None,
            failure: None,
            url: None,
            path: None,
            position_millis: 0,
            retry_of: None,
            session_id: None,
            transfer_id: None,
            revision: 0,
            source_size: None,
            source_modified: None,
        }
    }

    fn mutate(&self, action: impl FnOnce(&mut State) -> Option<String>) {
        let changed = {
            let Ok(mut state) = self.lock() else {
                return;
            };
            if !state.running {
                return;
            }
            let Some(peer) = action(&mut state) else {
                return;
            };
            state.revision = state.revision.saturating_add(1);
            let revision = state.revision;
            for entry in &mut state.entries {
                if entry.revision == 0 {
                    entry.revision = revision;
                }
            }
            prune(&mut state.entries);
            state.storage_error = self
                .store
                .save(&state.entries)
                .err()
                .map(|error| error.to_string());
            peer
        };
        self.changed.publish(super::ActivityChange::new(&changed));
    }

    fn lock(&self) -> Result<MutexGuard<'_, State>, ActivityError> {
        self.state
            .lock()
            .map_err(|_| ActivityError::Synchronization)
    }
}

pub(super) fn recover(entries: &mut [Activity]) {
    for entry in entries {
        if !entry.is_finished() {
            entry.status = ActivityStatus::Interrupted;
        }
    }
}

pub(super) fn prune(entries: &mut Vec<Activity>) {
    while entries.len() > MAX_ENTRIES {
        let Some(index) = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.is_finished())
            .min_by_key(|(_, entry)| (entry.started_at, entry.revision))
            .map(|(index, _)| index)
        else {
            break;
        };
        entries.remove(index);
    }
}

pub(super) fn transfer_key(transfer: &FileTransfer) -> String {
    match transfer.direction() {
        FileTransferDirection::Outgoing => format!("file.out.{}", transfer.id()),
        FileTransferDirection::Incoming => format!("file.in.{}", transfer.id()),
    }
}

fn payload(record: &mut Activity, payload: &HandoffPayload) {
    match payload {
        HandoffPayload::Url(url) => {
            record.kind = ActivityKind::Url;
            record.url = Some(url.url().to_owned());
            record.title = url.url().to_owned();
        }
        HandoffPayload::YouTube(video) => {
            record.kind = ActivityKind::YouTube;
            record.url = Some(video.resume_url());
            record.title = video.resume_url();
            record.position_millis = video.playback_position().as_millis();
        }
        HandoffPayload::LocalVideo(video) => {
            record.kind = ActivityKind::LocalVideo;
            record.path = Some(video.file_path().to_owned());
            record.title = video
                .file_path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            record.position_millis = video.playback_position().as_millis();
            record.transfer_id = video.transfer_id().map(|id| id.as_str().to_owned());
        }
    }
}

fn capture_source(record: &mut Activity) {
    if let Some(path) = &record.path
        && let Ok(metadata) = path.symlink_metadata()
    {
        record.source_size = Some(metadata.len());
        record.source_modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|time| time.as_millis() as u64);
    }
}

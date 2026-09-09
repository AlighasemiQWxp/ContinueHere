use super::{
    Activity, ActivityError, ActivityKind,
    controller::{ActivityController, transfer_key},
};
use crate::{
    handoff::{HandoffCapability, HandoffHandle, HandoffState},
    models::DeviceId,
    pairing::TrustedPeerLookup,
    transfer::{FileTransferCapability, FileTransferHandle, FileTransferState},
    transport::ConnectionCapability,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

pub(super) struct RetryController {
    handles: Mutex<HashMap<String, Option<RetryHandle>>>,
    handoff: HandoffCapability,
    transfer: FileTransferCapability,
    connections: ConnectionCapability,
    trusted: TrustedPeerLookup,
    wake: Mutex<Option<mpsc::SyncSender<()>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

enum RetryHandle {
    Handoff(HandoffHandle),
    Transfer(FileTransferHandle),
}
impl RetryHandle {
    fn activity_id(&self) -> Option<String> {
        match self {
            Self::Handoff(handle) => handle
                .handoff()
                .map(|value| format!("handoff.out.{}", value.id())),
            Self::Transfer(handle) => handle.transfer().map(|value| transfer_key(&value)),
        }
    }
    fn finished(&self) -> bool {
        match self {
            Self::Handoff(handle) => handle
                .handoff()
                .is_none_or(|value| value.state() != HandoffState::Sending),
            Self::Transfer(handle) => handle.transfer().is_none_or(|value| {
                matches!(
                    value.state(),
                    FileTransferState::Completed
                        | FileTransferState::Rejected
                        | FileTransferState::Cancelled
                        | FileTransferState::Failed
                )
            }),
        }
    }
    fn release(self) -> Result<(), ActivityError> {
        match self {
            Self::Handoff(handle) => handle
                .release()
                .map(|_| ())
                .map_err(|error| ActivityError::Retry(error.to_string())),
            Self::Transfer(handle) => handle
                .release()
                .map(|_| ())
                .map_err(|error| ActivityError::Retry(error.to_string())),
        }
    }
}

impl RetryController {
    pub(super) fn new(
        handoff: HandoffCapability,
        transfer: FileTransferCapability,
        connections: ConnectionCapability,
        trusted: TrustedPeerLookup,
    ) -> Arc<Self> {
        Arc::new(Self {
            handles: Mutex::new(HashMap::new()),
            handoff,
            transfer,
            connections,
            trusted,
            wake: Mutex::new(None),
            worker: Mutex::new(None),
        })
    }

    pub(super) fn start(self: &Arc<Self>) -> Result<(), ActivityError> {
        let (wake, receiver) = mpsc::sync_channel(1);
        let weak = Arc::downgrade(self);
        let worker = thread::Builder::new()
            .name("continuehere-history-retry".to_owned())
            .spawn(move || {
                while receiver.recv().is_ok() {
                    let Some(controller) = weak.upgrade() else {
                        break;
                    };
                    controller.release_finished();
                }
            })?;
        *self
            .wake
            .lock()
            .map_err(|_| ActivityError::Synchronization)? = Some(wake);
        *self
            .worker
            .lock()
            .map_err(|_| ActivityError::Synchronization)? = Some(worker);
        Ok(())
    }

    pub(super) fn changed(&self) {
        if let Ok(wake) = self.wake.lock()
            && let Some(wake) = wake.as_ref()
        {
            let _ = wake.try_send(());
        }
    }

    pub(super) fn retry(
        &self,
        activity: Activity,
        history: &ActivityController,
    ) -> Result<(), ActivityError> {
        if !activity.can_retry() {
            return Err(ActivityError::NotRetryable);
        }
        let peer = DeviceId::new(&activity.device_id).map_err(|_| ActivityError::InvalidData)?;
        if !self.connections.is_connected(&peer)
            || self
                .trusted
                .get(&peer)
                .map_err(|error| ActivityError::Retry(error.to_string()))?
                .is_none()
        {
            return Err(ActivityError::NotConnected);
        }
        validate_source(&activity)?;
        {
            let mut handles = self
                .handles
                .lock()
                .map_err(|_| ActivityError::Synchronization)?;
            if handles.contains_key(&activity.id) {
                return Err(ActivityError::RetryInProgress);
            }
            handles.insert(activity.id.clone(), None);
        }
        let result = self.begin(&activity, peer);
        match result {
            Ok(handle) => {
                if let Some(id) = handle.activity_id() {
                    history.link_retry(&id, &activity.id);
                }
                self.handles
                    .lock()
                    .map_err(|_| ActivityError::Synchronization)?
                    .insert(activity.id, Some(handle));
                self.changed();
                Ok(())
            }
            Err(error) => {
                self.handles
                    .lock()
                    .map_err(|_| ActivityError::Synchronization)?
                    .remove(&activity.id);
                Err(error)
            }
        }
    }

    fn begin(&self, activity: &Activity, peer: DeviceId) -> Result<RetryHandle, ActivityError> {
        let identifier = format!("history.retry.{}", uuid::Uuid::new_v4());
        if matches!(activity.kind, ActivityKind::File | ActivityKind::Folder) {
            let handle = self
                .transfer
                .get_handle(&identifier)
                .map_err(|error| ActivityError::Retry(error.to_string()))?;
            let source = activity.path.as_ref().ok_or(ActivityError::SourceChanged)?;
            let configured = if activity.kind == ActivityKind::Folder {
                handle.configure_folder(peer, source)
            } else {
                handle.configure(peer, source)
            };
            if let Err(error) = configured.and_then(|()| handle.use_handle()) {
                handle
                    .release()
                    .map_err(|error| ActivityError::Retry(error.to_string()))?;
                return Err(ActivityError::Retry(error.to_string()));
            }
            return Ok(RetryHandle::Transfer(handle));
        }
        let handle = self
            .handoff
            .get_handle(&identifier)
            .map_err(|error| ActivityError::Retry(error.to_string()))?;
        let position = Duration::from_millis(activity.position_millis);
        let configured = match activity.kind {
            ActivityKind::Url => {
                handle.configure_url(peer, activity.url.as_deref().unwrap_or_default())
            }
            ActivityKind::YouTube => handle.configure_youtube(
                peer,
                activity.url.as_deref().unwrap_or_default(),
                position,
            ),
            ActivityKind::LocalVideo => handle.configure_local_video(
                peer,
                activity.path.clone().unwrap_or_default(),
                position,
            ),
            _ => {
                handle
                    .release()
                    .map_err(|error| ActivityError::Retry(error.to_string()))?;
                return Err(ActivityError::NotRetryable);
            }
        };
        if let Err(error) = configured.and_then(|()| handle.use_handle()) {
            handle
                .release()
                .map_err(|error| ActivityError::Retry(error.to_string()))?;
            return Err(ActivityError::Retry(error.to_string()));
        }
        Ok(RetryHandle::Handoff(handle))
    }

    fn release_finished(&self) {
        let finished = {
            let Ok(mut handles) = self.handles.lock() else {
                return;
            };
            let finished = handles
                .iter()
                .filter(|(_, handle)| handle.as_ref().is_some_and(RetryHandle::finished))
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            finished
                .into_iter()
                .filter_map(|id| handles.remove(&id).flatten())
                .collect::<Vec<_>>()
        };
        for handle in finished {
            let _ = handle.release();
        }
    }

    pub(super) fn stop(&self) -> Result<(), ActivityError> {
        self.wake
            .lock()
            .map_err(|_| ActivityError::Synchronization)?
            .take();
        if let Some(worker) = self
            .worker
            .lock()
            .map_err(|_| ActivityError::Synchronization)?
            .take()
        {
            worker.join().map_err(|_| ActivityError::Synchronization)?;
        }
        let handles = std::mem::take(
            &mut *self
                .handles
                .lock()
                .map_err(|_| ActivityError::Synchronization)?,
        );
        let mut failure = None;
        for handle in handles.into_values().flatten() {
            if let Err(error) = handle.release() {
                failure = Some(error);
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

pub(super) fn validate_source(activity: &Activity) -> Result<(), ActivityError> {
    if activity.kind == ActivityKind::Folder {
        let path = activity.path.as_ref().ok_or(ActivityError::SourceChanged)?;
        let metadata = path
            .symlink_metadata()
            .map_err(|_| ActivityError::SourceChanged)?;
        if !path.is_absolute()
            || !metadata.file_type().is_dir()
            || metadata.file_type().is_symlink()
        {
            return Err(ActivityError::SourceChanged);
        }
        return Ok(());
    }
    if !matches!(activity.kind, ActivityKind::File | ActivityKind::LocalVideo) {
        return Ok(());
    }
    let path = activity.path.as_ref().ok_or(ActivityError::SourceChanged)?;
    let metadata = path
        .symlink_metadata()
        .map_err(|_| ActivityError::SourceChanged)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|time| time.as_millis() as u64);
    if !path.is_absolute()
        || !metadata.file_type().is_file()
        || Some(metadata.len()) != activity.source_size
        || modified != activity.source_modified
    {
        return Err(ActivityError::SourceChanged);
    }
    Ok(())
}

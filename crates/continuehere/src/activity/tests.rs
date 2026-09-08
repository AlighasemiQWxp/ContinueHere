use super::{
    Activity, ActivityChangedDelegate, ActivityDirection, ActivityKind, ActivityStatus,
    controller::{ActivityController, prune},
    event::ActivityChangedEvent,
    retry::validate_source,
    store::{self, ActivityStore},
};
use crate::{
    handoff::{
        Handoff, HandoffChange, HandoffConfig, HandoffId, HandoffPayload, IncomingHandoff,
        IncomingHandoffChange, LocalVideoHandoff, UrlHandoff,
    },
    models::{DeviceId, Platform, ProtocolVersion},
    pairing::TrustedDeviceRegistry,
    transfer::{FileTransfer, FileTransferChange, FileTransferId, FileTransferState},
    transport::{AuthenticatedConnection, ConnectionChange, ConnectionDirection},
};
use std::{fs, sync::Arc, time::Duration};
use tempfile::tempdir;

fn controller(directory: &std::path::Path) -> Arc<ActivityController> {
    let trusted = TrustedDeviceRegistry::new(directory.to_owned());
    trusted.load().expect("trust should load");
    let controller = ActivityController::new(
        directory.to_owned(),
        trusted.lookup(),
        ActivityChangedEvent::default(),
    );
    controller.start().expect("history should start");
    controller
}

fn peer() -> DeviceId {
    DeviceId::new("test-peer").expect("peer should be valid")
}
fn connection() -> AuthenticatedConnection {
    AuthenticatedConnection::new(
        peer(),
        "Other laptop".to_owned(),
        Platform::Windows,
        ProtocolVersion::CURRENT,
        Vec::new(),
        ConnectionDirection::Incoming,
    )
}
fn incoming_file(path: &std::path::Path) -> FileTransfer {
    let mut file = FileTransfer::incoming(FileTransferId::new(), peer(), "clip.mp4".to_owned(), 4);
    file.set_destination(path.to_owned());
    file
}
fn url() -> Handoff {
    Handoff::new(
        HandoffId::new(),
        HandoffConfig::url(
            peer(),
            UrlHandoff::new("https://example.com").expect("URL should be valid"),
        ),
    )
}

#[test]
fn completion_and_disconnect_are_independent_and_survive_restart() {
    let directory = tempdir().expect("directory should exist");
    let history = controller(directory.path());
    let path = directory.path().join("clip.mp4");
    fs::write(&path, b"test").expect("file should exist");
    history.connection(ConnectionChange::Added(connection()));
    let mut file = incoming_file(&path);
    history.transfer(FileTransferChange::Added(file.clone()));
    file.set_state(FileTransferState::Completed);
    history.transfer(FileTransferChange::Updated(file.clone()));
    let snapshot = history.snapshot().expect("snapshot should be available");
    let before = snapshot
        .entries()
        .iter()
        .find(|entry| entry.kind() == ActivityKind::File)
        .expect("file should be recorded")
        .clone();
    history.transfer(FileTransferChange::Removed(file));
    history.connection(ConnectionChange::Removed(connection()));
    history.stop().expect("history should stop");
    let restored = controller(directory.path());
    let snapshot = restored.snapshot().expect("snapshot should be available");
    assert_eq!(snapshot.entries().len(), 2);
    let file = snapshot
        .entries()
        .iter()
        .find(|entry| entry.kind() == ActivityKind::File)
        .expect("file should remain");
    assert_eq!(file.completed_at(), before.completed_at());
    assert!(file.file_completed_at().is_some());
    let session = snapshot
        .entries()
        .iter()
        .find(|entry| entry.kind() == ActivityKind::Session)
        .expect("session should remain");
    assert_eq!(file.session_id(), Some(session.id()));
    assert_eq!(session.ended_at(), session.disconnected_at());
    assert_eq!(session.status(), ActivityStatus::Disconnected);
    assert!(session.disconnected_at().is_some());
    restored.remove(None).expect("history should clear");
    assert!(path.exists());
}

#[test]
fn restart_marks_unfinished_work_interrupted_without_inventing_end_time() {
    let directory = tempdir().expect("directory should exist");
    let history = controller(directory.path());
    history.handoff(HandoffChange::Added(url()));
    let restored = controller(directory.path());
    let snapshot = restored.snapshot().expect("snapshot should be available");
    let activity = &snapshot.entries()[0];
    assert_eq!(activity.status(), ActivityStatus::Interrupted);
    assert_eq!(activity.ended_at(), None);
    assert!(activity.can_retry());
}

#[test]
fn incoming_video_groups_its_file_and_preserves_file_completion() {
    let directory = tempdir().expect("directory should exist");
    let history = controller(directory.path());
    let path = directory.path().join("clip.mp4");
    fs::write(&path, b"test").expect("file should exist");
    let mut file = incoming_file(&path);
    history.transfer(FileTransferChange::Added(file.clone()));
    file.set_state(FileTransferState::Completed);
    history.transfer(FileTransferChange::Updated(file.clone()));
    let completed =
        history.snapshot().expect("snapshot should exist").entries()[0].file_completed_at();
    let video = LocalVideoHandoff::received(
        &file,
        crate::handoff::PlaybackPosition::new(Duration::from_secs(42))
            .expect("position should be valid"),
    )
    .expect("video should be valid");
    let handoff = IncomingHandoff::new(HandoffId::new(), peer(), HandoffPayload::LocalVideo(video));
    history.incoming(IncomingHandoffChange::Added(handoff));
    history.transfer(FileTransferChange::Removed(file));
    let snapshot = history.snapshot().expect("snapshot should exist");
    assert_eq!(snapshot.entries().len(), 1);
    let video = &snapshot.entries()[0];
    assert_eq!(video.kind(), ActivityKind::LocalVideo);
    assert_eq!(video.file_completed_at(), completed);
    assert_eq!(video.position_millis(), 42_000);
    assert!(!video.can_retry());
}

#[test]
fn duplicate_and_progress_events_do_not_rewrite_history() {
    let directory = tempdir().expect("directory should exist");
    let history = controller(directory.path());
    let mut file = incoming_file(&directory.path().join("clip.mp4"));
    history.transfer(FileTransferChange::Added(file.clone()));
    let revision = history.snapshot().expect("snapshot should exist").entries()[0].revision();
    for progress in 1..=4 {
        file.set_progress(progress);
        history.transfer(FileTransferChange::Updated(file.clone()));
    }
    assert_eq!(
        history.snapshot().expect("snapshot should exist").entries()[0].revision(),
        revision
    );
}

#[test]
fn clear_preserves_running_work_and_retry_links_keep_original_results() {
    let directory = tempdir().expect("directory should exist");
    let history = controller(directory.path());
    let mut first = url();
    history.handoff(HandoffChange::Added(first.clone()));
    first.cancel();
    history.handoff(HandoffChange::Updated(first.clone()));
    let original = format!("handoff.out.{}", first.id());
    let next = url();
    let next_id = format!("handoff.out.{}", next.id());
    history.handoff(HandoffChange::Added(next));
    history.link_retry(&next_id, &original);
    assert_eq!(
        history
            .find(&original)
            .expect("original should exist")
            .status(),
        ActivityStatus::Cancelled
    );
    assert_eq!(
        history
            .find(&next_id)
            .expect("retry should exist")
            .retry_of(),
        Some(original.as_str())
    );
    history.remove(None).expect("history should clear");
    let snapshot = history.snapshot().expect("snapshot should exist");
    assert_eq!(snapshot.entries().len(), 1);
    assert_eq!(snapshot.entries()[0].id(), next_id);
}

#[test]
fn storage_rejects_corruption_and_reports_runtime_write_failure() {
    let directory = tempdir().expect("directory should exist");
    let history = controller(directory.path());
    history.handoff(HandoffChange::Added(url()));
    let snapshot = history.snapshot().expect("snapshot should exist");
    let bytes = store::encode(snapshot.entries()).expect("history should encode");
    assert_eq!(
        store::decode(&bytes).expect("history should decode"),
        snapshot.entries()
    );
    for length in [0, 7, 8, bytes.len() - 1] {
        assert!(store::decode(&bytes[..length]).is_err());
    }
    let mut invalid = bytes.clone();
    invalid.push(0);
    assert!(store::decode(&invalid).is_err());
    fs::remove_file(directory.path().join("history.bin")).expect("test history should be removed");
    fs::create_dir(directory.path().join("history.bin")).expect("test should block writes");
    history.handoff(HandoffChange::Added(url()));
    assert!(
        history
            .snapshot()
            .expect("runtime history should remain readable")
            .storage_error()
            .is_some()
    );
    assert!(history.stop().is_err());
}

#[test]
fn events_allow_reading_the_committed_snapshot_and_unsubscribe_on_drop() {
    let directory = tempdir().expect("directory should exist");
    let trusted = TrustedDeviceRegistry::new(directory.path().to_owned());
    let event = ActivityChangedEvent::default();
    let history =
        ActivityController::new(directory.path().to_owned(), trusted.lookup(), event.clone());
    history.start().expect("history should start");
    let weak = Arc::downgrade(&history);
    let (send, receive) = std::sync::mpsc::channel();
    let subscription = event.subscribe(ActivityChangedDelegate::new(move |change| {
        let snapshot = weak
            .upgrade()
            .expect("history should exist")
            .snapshot()
            .expect("snapshot should be readable");
        send.send((change.device_id().cloned(), snapshot.entries().len()))
            .expect("receiver should exist");
    }));
    history.handoff(HandoffChange::Added(url()));
    assert_eq!(
        receive.try_recv().expect("event should arrive"),
        (Some(peer()), 1)
    );
    drop(subscription);
    history.handoff(HandoffChange::Added(url()));
    assert!(receive.try_recv().is_err());
}

#[test]
fn retention_keeps_active_entries_and_source_validation_rejects_changed_files() {
    let directory = tempdir().expect("directory should exist");
    let history = controller(directory.path());
    let path = directory.path().join("clip.mp4");
    fs::write(&path, b"test").expect("file should exist");
    let video =
        LocalVideoHandoff::new(path.clone(), Duration::ZERO).expect("video should be valid");
    let handoff = Handoff::new(HandoffId::new(), HandoffConfig::local_video(peer(), video));
    history.handoff(HandoffChange::Added(handoff));
    let mut entry = history.snapshot().expect("snapshot should exist").entries()[0].clone();
    assert!(validate_source(&entry).is_ok());
    fs::write(&path, b"different file").expect("file should change");
    assert!(validate_source(&entry).is_err());
    entry.status = ActivityStatus::Failed;
    assert!(entry.can_retry());
    entry.direction = ActivityDirection::Incoming;
    assert!(!entry.can_retry());
    let mut entries = (0..1101)
        .map(|index| {
            let mut entry = entry.clone();
            entry.id = index.to_string();
            entry.started_at = index;
            entry
        })
        .collect::<Vec<Activity>>();
    entries[0].status = ActivityStatus::Active;
    prune(&mut entries);
    assert_eq!(entries.len(), 1100);
    assert!(entries.iter().any(|entry| entry.id == "0"));
    assert!(!entries.iter().any(|entry| entry.id == "1"));
    let store = ActivityStore::new(directory.path().to_owned());
    store.save(&entries).expect("bounded history should save");
}

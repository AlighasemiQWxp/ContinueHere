use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use tempfile::{TempDir, tempdir};

use crate::{
    core::module::Module,
    discovery::DiscoveryEndpoint,
    managers::DeviceManager,
    models::DeviceId,
    pairing::{TrustedDevice, TrustedDeviceRegistry},
    security::{CredentialStore, SecurityError, SecurityManager},
    settings::SettingsManager,
    transfer::{FileTransfer, FileTransferManager, FileTransferState},
    transport::{
        HandoffDisposition, HandoffRejection, HandoffTransportPayload, InboundHandoff,
        InboundTransfer, TransferDisposition, TransferRejection, TransferTransportMessage,
        TransportManager,
    },
};

use super::{
    HandoffError, HandoffFailure, HandoffHandle, HandoffManager, HandoffPayload, HandoffState,
    IncomingHandoffChange, IncomingHandoffChangedDelegate, LocalVideoHandoff,
};

#[derive(Default)]
struct MemoryCredentials {
    secrets: Mutex<HashMap<String, Vec<u8>>>,
}

impl CredentialStore for MemoryCredentials {
    fn load(&self, device_id: &DeviceId) -> Result<Option<Vec<u8>>, SecurityError> {
        Ok(self
            .secrets
            .lock()
            .unwrap()
            .get(device_id.as_str())
            .cloned())
    }

    fn save(&self, device_id: &DeviceId, secret: &[u8]) -> Result<(), SecurityError> {
        self.secrets
            .lock()
            .unwrap()
            .insert(device_id.as_str().to_owned(), secret.to_vec());
        Ok(())
    }
}

struct Peer {
    directory: TempDir,
    devices: DeviceManager,
    security: SecurityManager,
    trusted: TrustedDeviceRegistry,
    transport: TransportManager,
    transfers: FileTransferManager,
    handoff: HandoffManager,
}

impl Peer {
    async fn new() -> Self {
        let directory = tempdir().unwrap();
        let mut devices = DeviceManager::new(directory.path().to_path_buf());
        let mut security = SecurityManager::with_store(Arc::new(MemoryCredentials::default()));
        devices.start().await.unwrap();
        security.start().await.unwrap();
        let trusted = TrustedDeviceRegistry::new(directory.path().to_path_buf());
        trusted.load().unwrap();
        let settings = SettingsManager::new(directory.path().to_path_buf()).unwrap();
        settings
            .directories()
            .set_default_transfer_directory(directory.path())
            .unwrap();
        let mut transport = TransportManager::new(
            devices.capability(),
            security.capability(),
            trusted.lookup(),
        );
        let mut transfers = FileTransferManager::new(
            transport.transfer_capability(),
            settings.directories().shared(),
        );
        let mut handoff =
            HandoffManager::new(transport.handoff_capability(), transfers.capability());
        transport.start().await.unwrap();
        transfers.start().await.unwrap();
        handoff.start().await.unwrap();
        Self {
            directory,
            devices,
            security,
            trusted,
            transport,
            transfers,
            handoff,
        }
    }

    fn id(&self) -> DeviceId {
        self.devices.identity().id().clone()
    }

    fn trust(&self, peer: &Peer) {
        let identity = peer.devices.identity();
        let fingerprint = peer
            .security
            .capability()
            .identity(identity.id())
            .unwrap()
            .fingerprint();
        self.trusted
            .persist(TrustedDevice::new(
                identity.id().clone(),
                fingerprint,
                identity.display_name().to_owned(),
                identity.platform(),
            ))
            .unwrap();
    }

    async fn connect(&self, peer: &Peer) {
        self.trust(peer);
        peer.trust(self);
        let endpoint = DiscoveryEndpoint::new(
            "127.0.0.1",
            peer.transport.listening_endpoint().unwrap().port(),
        )
        .unwrap();
        self.transport.connect(&peer.id(), &endpoint).await.unwrap();
    }

    fn source(&self, name: &str) -> PathBuf {
        let path = self.directory.path().join(name);
        std::fs::write(&path, vec![31; 96 * 1024 + 7]).unwrap();
        path
    }

    fn send(&self, peer: &Peer, name: &str) -> HandoffHandle {
        let handle = self.handoff.get_handle(name).unwrap();
        handle
            .configure_local_video(peer.id(), self.source(name), Duration::from_millis(750_123))
            .unwrap();
        handle.use_handle().unwrap();
        handle
    }

    async fn offer(&self) -> FileTransfer {
        wait_until(|| {
            self.transfers
                .transfers()
                .iter()
                .any(|transfer| transfer.state() == FileTransferState::Offered)
        })
        .await;
        self.transfers
            .transfers()
            .into_iter()
            .find(|transfer| transfer.state() == FileTransferState::Offered)
            .unwrap()
    }

    async fn stop(&mut self) {
        self.handoff.stop().await.unwrap();
        self.transfers.stop().await.unwrap();
        self.transport.stop().await.unwrap();
        self.security.stop().await.unwrap();
        self.devices.stop().await.unwrap();
    }
}

async fn wait_until(condition: impl Fn() -> bool) {
    for _ in 0..500 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(condition(), "condition should become true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn video_requires_acceptance_and_delivers_verified_path_and_position_once() {
    let mut sender = Peer::new().await;
    let mut receiver = Peer::new().await;
    sender.connect(&receiver).await;
    let (events, received) = mpsc::channel();
    let _subscription = receiver
        .handoff
        .on_incoming_changed(IncomingHandoffChangedDelegate::new(move |event| {
            events.send(event).unwrap();
        }));
    let handle = sender.send(&receiver, "lesson.mp4");
    let offer = receiver.offer().await;
    assert!(receiver.handoff.incoming().is_empty());
    assert!(received.try_recv().is_err());
    let premature = InboundHandoff::new(
        [71; 16],
        sender.id(),
        HandoffTransportPayload::LocalVideo {
            transfer_id: offer.id().bytes(),
            playback_position_millis: 750_123,
        },
    );
    assert_eq!(
        receiver
            .transport
            .handoff_capability()
            .receive(premature)
            .await,
        HandoffDisposition::Rejected(HandoffRejection::Invalid)
    );
    let destination = tempdir().unwrap();
    receiver
        .transfers
        .accept_incoming(offer.id(), Some(destination.path()))
        .unwrap();
    wait_until(|| {
        handle
            .handoff()
            .is_some_and(|handoff| handoff.state() == HandoffState::Delivered)
    })
    .await;
    let incoming = match received.recv_timeout(Duration::from_secs(1)).unwrap() {
        IncomingHandoffChange::Added(incoming) => incoming,
        _ => panic!("expected incoming handoff"),
    };
    let video = match incoming.payload() {
        HandoffPayload::LocalVideo(video) => video,
        _ => panic!("expected local video"),
    };
    assert_eq!(video.file_path(), destination.path().join("lesson.mp4"));
    assert_eq!(video.transfer_id(), Some(offer.id()));
    assert_eq!(video.playback_position().as_millis(), 750_123);
    assert_eq!(
        std::fs::read(video.file_path()).unwrap(),
        std::fs::read(sender.directory.path().join("lesson.mp4")).unwrap()
    );
    assert_eq!(
        handle.handoff().unwrap().transfer().unwrap().state(),
        FileTransferState::Completed
    );
    assert!(!receiver.directory.path().join("lesson.mp4").exists());

    let wrong_peer = InboundHandoff::new(
        [72; 16],
        DeviceId::new("different-peer").unwrap(),
        HandoffTransportPayload::LocalVideo {
            transfer_id: offer.id().bytes(),
            playback_position_millis: 750_123,
        },
    );
    assert_eq!(
        receiver
            .transport
            .handoff_capability()
            .receive(wrong_peer)
            .await,
        HandoffDisposition::Rejected(HandoffRejection::Invalid)
    );
    receiver.transfers.remove(offer.id()).unwrap();
    let duplicate = || {
        InboundHandoff::new(
            incoming.id().bytes(),
            sender.id(),
            HandoffTransportPayload::LocalVideo {
                transfer_id: offer.id().bytes(),
                playback_position_millis: 750_123,
            },
        )
    };
    assert_eq!(
        receiver
            .transport
            .handoff_capability()
            .receive(duplicate())
            .await,
        HandoffDisposition::Accepted
    );
    assert!(received.try_recv().is_err());
    let altered = InboundHandoff::new(
        incoming.id().bytes(),
        sender.id(),
        HandoffTransportPayload::LocalVideo {
            transfer_id: offer.id().bytes(),
            playback_position_millis: 1,
        },
    );
    assert_eq!(
        receiver
            .transport
            .handoff_capability()
            .receive(altered)
            .await,
        HandoffDisposition::Rejected(HandoffRejection::Invalid)
    );
    assert_eq!(receiver.handoff.incoming().len(), 1);
    handle.release().unwrap();
    sender.stop().await;
    receiver.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejection_and_cancellation_never_publish_a_video_or_close_the_connection() {
    let mut sender = Peer::new().await;
    let mut receiver = Peer::new().await;
    sender.connect(&receiver).await;
    let declined = sender.send(&receiver, "declined.mp4");
    let offer = receiver.offer().await;
    receiver.transfers.reject_incoming(offer.id()).unwrap();
    wait_until(|| {
        declined
            .handoff()
            .is_some_and(|handoff| handoff.state() == HandoffState::Rejected)
    })
    .await;
    declined.release().unwrap();
    let cancelled = sender.send(&receiver, "cancelled.mp4");
    let offer = receiver.offer().await;
    let unrelated = sender.transfers.get_handle("unrelated-file").unwrap();
    unrelated
        .configure(receiver.id(), sender.source("unrelated.bin"))
        .unwrap();
    unrelated.use_handle().unwrap();
    wait_until(|| {
        receiver
            .transfers
            .transfers()
            .iter()
            .filter(|transfer| transfer.state() == FileTransferState::Offered)
            .count()
            == 2
    })
    .await;
    cancelled.release().unwrap();
    wait_until(|| {
        receiver.transfers.transfers().iter().any(|transfer| {
            transfer.id() == offer.id() && transfer.state() == FileTransferState::Cancelled
        })
    })
    .await;
    assert!(receiver.handoff.incoming().is_empty());
    assert!(!receiver.directory.path().join("cancelled.mp4").exists());
    let ordinary_offer = receiver.offer().await;
    assert_eq!(ordinary_offer.file_name(), "unrelated.bin");
    receiver
        .transfers
        .accept_incoming(ordinary_offer.id(), None)
        .unwrap();
    wait_until(|| {
        unrelated
            .transfer()
            .is_some_and(|transfer| transfer.state() == FileTransferState::Completed)
    })
    .await;
    assert!(receiver.directory.path().join("unrelated.bin").is_file());
    unrelated.release().unwrap();
    let url = sender.handoff.get_handle("url-after-cancellation").unwrap();
    url.configure_url(receiver.id(), "https://example.com")
        .unwrap();
    url.use_handle().unwrap();
    wait_until(|| {
        url.handoff()
            .is_some_and(|handoff| handoff.state() == HandoffState::Delivered)
    })
    .await;
    url.release().unwrap();
    sender.stop().await;
    receiver.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_receiver_is_rejected_before_creating_a_transfer() {
    let mut sender = Peer::new().await;
    let mut receiver = Peer::new().await;
    receiver.handoff.stop().await.unwrap();
    sender.connect(&receiver).await;
    let handle = sender.send(&receiver, "unsupported.mp4");
    wait_until(|| {
        handle
            .handoff()
            .is_some_and(|handoff| handoff.state() == HandoffState::Rejected)
    })
    .await;
    assert_eq!(
        handle.handoff().unwrap().failure(),
        Some(HandoffFailure::Unsupported)
    );
    assert!(sender.transfers.transfers().is_empty());
    assert!(receiver.transfers.transfers().is_empty());
    handle.release().unwrap();
    sender.stop().await;
    receiver.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_cancels_the_owned_transfer_and_leaves_no_ready_video() {
    let mut sender = Peer::new().await;
    let mut receiver = Peer::new().await;
    sender.connect(&receiver).await;
    let _handle = sender.send(&receiver, "shutdown.mp4");
    let offer = receiver.offer().await;
    sender.handoff.stop().await.unwrap();
    wait_until(|| {
        receiver.transfers.transfers().iter().any(|transfer| {
            transfer.id() == offer.id() && transfer.state() == FileTransferState::Cancelled
        })
    })
    .await;
    assert!(receiver.handoff.incoming().is_empty());
    assert!(!receiver.directory.path().join("shutdown.mp4").exists());
    sender.stop().await;
    receiver.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_and_integrity_failed_transfers_cannot_become_video_handoffs() {
    let mut receiver = Peer::new().await;
    let sender_id = DeviceId::new("authenticated-sender").unwrap();
    let transfer_id = [45; 16];
    let handoff = || {
        InboundHandoff::new(
            [46; 16],
            sender_id.clone(),
            HandoffTransportPayload::LocalVideo {
                transfer_id,
                playback_position_millis: 500,
            },
        )
    };
    assert_eq!(
        receiver
            .transport
            .handoff_capability()
            .receive(handoff())
            .await,
        HandoffDisposition::Rejected(HandoffRejection::Invalid)
    );
    let transport = receiver.transport.transfer_capability();
    let offer_sender = sender_id.clone();
    let offer_task = tokio::spawn(async move {
        transport
            .receive(InboundTransfer::new(
                transfer_id,
                offer_sender,
                TransferTransportMessage::Offer {
                    file_name: "broken.mp4".to_owned(),
                    file_size: 3,
                },
            ))
            .await
    });
    let offer = receiver.offer().await;
    receiver
        .transfers
        .accept_incoming(offer.id(), None)
        .unwrap();
    assert_eq!(offer_task.await.unwrap(), TransferDisposition::Accepted);
    assert_eq!(
        receiver
            .transport
            .transfer_capability()
            .receive(InboundTransfer::new(
                transfer_id,
                sender_id.clone(),
                TransferTransportMessage::Chunk {
                    offset: 0,
                    bytes: vec![1, 2, 3]
                },
            ))
            .await,
        TransferDisposition::Accepted
    );
    assert_eq!(
        receiver
            .transport
            .transfer_capability()
            .receive(InboundTransfer::new(
                transfer_id,
                sender_id.clone(),
                TransferTransportMessage::Finish { digest: [0; 32] },
            ))
            .await,
        TransferDisposition::Rejected(TransferRejection::Integrity)
    );
    assert_eq!(
        receiver
            .transport
            .handoff_capability()
            .receive(handoff())
            .await,
        HandoffDisposition::Rejected(HandoffRejection::Invalid)
    );
    assert!(receiver.handoff.incoming().is_empty());
    assert!(!receiver.directory.path().join("broken.mp4").exists());
    assert!(
        !std::fs::read_dir(receiver.directory.path())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .path()
                .extension()
                .is_some_and(|extension| extension == "part"))
    );
    receiver.stop().await;
}

#[test]
fn local_video_validation_rejects_missing_empty_and_non_video_sources() {
    let directory = tempdir().unwrap();
    for name in ["missing.mp4", "empty.mp4", "program.exe"] {
        let path = directory.path().join(name);
        if name == "empty.mp4" {
            std::fs::write(&path, []).unwrap();
        }
        if name == "program.exe" {
            std::fs::write(&path, [1]).unwrap();
        }
        assert_eq!(
            LocalVideoHandoff::new(path, Duration::ZERO),
            Err(HandoffError::InvalidLocalVideo)
        );
    }
    assert_eq!(
        LocalVideoHandoff::new(PathBuf::from("relative.mp4"), Duration::ZERO),
        Err(HandoffError::InvalidLocalVideo)
    );
    let path = directory.path().join("video.MP4");
    std::fs::write(&path, [1]).unwrap();
    let video = LocalVideoHandoff::new(path.clone(), Duration::ZERO).unwrap();
    assert_eq!(video.playback_position().as_millis(), 0);
    assert_eq!(
        LocalVideoHandoff::new(path, Duration::from_secs(u64::MAX)),
        Err(HandoffError::PlaybackPositionTooLarge)
    );
}

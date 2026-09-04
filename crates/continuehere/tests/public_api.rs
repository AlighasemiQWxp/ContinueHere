use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use continuehere::{
    Capability, ContinueHere, Device, DeviceId, DeviceIdentityChangedDelegate, DeviceManager,
    DeviceState, DirectoryChangedDelegate, DirectoryManager, DirectorySettings, DiscoveryChange,
    DiscoveryChangedDelegate, DiscoveryEndpoint, DiscoveryManager, DiscoveryMode, DiscoverySource,
    FileTransferManager, HandoffManager, Language, LanguageChangedDelegate, LocalizationKey,
    LocalizationManager, LocalizationSettings, PairingManager, Platform, PlaybackPosition,
    ProtocolVersion, SettingsManager, TextDirection, TransportManager, YouTubeHandoff,
};
use tempfile::tempdir;

#[test]
fn shared_models_are_available_through_the_public_api() {
    let id = DeviceId::new("desktop-1").expect("identifier should be valid");
    let device = Device::new(
        id,
        "Desktop",
        Platform::Windows,
        ProtocolVersion::CURRENT,
        [Capability::UrlHandoff, Capability::FileTransfer],
        DeviceState::Available,
    );

    assert_eq!(device.id().as_str(), "desktop-1");
    assert_eq!(device.display_name(), "Desktop");
    assert!(device.supports(Capability::UrlHandoff));
    assert_eq!(device.state(), DeviceState::Available);

    let youtube = YouTubeHandoff::new(
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        Duration::from_secs(452),
    )
    .expect("YouTube handoff should be valid");
    let position: PlaybackPosition = youtube.playback_position();
    assert_eq!(position.duration(), Duration::from_secs(452));
    assert_eq!(
        youtube.resume_url(),
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=452s"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn builder_exposes_the_core_managers() {
    let project = tempdir().expect("temporary project directory should be available");
    let build_result = ContinueHere::builder(project.path()).build().await;
    let app = match build_result {
        Ok(app) => app,
        Err(error) => panic!("failed to build ContinueHere: {error}"),
    };

    let _: &SettingsManager = app.settings();
    let _: &DirectorySettings = app.settings().directories();
    let _: &LocalizationSettings = app.settings().localization();
    let _: &DirectoryManager = app.directories();
    let _: &LocalizationManager = app.localization();
    let _: &DeviceManager = app.devices();
    let _: &DiscoveryManager = app.discovery();
    let _: &PairingManager = app.pairing();
    let _: &TransportManager = app.transport();
    let _: &HandoffManager = app.handoff();
    let _: &FileTransferManager = app.file_transfers();
    let identity = app.devices().identity();

    assert!(!identity.id().as_str().is_empty());
    assert!(!identity.display_name().is_empty());
    assert!(project.path().join("device_identity.bin").is_file());
    assert!(app.pairing().listening_endpoint().is_err());
    assert_ne!(
        app.transport()
            .listening_endpoint()
            .expect("application listener should be available")
            .port(),
        0
    );

    let shutdown_result = app.shutdown().await;
    if let Err(error) = shutdown_result {
        panic!("failed to shut down ContinueHere: {error}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn manual_discovery_is_public_and_scoped_to_its_handle() {
    let project = tempdir().expect("temporary project directory should be available");
    let app = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should build");
    let (sender, receiver) = mpsc::channel();
    let _subscription = app
        .discovery()
        .on_changed(DiscoveryChangedDelegate::new(move |change| {
            sender
                .send(change)
                .expect("change receiver should remain available");
        }));
    let endpoint = "127.0.0.1:5200"
        .parse::<DiscoveryEndpoint>()
        .expect("manual endpoint should be valid");
    let handle = app
        .discovery()
        .get_handle("public-manual-discovery")
        .expect("discovery handle should be available");
    handle
        .configure(DiscoveryMode::ManualEndpoint(endpoint.clone()))
        .expect("discovery handle should configure");
    handle.use_handle().expect("manual discovery should start");

    let candidate = match receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("manual candidate should be published")
    {
        DiscoveryChange::Added(candidate) => candidate,
        _ => panic!("first discovery change should add the candidate"),
    };
    assert_eq!(candidate.source(), DiscoverySource::Manual);
    assert_eq!(candidate.endpoints(), &[endpoint]);
    assert_eq!(app.discovery().candidates(), vec![candidate.clone()]);

    handle.release().expect("discovery handle should release");
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("manual candidate should be removed"),
        DiscoveryChange::Removed(candidate)
    );
    app.shutdown().await.expect("ContinueHere should stop");
}

#[tokio::test(flavor = "current_thread")]
async fn local_device_identity_persists_and_publishes_name_changes() {
    let project = tempdir().expect("temporary project directory should be available");
    let app = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should build");
    let original_id = app.devices().identity().id().clone();
    let changes = Arc::new(Mutex::new(Vec::new()));
    let recorded_changes = Arc::clone(&changes);
    let _subscription = app
        .devices()
        .on_identity_changed(DeviceIdentityChangedDelegate::new(move |identity| {
            recorded_changes
                .lock()
                .expect("recorded changes should be available")
                .push(identity);
        }));

    app.devices()
        .set_display_name("Portable Workstation")
        .expect("display name should save");

    assert_eq!(app.devices().identity().id(), &original_id);
    assert_eq!(
        changes
            .lock()
            .expect("recorded changes should be available")
            .as_slice(),
        &[app.devices().identity()]
    );
    app.shutdown().await.expect("ContinueHere should stop");

    let loaded = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should rebuild");

    assert_eq!(loaded.devices().identity().id(), &original_id);
    assert_eq!(
        loaded.devices().identity().display_name(),
        "Portable Workstation"
    );
    loaded.shutdown().await.expect("ContinueHere should stop");
}

#[tokio::test(flavor = "current_thread")]
async fn language_selection_persists_and_updates_localization() {
    let project = tempdir().expect("temporary project directory should be available");
    let app = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should build");
    let changes = Arc::new(Mutex::new(Vec::new()));
    let recorded_changes = Arc::clone(&changes);
    let _subscription = app
        .localization()
        .on_language_changed(LanguageChangedDelegate::new(move |language| {
            recorded_changes
                .lock()
                .expect("recorded changes should be available")
                .push(language);
        }));

    assert_eq!(app.localization().language(), Language::English);
    assert_eq!(
        app.localization().text_direction(),
        TextDirection::LeftToRight
    );
    assert_eq!(
        app.localization().text(LocalizationKey::LanguagePersian),
        "Persian"
    );

    app.settings()
        .localization()
        .set_language(Language::Persian)
        .expect("language should save");

    assert_eq!(app.localization().language(), Language::Persian);
    assert!(app.localization().is_rtl());
    assert_eq!(
        app.localization().text(LocalizationKey::LanguagePersian),
        "فارسی"
    );
    assert_eq!(
        *changes
            .lock()
            .expect("recorded changes should be available"),
        vec![Language::Persian]
    );
    app.shutdown().await.expect("ContinueHere should stop");

    let loaded = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should rebuild");

    assert_eq!(loaded.localization().language(), Language::Persian);
    assert_eq!(
        loaded.localization().text_direction(),
        TextDirection::RightToLeft
    );
    loaded.shutdown().await.expect("ContinueHere should stop");
}

#[tokio::test(flavor = "current_thread")]
async fn default_transfer_directory_persists_through_the_public_api() {
    let project = tempdir().expect("temporary project directory should be available");
    let saved = project.path().join("received");
    let app = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should build");
    let changes = Arc::new(Mutex::new(Vec::new()));
    let recorded_changes = Arc::clone(&changes);
    let _subscription = app
        .directories()
        .on_directory_changed(DirectoryChangedDelegate::new(move |directory| {
            recorded_changes
                .lock()
                .expect("recorded changes should be available")
                .push(directory.to_path_buf());
        }));
    app.settings()
        .directories()
        .set_default_transfer_directory(saved.clone())
        .expect("default directory should save");
    assert_eq!(
        *changes
            .lock()
            .expect("recorded changes should be available"),
        vec![saved.clone()]
    );
    app.shutdown().await.expect("ContinueHere should stop");

    let loaded = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should rebuild");

    assert_eq!(loaded.directories().default_transfer_directory(), saved);
    assert!(project.path().join("settings.bin").is_file());
    loaded.shutdown().await.expect("ContinueHere should stop");
}

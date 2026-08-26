use continuehere::{
    Capability, ContinueHere, Device, DeviceId, DeviceManager, DeviceState, DirectoryManager,
    DirectorySettings, LocalizationManager, Platform, ProtocolVersion, SettingsManager,
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
    let _: &DirectoryManager = app.directories();
    let _: &LocalizationManager = app.localization();
    let _: &DeviceManager = app.devices();

    let shutdown_result = app.shutdown().await;
    if let Err(error) = shutdown_result {
        panic!("failed to shut down ContinueHere: {error}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn default_transfer_directory_persists_through_the_public_api() {
    let project = tempdir().expect("temporary project directory should be available");
    let saved = project.path().join("received");
    let app = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should build");
    app.settings()
        .directories()
        .set_default_transfer_directory(saved.clone())
        .expect("default directory should save");
    app.shutdown().await.expect("ContinueHere should stop");

    let loaded = ContinueHere::builder(project.path())
        .build()
        .await
        .expect("ContinueHere should rebuild");

    assert_eq!(loaded.directories().default_transfer_directory(), saved);
    assert!(project.path().join("settings.bin").is_file());
    loaded.shutdown().await.expect("ContinueHere should stop");
}

use continuehere::{
    Capability, ContinueHere, Device, DeviceId, DeviceManager, DeviceState, LocalizationManager,
    Platform, ProtocolVersion, SettingsManager,
};

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
    let build_result = ContinueHere::builder().build().await;
    let app = match build_result {
        Ok(app) => app,
        Err(error) => panic!("failed to build ContinueHere: {error}"),
    };

    let _: &SettingsManager = app.settings();
    let _: &LocalizationManager = app.localization();
    let _: &LocalizationManager = app.settings().localization();
    let _: &DeviceManager = app.devices();

    assert!(std::ptr::eq(
        app.localization(),
        app.settings().localization()
    ));

    let shutdown_result = app.shutdown().await;
    if let Err(error) = shutdown_result {
        panic!("failed to shut down ContinueHere: {error}");
    }
}

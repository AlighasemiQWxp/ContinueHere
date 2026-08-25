use continuehere::{ContinueHere, DeviceManager, LocalizationManager, SettingsManager};

#[tokio::test(flavor = "current_thread")]
async fn builder_exposes_the_core_managers() {
    let build_result = ContinueHere::builder().build().await;
    let app = match build_result {
        Ok(app) => app,
        Err(error) => panic!("failed to build ContinueHere: {error}"),
    };

    let _: &SettingsManager = app.settings();
    let _: &LocalizationManager = app.localization();
    let _: &DeviceManager = app.devices();

    let shutdown_result = app.shutdown().await;
    if let Err(error) = shutdown_result {
        panic!("failed to shut down ContinueHere: {error}");
    }
}

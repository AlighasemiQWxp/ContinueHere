#![cfg(target_os = "android")]

mod application;
mod platform;
mod ui;

#[allow(
    unsafe_code,
    reason = "Android NativeActivity requires this exported entry-point symbol"
)]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    crate::platform::initialize_android(app.clone())
        .expect("Android platform initialization failed");
    slint::android::init(app).expect("Android UI initialization failed");
    crate::application::run().expect("Android application failed");
}

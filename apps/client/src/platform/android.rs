use std::sync::{Mutex, OnceLock};

use jni::objects::{GlobalRef, JObject, JValue};
use slint::android::AndroidApp;

static APPLICATION: OnceLock<Mutex<Option<AndroidApp>>> = OnceLock::new();
static MULTICAST_LOCK: OnceLock<Mutex<Option<GlobalRef>>> = OnceLock::new();

pub(super) fn initialize(app: AndroidApp) -> std::io::Result<()> {
    let application = APPLICATION.get_or_init(|| Mutex::new(None));
    *application
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(app);
    clear_imports()?;
    acquire_multicast_lock()
}

pub(super) fn shutdown() -> std::io::Result<()> {
    clear_imports()?;
    let lock = MULTICAST_LOCK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take();
    let Some(lock) = lock else {
        return Ok(());
    };
    robius_android_env::with_activity(|environment, _activity| {
        environment.call_method(lock.as_obj(), "release", "()V", &[])?;
        Ok::<(), jni::errors::Error>(())
    })
    .map_err(jni_error)?
    .map_err(jni_error)?;
    Ok(())
}

fn clear_imports() -> std::io::Result<()> {
    let imports = project_directory()?.join("imports");
    match std::fs::remove_dir_all(imports) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn project_directory() -> std::io::Result<std::path::PathBuf> {
    with_application(|application| {
        application.internal_data_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Android application storage is unavailable",
            )
        })
    })
}

fn with_application<T>(
    action: impl FnOnce(&AndroidApp) -> std::io::Result<T>,
) -> std::io::Result<T> {
    let application = APPLICATION
        .get()
        .ok_or_else(|| std::io::Error::other("Android application is not initialized"))?
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let application = application
        .as_ref()
        .ok_or_else(|| std::io::Error::other("Android application is unavailable"))?;
    action(application)
}

fn acquire_multicast_lock() -> std::io::Result<()> {
    if MULTICAST_LOCK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .is_some()
    {
        return Ok(());
    }
    let lock = robius_android_env::with_activity(|environment, activity| {
        let service_name = JObject::from(environment.new_string("wifi")?);
        let wifi_manager = environment
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_name)],
            )?
            .l()?;
        let lock_name = JObject::from(environment.new_string("ContinueHere discovery")?);
        let lock = environment
            .call_method(
                wifi_manager,
                "createMulticastLock",
                "(Ljava/lang/String;)Landroid/net/wifi/WifiManager$MulticastLock;",
                &[JValue::Object(&lock_name)],
            )?
            .l()?;
        environment.call_method(&lock, "setReferenceCounted", "(Z)V", &[JValue::Bool(0)])?;
        environment.call_method(&lock, "acquire", "()V", &[])?;
        environment.new_global_ref(lock)
    })
    .map_err(jni_error)?
    .map_err(jni_error)?;
    *MULTICAST_LOCK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(lock);
    Ok(())
}

fn jni_error(error: jni::errors::Error) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

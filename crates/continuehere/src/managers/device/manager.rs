use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use async_trait::async_trait;

use crate::{
    Error,
    core::{error::ModuleError, module::Module},
    models::{LocalDeviceIdentity, Platform},
};

use super::{
    DeviceIdentityChangedDelegate, DeviceIdentityChangedSubscription, DeviceIdentityError,
    event::DeviceIdentityChangedEvent, store::DeviceIdentityStore,
};

pub struct DeviceManager {
    state: Arc<Mutex<DeviceIdentityState>>,
    platform: Platform,
    identity_changed: DeviceIdentityChangedEvent,
}

#[derive(Clone)]
pub(crate) struct DeviceIdentityCapability {
    state: Arc<Mutex<DeviceIdentityState>>,
}

struct DeviceIdentityState {
    store: DeviceIdentityStore,
    identity: Option<LocalDeviceIdentity>,
}

impl DeviceManager {
    pub(crate) fn new(project_directory: PathBuf) -> Self {
        Self {
            state: Arc::new(Mutex::new(DeviceIdentityState {
                store: DeviceIdentityStore::new(project_directory.join("device_identity.bin")),
                identity: None,
            })),
            platform: Platform::current(),
            identity_changed: DeviceIdentityChangedEvent::default(),
        }
    }

    pub fn identity(&self) -> LocalDeviceIdentity {
        lock_state(&self.state)
            .identity
            .clone()
            .expect("device identity is available while ContinueHere is running")
    }

    pub(crate) fn capability(&self) -> DeviceIdentityCapability {
        DeviceIdentityCapability {
            state: Arc::clone(&self.state),
        }
    }

    pub fn set_display_name(&self, display_name: impl Into<String>) -> crate::Result<()> {
        let updated = {
            let mut state = lock_state(&self.state);
            let current = state
                .identity
                .as_ref()
                .ok_or(DeviceIdentityError::IdentityUnavailable)
                .map_err(Error::device_identity)?;
            let updated = current
                .renamed(display_name)
                .map_err(Error::device_identity)?;
            if &updated == current {
                return Ok(());
            }

            state.store.save(&updated).map_err(Error::device_identity)?;
            state.identity = Some(updated.clone());
            updated
        };

        self.identity_changed.publish(updated);
        Ok(())
    }

    pub fn on_identity_changed(
        &self,
        delegate: DeviceIdentityChangedDelegate,
    ) -> DeviceIdentityChangedSubscription {
        self.identity_changed.subscribe(delegate)
    }
}

impl DeviceIdentityCapability {
    pub(crate) fn identity(&self) -> Option<LocalDeviceIdentity> {
        lock_state(&self.state).identity.clone()
    }
}

#[async_trait]
impl Module for DeviceManager {
    fn name(&self) -> &'static str {
        "devices"
    }

    async fn start(&mut self) -> Result<(), ModuleError> {
        let mut state = lock_state(&self.state);
        let identity = state.store.load_or_create(self.platform)?;
        state.identity = Some(identity);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ModuleError> {
        lock_state(&self.state).identity = None;
        Ok(())
    }
}

fn lock_state(state: &Mutex<DeviceIdentityState>) -> MutexGuard<'_, DeviceIdentityState> {
    state.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    use tempfile::tempdir;

    use super::DeviceManager;
    use crate::{core::module::Module, managers::DeviceIdentityChangedDelegate};

    #[tokio::test]
    async fn manager_reloads_the_same_identity_after_restart() {
        let project = tempdir().expect("temporary project directory should be available");
        let mut manager = DeviceManager::new(project.path().to_path_buf());
        manager.start().await.expect("devices should start");
        let created = manager.identity();
        manager.stop().await.expect("devices should stop");

        manager.start().await.expect("devices should restart");

        assert_eq!(manager.identity(), created);
    }

    #[tokio::test]
    async fn display_name_change_persists_before_publishing() {
        let project = tempdir().expect("temporary project directory should be available");
        let mut manager = DeviceManager::new(project.path().to_path_buf());
        manager.start().await.expect("devices should start");
        let original_id = manager.identity().id().clone();
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded_changes = Arc::clone(&changes);
        let _subscription =
            manager.on_identity_changed(DeviceIdentityChangedDelegate::new(move |identity| {
                recorded_changes
                    .lock()
                    .expect("recorded changes should be available")
                    .push(identity);
            }));

        manager
            .set_display_name("  Workstation  ")
            .expect("display name should save");
        manager
            .set_display_name("Workstation")
            .expect("unchanged display name should succeed");

        assert_eq!(manager.identity().id(), &original_id);
        assert_eq!(manager.identity().display_name(), "Workstation");
        assert_eq!(
            changes
                .lock()
                .expect("recorded changes should be available")
                .len(),
            1
        );

        manager.stop().await.expect("devices should stop");
        manager.start().await.expect("devices should restart");
        assert_eq!(manager.identity().display_name(), "Workstation");
    }

    #[tokio::test]
    async fn failed_display_name_write_preserves_state_and_publishes_nothing() {
        let project = tempdir().expect("temporary project directory should be available");
        let identity_path = project.path().join("device_identity.bin");
        let mut manager = DeviceManager::new(project.path().to_path_buf());
        manager.start().await.expect("devices should start");
        let original = manager.identity();
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded_changes = Arc::clone(&changes);
        let _subscription =
            manager.on_identity_changed(DeviceIdentityChangedDelegate::new(move |identity| {
                recorded_changes
                    .lock()
                    .expect("recorded changes should be available")
                    .push(identity);
            }));
        fs::remove_file(&identity_path).expect("identity fixture should be removed");
        fs::create_dir(&identity_path).expect("blocking directory should be created");

        assert!(manager.set_display_name("Workstation").is_err());
        assert_eq!(manager.identity(), original);
        assert!(
            changes
                .lock()
                .expect("recorded changes should be available")
                .is_empty()
        );
    }
}

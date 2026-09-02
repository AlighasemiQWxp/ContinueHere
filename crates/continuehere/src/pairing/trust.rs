use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use tokio::sync::watch;

use crate::models::DeviceId;

use super::{PairingError, TrustedDevice, TrustedDeviceStore};

pub(crate) struct TrustedDeviceRegistry {
    state: Arc<Mutex<TrustedDeviceState>>,
    revision: watch::Sender<u64>,
}

#[derive(Clone)]
pub(crate) struct TrustedPeerLookup {
    state: Arc<Mutex<TrustedDeviceState>>,
    revision: watch::Receiver<u64>,
}

struct TrustedDeviceState {
    store: TrustedDeviceStore,
    devices: BTreeMap<DeviceId, TrustedDevice>,
    loaded: bool,
}

pub(crate) struct TrustMutation {
    device: TrustedDevice,
    newly_added: bool,
}

impl TrustedDeviceRegistry {
    pub(crate) fn new(project_directory: PathBuf) -> Self {
        let (revision, _) = watch::channel(0);
        Self {
            state: Arc::new(Mutex::new(TrustedDeviceState {
                store: TrustedDeviceStore::new(project_directory.join("trusted_devices.bin")),
                devices: BTreeMap::new(),
                loaded: false,
            })),
            revision,
        }
    }

    pub(crate) fn lookup(&self) -> TrustedPeerLookup {
        TrustedPeerLookup {
            state: Arc::clone(&self.state),
            revision: self.revision.subscribe(),
        }
    }

    pub(crate) fn load(&self) -> Result<(), PairingError> {
        let mut state = lock(&self.state)?;
        let devices = state.store.load()?;
        state.devices = devices;
        state.loaded = true;
        Ok(())
    }

    pub(crate) fn devices(&self) -> Vec<TrustedDevice> {
        match lock(&self.state) {
            Ok(state) => state.devices.values().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    pub(crate) fn persist(&self, device: TrustedDevice) -> Result<TrustMutation, PairingError> {
        let mutation =
            {
                let mut state = lock(&self.state)?;
                if let Some(existing) = state.devices.get(device.device_id()) {
                    if existing.public_key_fingerprint() != device.public_key_fingerprint() {
                        return Err(PairingError::IdentityConflict);
                    }
                    return Ok(TrustMutation {
                        device: existing.clone(),
                        newly_added: false,
                    });
                }
                if state.devices.values().any(|trusted| {
                    trusted.public_key_fingerprint() == device.public_key_fingerprint()
                }) {
                    return Err(PairingError::IdentityConflict);
                }
                let mut updated = state.devices.clone();
                updated.insert(device.device_id().clone(), device.clone());
                state.store.save(&updated)?;
                state.devices = updated;
                TrustMutation {
                    device,
                    newly_added: true,
                }
            };
        self.publish_revision();
        Ok(mutation)
    }

    pub(crate) fn remove(&self, device_id: &DeviceId) -> Result<TrustedDevice, PairingError> {
        let removed = {
            let mut state = lock(&self.state)?;
            let removed = state
                .devices
                .get(device_id)
                .cloned()
                .ok_or(PairingError::TrustedDeviceNotFound)?;
            let mut updated = state.devices.clone();
            updated.remove(device_id);
            state.store.save(&updated)?;
            state.devices = updated;
            removed
        };
        self.publish_revision();
        Ok(removed)
    }

    pub(crate) fn rollback(&self, mutation: &TrustMutation) {
        if !mutation.newly_added {
            return;
        }
        let removed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let should_remove =
                state
                    .devices
                    .get(mutation.device.device_id())
                    .is_some_and(|device| {
                        device.public_key_fingerprint() == mutation.device.public_key_fingerprint()
                    });
            if !should_remove {
                return;
            }
            let mut updated = state.devices.clone();
            updated.remove(mutation.device.device_id());
            if state.store.save(&updated).is_err() {
                return;
            }
            state.devices = updated;
            true
        };
        if removed {
            self.publish_revision();
        }
    }

    fn publish_revision(&self) {
        self.revision.send_modify(|revision| {
            *revision = revision.wrapping_add(1);
        });
    }
}

impl TrustMutation {
    pub(crate) const fn newly_added(&self) -> bool {
        self.newly_added
    }

    pub(crate) fn into_device(self) -> TrustedDevice {
        self.device
    }
}

impl TrustedPeerLookup {
    pub(crate) fn get(&self, device_id: &DeviceId) -> Result<Option<TrustedDevice>, PairingError> {
        let state = lock(&self.state)?;
        if !state.loaded {
            return Err(PairingError::ManagerUnavailable);
        }
        Ok(state.devices.get(device_id).cloned())
    }

    pub(crate) fn find_by_fingerprint(
        &self,
        fingerprint: &[u8; 32],
    ) -> Result<Option<TrustedDevice>, PairingError> {
        let state = lock(&self.state)?;
        if !state.loaded {
            return Err(PairingError::ManagerUnavailable);
        }
        Ok(state
            .devices
            .values()
            .find(|device| device.public_key_fingerprint() == fingerprint)
            .cloned())
    }

    pub(crate) fn revision(&self) -> watch::Receiver<u64> {
        self.revision.clone()
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, PairingError> {
    value
        .lock()
        .map_err(|_| PairingError::TrustSynchronizationFailed)
}

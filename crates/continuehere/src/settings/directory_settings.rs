use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use tokio::sync::watch;

use crate::Error;

use super::{error::SettingsError, store::SettingsStore};

const SECTION_NAME: &str = "directories";
const SECTION_VERSION: u16 = 1;

pub struct DirectorySettings {
    inner: Arc<DirectorySettingsInner>,
}

struct DirectorySettingsInner {
    store: Arc<Mutex<SettingsStore>>,
    default_transfer_directory: watch::Sender<PathBuf>,
}

impl DirectorySettings {
    pub(super) fn new(store: Arc<Mutex<SettingsStore>>, default_directory: PathBuf) -> Self {
        let (default_transfer_directory, _) = watch::channel(default_directory);
        Self {
            inner: Arc::new(DirectorySettingsInner {
                store,
                default_transfer_directory,
            }),
        }
    }

    pub fn default_transfer_directory(&self) -> PathBuf {
        self.inner.default_transfer_directory.borrow().clone()
    }

    pub fn set_default_transfer_directory(
        &self,
        directory: impl Into<PathBuf>,
    ) -> crate::Result<()> {
        let directory = directory.into();
        validate_directory_path(&directory).map_err(Error::settings)?;

        if self.default_transfer_directory() == directory {
            return Ok(());
        }

        let payload = encode_directory(&directory).map_err(Error::settings)?;
        let mut store = self.lock_store().map_err(Error::settings)?;
        store
            .write_section(SECTION_NAME, SECTION_VERSION, payload)
            .map_err(Error::settings)?;
        self.inner
            .default_transfer_directory
            .send_replace(directory);
        Ok(())
    }

    pub fn subscribe(&self) -> DirectorySettingsListener {
        DirectorySettingsListener {
            receiver: self.inner.default_transfer_directory.subscribe(),
        }
    }

    pub(crate) fn shared(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }

    pub(super) fn load(&self) -> std::result::Result<(), SettingsError> {
        let section = self.lock_store()?.section(SECTION_NAME);
        let Some(section) = section else {
            return Ok(());
        };

        if section.version() != SECTION_VERSION {
            return Err(SettingsError::UnsupportedSectionVersion {
                name: SECTION_NAME,
                version: section.version(),
            });
        }

        let directory = decode_directory(section.payload())?;
        self.inner
            .default_transfer_directory
            .send_replace(directory);
        Ok(())
    }

    fn lock_store(&self) -> std::result::Result<MutexGuard<'_, SettingsStore>, SettingsError> {
        self.inner
            .store
            .lock()
            .map_err(|_| SettingsError::StateUnavailable)
    }
}

pub struct DirectorySettingsListener {
    receiver: watch::Receiver<PathBuf>,
}

impl DirectorySettingsListener {
    pub fn current(&self) -> PathBuf {
        self.receiver.borrow().clone()
    }

    pub async fn changed(&mut self) -> Option<PathBuf> {
        self.receiver.changed().await.ok()?;
        Some(self.receiver.borrow_and_update().clone())
    }
}

pub(crate) fn validate_directory_path(directory: &Path) -> std::result::Result<(), SettingsError> {
    if directory.as_os_str().is_empty() || !directory.is_absolute() {
        return Err(SettingsError::InvalidDirectoryPath {
            path: directory.to_path_buf(),
        });
    }
    Ok(())
}

fn encode_directory(directory: &Path) -> std::result::Result<Vec<u8>, SettingsError> {
    let path = directory
        .to_str()
        .ok_or_else(|| SettingsError::UnsupportedDirectoryPath {
            path: directory.to_path_buf(),
        })?;
    Ok(path.as_bytes().to_vec())
}

fn decode_directory(payload: &[u8]) -> std::result::Result<PathBuf, SettingsError> {
    let path = std::str::from_utf8(payload)
        .map_err(|_| SettingsError::InvalidSectionData { name: SECTION_NAME })?;
    let directory = PathBuf::from(path);
    validate_directory_path(&directory)?;
    Ok(directory)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    use tempfile::tempdir;

    use super::DirectorySettings;
    use crate::settings::store::SettingsStore;

    #[tokio::test]
    async fn setting_a_directory_persists_and_notifies() {
        let project = tempdir().expect("temporary project directory should be available");
        let first = project.path().join("first");
        let second = project.path().join("second");
        let store = Arc::new(Mutex::new(SettingsStore::new(
            project.path().join("settings.bin"),
        )));
        let settings = DirectorySettings::new(Arc::clone(&store), first);
        let mut listener = settings.subscribe();

        settings
            .set_default_transfer_directory(second.clone())
            .expect("directory should save");

        assert_eq!(listener.changed().await, Some(second.clone()));
        assert_eq!(settings.default_transfer_directory(), second);
        assert!(project.path().join("settings.bin").is_file());
    }

    #[test]
    fn persisted_directory_loads_into_a_new_capability() {
        let project = tempdir().expect("temporary project directory should be available");
        let fallback = project.path().join("fallback");
        let saved = project.path().join("saved");
        let path = project.path().join("settings.bin");
        let store = Arc::new(Mutex::new(SettingsStore::new(path.clone())));
        let settings = DirectorySettings::new(store, fallback.clone());
        settings
            .set_default_transfer_directory(saved.clone())
            .expect("directory should save");

        let loaded_store = Arc::new(Mutex::new(SettingsStore::new(path)));
        loaded_store
            .lock()
            .expect("store should be available")
            .load()
            .expect("settings should load");
        let loaded = DirectorySettings::new(loaded_store, fallback);
        loaded.load().expect("directory section should load");

        assert_eq!(loaded.default_transfer_directory(), saved);
    }

    #[test]
    fn failed_write_keeps_the_previous_runtime_value() {
        let project = tempdir().expect("temporary project directory should be available");
        let first = project.path().join("first");
        let second = project.path().join("second");
        let path = project.path().join("settings.bin");
        let store = Arc::new(Mutex::new(SettingsStore::new(path.clone())));
        let settings = DirectorySettings::new(store, project.path().to_path_buf());
        settings
            .set_default_transfer_directory(first.clone())
            .expect("first directory should save");
        let listener = settings.subscribe();

        fs::rename(&path, project.path().join("settings.backup"))
            .expect("settings file should move inside the temporary directory");
        fs::create_dir(&path).expect("blocking directory should be created");

        assert!(settings.set_default_transfer_directory(second).is_err());
        assert_eq!(settings.default_transfer_directory(), first);
        assert_eq!(listener.current(), first);
    }
}

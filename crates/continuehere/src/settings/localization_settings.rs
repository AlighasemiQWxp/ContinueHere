use std::sync::{Arc, Mutex, MutexGuard};

use crate::{
    Error,
    locales::{
        Language, LanguageChangedDelegate, LanguageChangedEvent, LanguageChangedSubscription,
    },
};

use super::{error::SettingsError, store::SettingsStore};

const SECTION_NAME: &str = "localization";
const SECTION_VERSION: u16 = 1;
const ENGLISH_VALUE: u8 = 0;
const PERSIAN_VALUE: u8 = 1;

pub struct LocalizationSettings {
    inner: Arc<LocalizationSettingsInner>,
}

struct LocalizationSettingsInner {
    store: Arc<Mutex<SettingsStore>>,
    language: Mutex<Language>,
    language_changed: LanguageChangedEvent,
}

impl LocalizationSettings {
    pub(super) fn new(store: Arc<Mutex<SettingsStore>>) -> Self {
        Self {
            inner: Arc::new(LocalizationSettingsInner {
                store,
                language: Mutex::new(Language::default()),
                language_changed: LanguageChangedEvent::default(),
            }),
        }
    }

    pub fn language(&self) -> Language {
        *self.lock_language()
    }

    pub fn set_language(&self, language: Language) -> crate::Result<()> {
        if self.language() == language {
            return Ok(());
        }

        let payload = vec![encode_language(language)];
        let mut store = self.lock_store().map_err(Error::settings)?;
        store
            .write_section(SECTION_NAME, SECTION_VERSION, payload)
            .map_err(Error::settings)?;
        *self.lock_language() = language;
        drop(store);
        self.inner.language_changed.publish(language);
        Ok(())
    }

    pub fn on_language_changed(
        &self,
        delegate: LanguageChangedDelegate,
    ) -> LanguageChangedSubscription {
        self.inner.language_changed.subscribe(delegate)
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

        *self.lock_language() = decode_language(section.payload())?;
        Ok(())
    }

    fn lock_store(&self) -> std::result::Result<MutexGuard<'_, SettingsStore>, SettingsError> {
        self.inner
            .store
            .lock()
            .map_err(|_| SettingsError::StateUnavailable)
    }

    fn lock_language(&self) -> MutexGuard<'_, Language> {
        self.inner
            .language
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

fn encode_language(language: Language) -> u8 {
    match language {
        Language::English => ENGLISH_VALUE,
        Language::Persian => PERSIAN_VALUE,
    }
}

fn decode_language(payload: &[u8]) -> std::result::Result<Language, SettingsError> {
    match payload {
        [ENGLISH_VALUE] => Ok(Language::English),
        [PERSIAN_VALUE] => Ok(Language::Persian),
        _ => Err(SettingsError::InvalidSectionData { name: SECTION_NAME }),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    use tempfile::tempdir;

    use super::LocalizationSettings;
    use crate::{
        locales::{Language, LanguageChangedDelegate},
        settings::store::SettingsStore,
    };

    #[test]
    fn setting_a_language_persists_and_notifies_once() {
        let project = tempdir().expect("temporary project directory should be available");
        let store = Arc::new(Mutex::new(SettingsStore::new(
            project.path().join("settings.bin"),
        )));
        let settings = LocalizationSettings::new(store);
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded_changes = Arc::clone(&changes);
        let _subscription =
            settings.on_language_changed(LanguageChangedDelegate::new(move |language| {
                recorded_changes
                    .lock()
                    .expect("recorded changes should be available")
                    .push(language);
            }));

        settings
            .set_language(Language::Persian)
            .expect("language should save");
        settings
            .set_language(Language::Persian)
            .expect("unchanged language should succeed");

        assert_eq!(settings.language(), Language::Persian);
        assert_eq!(
            *changes
                .lock()
                .expect("recorded changes should be available"),
            vec![Language::Persian]
        );
        assert!(project.path().join("settings.bin").is_file());
    }

    #[test]
    fn persisted_language_loads_into_a_new_capability() {
        let project = tempdir().expect("temporary project directory should be available");
        let path = project.path().join("settings.bin");
        let store = Arc::new(Mutex::new(SettingsStore::new(path.clone())));
        let settings = LocalizationSettings::new(store);
        settings
            .set_language(Language::Persian)
            .expect("language should save");

        let loaded_store = Arc::new(Mutex::new(SettingsStore::new(path)));
        loaded_store
            .lock()
            .expect("store should be available")
            .load()
            .expect("settings should load");
        let loaded = LocalizationSettings::new(loaded_store);
        loaded.load().expect("localization section should load");

        assert_eq!(loaded.language(), Language::Persian);
    }

    #[test]
    fn malformed_language_payload_is_rejected() {
        let project = tempdir().expect("temporary project directory should be available");
        let store = Arc::new(Mutex::new(SettingsStore::new(
            project.path().join("settings.bin"),
        )));
        store
            .lock()
            .expect("store should be available")
            .write_section("localization", 1, vec![99])
            .expect("malformed fixture should save");
        let settings = LocalizationSettings::new(store);

        assert!(settings.load().is_err());
        assert_eq!(settings.language(), Language::English);
    }

    #[test]
    fn failed_write_keeps_the_previous_language_and_skips_notification() {
        let project = tempdir().expect("temporary project directory should be available");
        let path = project.path().join("settings.bin");
        let store = Arc::new(Mutex::new(SettingsStore::new(path.clone())));
        let settings = LocalizationSettings::new(store);
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded_changes = Arc::clone(&changes);
        let _subscription =
            settings.on_language_changed(LanguageChangedDelegate::new(move |language| {
                recorded_changes
                    .lock()
                    .expect("recorded changes should be available")
                    .push(language);
            }));

        fs::create_dir(&path).expect("blocking directory should be created");

        assert!(settings.set_language(Language::Persian).is_err());
        assert_eq!(settings.language(), Language::English);
        assert!(
            changes
                .lock()
                .expect("recorded changes should be available")
                .is_empty()
        );
    }

    #[test]
    fn dropping_a_subscription_stops_notifications() {
        let project = tempdir().expect("temporary project directory should be available");
        let store = Arc::new(Mutex::new(SettingsStore::new(
            project.path().join("settings.bin"),
        )));
        let settings = LocalizationSettings::new(store);
        let changes = Arc::new(Mutex::new(Vec::new()));
        let recorded_changes = Arc::clone(&changes);
        let subscription =
            settings.on_language_changed(LanguageChangedDelegate::new(move |language| {
                recorded_changes
                    .lock()
                    .expect("recorded changes should be available")
                    .push(language);
            }));

        drop(subscription);
        settings
            .set_language(Language::Persian)
            .expect("language should save");

        assert!(
            changes
                .lock()
                .expect("recorded changes should be available")
                .is_empty()
        );
    }
}

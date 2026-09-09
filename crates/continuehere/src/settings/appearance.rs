use std::sync::{Arc, Mutex};

use super::{appearance_event::AppearanceChangedEvent, error::SettingsError, store::SettingsStore};
use crate::{AppearanceChangedDelegate, AppearanceChangedSubscription, Error};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeStyle {
    #[default]
    Purple,
    Red,
    Green,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appearance {
    theme: ThemeStyle,
    brightness: u8,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemeStyle::Purple,
            brightness: 100,
        }
    }
}

impl Appearance {
    pub fn theme(self) -> ThemeStyle {
        self.theme
    }
    pub fn brightness(self) -> u8 {
        self.brightness
    }
}

pub struct AppearanceSettings {
    store: Arc<Mutex<SettingsStore>>,
    value: Mutex<Appearance>,
    changed: AppearanceChangedEvent,
}

impl AppearanceSettings {
    pub(super) fn new(store: Arc<Mutex<SettingsStore>>) -> Self {
        Self {
            store,
            value: Mutex::new(Appearance::default()),
            changed: AppearanceChangedEvent::default(),
        }
    }

    pub fn appearance(&self) -> Appearance {
        *self.value.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub fn set_theme(&self, theme: ThemeStyle) -> crate::Result<()> {
        self.update(Some(theme), None)
    }

    pub fn set_brightness(&self, brightness: u8) -> crate::Result<()> {
        if !(50..=100).contains(&brightness) {
            return Err(Error::settings(SettingsError::InvalidSectionData {
                name: "appearance",
            }));
        }
        self.update(None, Some(brightness))
    }

    pub fn on_changed(&self, delegate: AppearanceChangedDelegate) -> AppearanceChangedSubscription {
        self.changed.subscribe(delegate)
    }

    fn update(&self, theme: Option<ThemeStyle>, brightness: Option<u8>) -> crate::Result<()> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| Error::settings(SettingsError::StateUnavailable))?;
        let mut value = self.value.lock().unwrap_or_else(|error| error.into_inner());
        let next = Appearance {
            theme: theme.unwrap_or(value.theme),
            brightness: brightness.unwrap_or(value.brightness),
        };
        if *value == next {
            return Ok(());
        }
        let theme = match next.theme {
            ThemeStyle::Purple => 0,
            ThemeStyle::Red => 1,
            ThemeStyle::Green => 2,
        };
        store
            .write_section("appearance", 1, vec![theme, next.brightness])
            .map_err(Error::settings)?;
        *value = next;
        drop(value);
        drop(store);
        self.changed.publish(next);
        Ok(())
    }

    pub(super) fn load(&self) -> Result<(), SettingsError> {
        let store = self
            .store
            .lock()
            .map_err(|_| SettingsError::StateUnavailable)?;
        let Some(section) = store.section("appearance") else {
            return Ok(());
        };
        if section.version() != 1 {
            return Err(SettingsError::UnsupportedSectionVersion {
                name: "appearance",
                version: section.version(),
            });
        }
        let [theme, brightness] = section.payload() else {
            return Err(SettingsError::InvalidSectionData { name: "appearance" });
        };
        let theme = match theme {
            0 => ThemeStyle::Purple,
            1 => ThemeStyle::Red,
            2 => ThemeStyle::Green,
            _ => return Err(SettingsError::InvalidSectionData { name: "appearance" }),
        };
        if !(50..=100).contains(brightness) {
            return Err(SettingsError::InvalidSectionData { name: "appearance" });
        }
        *self.value.lock().unwrap_or_else(|error| error.into_inner()) = Appearance {
            theme,
            brightness: *brightness,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_persists_and_notifications_observe_committed_values() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.bin");
        let store = Arc::new(Mutex::new(SettingsStore::new(path.clone())));
        let settings = AppearanceSettings::new(Arc::clone(&store));
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let changes = Arc::clone(&recorded);
        let persisted = Arc::clone(&store);
        let _subscription = settings.on_changed(AppearanceChangedDelegate::new(move |value| {
            assert!(persisted.lock().unwrap().section("appearance").is_some());
            changes.lock().unwrap().push(value);
        }));
        settings.set_theme(ThemeStyle::Green).unwrap();
        settings.set_brightness(75).unwrap();
        settings.set_brightness(75).unwrap();
        assert_eq!(recorded.lock().unwrap().len(), 2);
        assert!(settings.set_brightness(49).is_err());
        let loaded = Arc::new(Mutex::new(SettingsStore::new(path)));
        loaded.lock().unwrap().load().unwrap();
        let restored = AppearanceSettings::new(loaded);
        restored.load().unwrap();
        assert_eq!(restored.appearance(), settings.appearance());
    }

    #[test]
    fn failed_save_preserves_the_previous_appearance() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("blocked");
        std::fs::create_dir(&path).unwrap();
        let settings = AppearanceSettings::new(Arc::new(Mutex::new(SettingsStore::new(path))));
        assert!(settings.set_theme(ThemeStyle::Red).is_err());
        assert_eq!(settings.appearance(), Appearance::default());
    }
}

use super::{Language, MainWindow, SettingsAccess};

pub(super) struct SettingsSnapshot {
    display_name: String,
    directory: String,
    language_index: i32,
}

pub(super) fn snapshot(access: &SettingsAccess) -> SettingsSnapshot {
    let language = access.localization().language();
    SettingsSnapshot {
        display_name: access.devices().identity().display_name().to_owned(),
        directory: access
            .directories()
            .default_transfer_directory()
            .to_string_lossy()
            .into_owned(),
        language_index: match language {
            Language::English => 0,
            Language::Persian => 1,
            _ => 0,
        },
    }
}

pub(super) fn apply_snapshot(window: &MainWindow, snapshot: SettingsSnapshot) {
    window.set_display_name(snapshot.display_name.into());
    window.set_default_directory(snapshot.directory.into());
    window.set_selected_language(snapshot.language_index);
}

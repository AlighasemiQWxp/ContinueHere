use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use continuehere::{
    AppearanceChangedDelegate, AppearanceChangedSubscription, ContinueHere,
    DeviceIdentityChangedDelegate, DeviceIdentityChangedSubscription, DirectoryChangedDelegate,
    DirectoryChangedSubscription, Language, LanguageChangedDelegate, LanguageChangedSubscription,
    ThemeStyle,
};
use slint::ComponentHandle;

use super::MainWindow;

#[derive(Clone)]
struct UiEventTarget(Arc<Mutex<slint::Weak<MainWindow>>>);

impl UiEventTarget {
    fn new(window: slint::Weak<MainWindow>) -> Self {
        Self(Arc::new(Mutex::new(window)))
    }

    fn request_refresh(&self) {
        let window = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let _event_result = window.upgrade_in_event_loop(|window| {
            window.invoke_refresh_settings_requested();
        });
    }
}

pub(super) struct SettingsUiController {
    _appearance_changed: AppearanceChangedSubscription,
    core: Rc<ContinueHere>,
    _identity_changed: DeviceIdentityChangedSubscription,
    _directory_changed: DirectoryChangedSubscription,
    _language_changed: LanguageChangedSubscription,
}

struct SettingsSnapshot {
    display_name: String,
    directory: String,
    language_index: i32,
    strings: UiStrings,
}

struct UiStrings {
    devices: &'static str,
    settings: &'static str,
    this_device: &'static str,
    nearby_devices: &'static str,
    trusted_devices: &'static str,
    no_nearby_devices: &'static str,
    no_trusted_devices: &'static str,
    manual_endpoint: &'static str,
    add: &'static str,
    active: &'static str,
    inactive: &'static str,
    connected: &'static str,
    disconnected: &'static str,
    device_name: &'static str,
    language: &'static str,
    english: &'static str,
    persian: &'static str,
    save: &'static str,
    dismiss: &'static str,
}

impl SettingsUiController {
    pub(super) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Rc<Self> {
        let event_target = UiEventTarget::new(window.as_weak());
        let appearance_target = event_target.clone();
        let appearance_changed =
            core.settings()
                .appearance()
                .on_changed(AppearanceChangedDelegate::new(move |_| {
                    appearance_target.request_refresh()
                }));
        let identity_changed =
            core.devices()
                .on_identity_changed(DeviceIdentityChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let directory_changed =
            core.directories()
                .on_directory_changed(DirectoryChangedDelegate::new(directory_refresh_delegate(
                    event_target.clone(),
                )));
        let language_changed = core
            .localization()
            .on_language_changed(LanguageChangedDelegate::new(refresh_delegate(event_target)));
        let controller = Rc::new(Self {
            _appearance_changed: appearance_changed,
            core,
            _identity_changed: identity_changed,
            _directory_changed: directory_changed,
            _language_changed: language_changed,
        });
        Self::bind_callbacks(Rc::downgrade(&controller), window);
        controller.refresh(window);
        controller
    }

    fn bind_callbacks(controller: std::rc::Weak<Self>, window: &MainWindow) {
        let theme_controller = controller.clone();
        let theme_view = window.as_weak();
        window.on_select_theme(move |index| {
            if let (Some(controller), Some(window)) =
                (theme_controller.upgrade(), theme_view.upgrade())
            {
                let theme = match index {
                    0 => ThemeStyle::Purple,
                    1 => ThemeStyle::Red,
                    2 => ThemeStyle::Green,
                    _ => return,
                };
                super::support::show_result(
                    &window,
                    controller.core.settings().appearance().set_theme(theme),
                );
                controller.refresh(&window);
            }
        });
        let brightness_controller = controller.clone();
        let brightness_view = window.as_weak();
        window.on_save_brightness(move |value| {
            if !value.is_finite() {
                return;
            }
            let Some(controller) = brightness_controller.upgrade() else {
                return;
            };
            if let Some(window) = brightness_view.upgrade() {
                super::support::show_result(
                    &window,
                    controller
                        .core
                        .settings()
                        .appearance()
                        .set_brightness(value.round().clamp(50.0, 100.0) as u8),
                );
                controller.refresh(&window);
            }
        });
        let folder_controller = controller.clone();
        let folder_window = window.as_weak();
        window.on_choose_directory(move || {
            if let (Some(controller), Some(window)) =
                (folder_controller.upgrade(), folder_window.upgrade())
            {
                let result = (|| -> super::support::UiResult {
                    if let Some(path) = crate::platform::select_directory()? {
                        controller
                            .core
                            .settings()
                            .directories()
                            .set_default_transfer_directory(path)?;
                    }
                    Ok(())
                })();
                super::support::show_result(&window, result);
            }
        });
        let window_weak = window.as_weak();
        let save_name_controller = controller.clone();
        let save_name_window = window_weak.clone();
        window.on_save_display_name(move |display_name| {
            let Some(controller) = save_name_controller.upgrade() else {
                return;
            };
            let result = controller
                .core
                .devices()
                .set_display_name(display_name.as_str());
            if result.is_ok()
                && let Some(window) = save_name_window.upgrade()
            {
                let name = controller
                    .core
                    .devices()
                    .identity()
                    .display_name()
                    .to_owned();
                let message = if window.get_rtl() {
                    format!("نام دستگاه به {name} تغییر یافت.")
                } else {
                    format!("Device name saved as {name}.")
                };
                window.invoke_show_notice(message.into());
            }
            show_result(result, &save_name_window);
        });

        let language_controller = controller.clone();
        window.on_select_language(move |index| {
            let Some(controller) = language_controller.upgrade() else {
                return;
            };
            let language = if index == 1 {
                Language::Persian
            } else {
                Language::English
            };
            let result = controller
                .core
                .settings()
                .localization()
                .set_language(language);
            show_result(result, &window_weak);
        });

        let refresh_controller = controller;
        let refresh_window = window.as_weak();
        window.on_refresh_settings_requested(move || {
            let Some(controller) = refresh_controller.upgrade() else {
                return;
            };
            let Some(window) = refresh_window.upgrade() else {
                return;
            };
            controller.refresh(&window);
        });
    }

    fn refresh(&self, window: &MainWindow) {
        let appearance = self.core.settings().appearance().appearance();
        window.set_theme_style(match appearance.theme() {
            ThemeStyle::Purple => 0,
            ThemeStyle::Red => 1,
            ThemeStyle::Green => 2,
        });
        window.set_brightness(f32::from(appearance.brightness()));
        apply_snapshot(window, snapshot(&self.core));
        window.invoke_refresh_pairing_requested();
        window.invoke_refresh_transfers_requested();
        window.invoke_refresh_history_requested();
    }
}

fn refresh_delegate<T>(event_target: UiEventTarget) -> impl Fn(T) + Send + Sync + 'static
where
    T: 'static,
{
    move |_| {
        event_target.request_refresh();
    }
}

fn directory_refresh_delegate(
    event_target: UiEventTarget,
) -> impl Fn(&std::path::Path) + Send + Sync + 'static {
    move |_| {
        event_target.request_refresh();
    }
}

fn snapshot(core: &ContinueHere) -> SettingsSnapshot {
    let language = core.localization().language();
    SettingsSnapshot {
        display_name: core.devices().identity().display_name().to_owned(),
        directory: core
            .directories()
            .default_transfer_directory()
            .to_string_lossy()
            .into_owned(),
        language_index: match language {
            Language::English => 0,
            Language::Persian => 1,
            _ => 0,
        },
        strings: UiStrings::new(language),
    }
}

fn apply_snapshot(window: &MainWindow, snapshot: SettingsSnapshot) {
    window.set_display_name(snapshot.display_name.into());
    window.set_default_directory(snapshot.directory.into());
    window.set_selected_language(snapshot.language_index);
    window.set_rtl(snapshot.language_index == 1);
    window.set_devices_text(snapshot.strings.devices.into());
    window.set_settings_text(snapshot.strings.settings.into());
    window.set_this_device_text(snapshot.strings.this_device.into());
    window.set_nearby_devices_text(snapshot.strings.nearby_devices.into());
    window.set_trusted_devices_text(snapshot.strings.trusted_devices.into());
    window.set_no_nearby_devices_text(snapshot.strings.no_nearby_devices.into());
    window.set_no_trusted_devices_text(snapshot.strings.no_trusted_devices.into());
    window.set_manual_endpoint_text(snapshot.strings.manual_endpoint.into());
    window.set_add_text(snapshot.strings.add.into());
    window.set_active_text(snapshot.strings.active.into());
    window.set_inactive_text(snapshot.strings.inactive.into());
    window.set_connected_text(snapshot.strings.connected.into());
    window.set_disconnected_text(snapshot.strings.disconnected.into());
    window.set_device_name_text(snapshot.strings.device_name.into());
    window.set_language_text(snapshot.strings.language.into());
    window.set_english_text(snapshot.strings.english.into());
    window.set_persian_text(snapshot.strings.persian.into());
    window.set_save_text(snapshot.strings.save.into());
    window.set_dismiss_text(snapshot.strings.dismiss.into());
}

fn show_result(result: continuehere::Result<()>, window: &slint::Weak<MainWindow>) {
    let Some(window) = window.upgrade() else {
        return;
    };
    match result {
        Ok(()) => window.set_error_message("".into()),
        Err(error) => window.set_error_message(error.to_string().into()),
    }
}

impl UiStrings {
    fn new(language: Language) -> Self {
        match language {
            Language::English => Self {
                devices: "Receive",
                settings: "Settings",
                this_device: "This device",
                nearby_devices: "Nearby devices",
                trusted_devices: "Trusted devices",
                no_nearby_devices: "No nearby devices found.",
                no_trusted_devices: "No trusted devices yet.",
                manual_endpoint: "Manual endpoint (host:port)",
                add: "Add",
                active: "Discovery active",
                inactive: "Discovery unavailable",
                connected: "Connected",
                disconnected: "Disconnected",
                device_name: "Device name",
                language: "Language",
                english: "English",
                persian: "Persian",
                save: "Save",
                dismiss: "Dismiss",
            },
            Language::Persian => Self {
                devices: "دریافت",
                settings: "تنظیمات",
                this_device: "این دستگاه",
                nearby_devices: "دستگاه‌های نزدیک",
                trusted_devices: "دستگاه‌های مورد اعتماد",
                no_nearby_devices: "هیچ دستگاه نزدیکی پیدا نشد.",
                no_trusted_devices: "هنوز دستگاه مورد اعتمادی وجود ندارد.",
                manual_endpoint: "نشانی دستی (میزبان:درگاه)",
                add: "افزودن",
                active: "جست‌وجو فعال است",
                inactive: "جست‌وجو در دسترس نیست",
                connected: "متصل",
                disconnected: "قطع شده",
                device_name: "نام دستگاه",
                language: "زبان",
                english: "انگلیسی",
                persian: "فارسی",
                save: "ذخیره",
                dismiss: "بستن",
            },
            _ => Self::new(Language::English),
        }
    }
}

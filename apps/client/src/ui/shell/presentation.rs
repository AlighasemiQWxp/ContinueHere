use continuehere::{Language, ThemeStyle};

use crate::ui::{MainWindow, shell::ShellAccess};

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

pub(super) fn apply(access: &ShellAccess, window: &MainWindow) {
    let language = access.localization().language();
    let strings = UiStrings::new(language);
    let appearance = access.appearance().appearance();
    window.set_theme_style(match appearance.theme() {
        ThemeStyle::Purple => 0,
        ThemeStyle::Red => 1,
        ThemeStyle::Green => 2,
        ThemeStyle::Blue => 3,
    });
    window.set_brightness(f32::from(appearance.brightness()));
    window.set_rtl(language == Language::Persian);
    window.set_devices_text(strings.devices.into());
    window.set_settings_text(strings.settings.into());
    window.set_this_device_text(strings.this_device.into());
    window.set_nearby_devices_text(strings.nearby_devices.into());
    window.set_trusted_devices_text(strings.trusted_devices.into());
    window.set_no_nearby_devices_text(strings.no_nearby_devices.into());
    window.set_no_trusted_devices_text(strings.no_trusted_devices.into());
    window.set_manual_endpoint_text(strings.manual_endpoint.into());
    window.set_add_text(strings.add.into());
    window.set_active_text(strings.active.into());
    window.set_inactive_text(strings.inactive.into());
    window.set_connected_text(strings.connected.into());
    window.set_disconnected_text(strings.disconnected.into());
    window.set_device_name_text(strings.device_name.into());
    window.set_language_text(strings.language.into());
    window.set_english_text(strings.english.into());
    window.set_persian_text(strings.persian.into());
    window.set_save_text(strings.save.into());
    window.set_dismiss_text(strings.dismiss.into());
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

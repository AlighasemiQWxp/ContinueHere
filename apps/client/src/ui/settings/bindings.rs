use std::{cell::RefCell, rc::Rc};

use continuehere::{Language, ThemeStyle};
use slint::ComponentHandle;

use crate::ui::{
    MainWindow,
    selection::{ContentSelectionUiController, selected_path},
};

use super::{SettingsUiController, show_result};

impl SettingsUiController {
    pub(super) fn bind_callbacks(
        controller: std::rc::Weak<Self>,
        window: &MainWindow,
        selection: Rc<RefCell<ContentSelectionUiController>>,
    ) {
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
                crate::ui::shared::show_result(
                    &window,
                    controller.access.appearance().set_theme(theme),
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
                crate::ui::shared::show_result(
                    &window,
                    controller
                        .access
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
                let selected_controller = Rc::downgrade(&controller);
                let result =
                    selection
                        .borrow_mut()
                        .choose_directory(&window, move |window, result| {
                            let result = selected_path(result).and_then(|path| {
                                let Some(path) = path else {
                                    return Ok(());
                                };
                                let Some(controller) = selected_controller.upgrade() else {
                                    return Ok(());
                                };
                                controller
                                    .access
                                    .directories()
                                    .set_default_transfer_directory(path)?;
                                Ok(())
                            });
                            crate::ui::shared::show_result(window, result);
                        });
                crate::ui::shared::show_result(&window, result);
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
                .access
                .devices()
                .set_display_name(display_name.as_str());
            if result.is_ok()
                && let Some(window) = save_name_window.upgrade()
            {
                let name = controller
                    .access
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
            let result = controller.access.localization().set_language(language);
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
}

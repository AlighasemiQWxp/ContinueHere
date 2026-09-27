use std::{cell::RefCell, rc::Weak as RcWeak};

use slint::ComponentHandle;

use crate::ui::MainWindow;

use super::{DeviceAction, DevicesUiController};

impl DevicesUiController {
    pub(super) fn bind_callbacks(controller: RcWeak<RefCell<Self>>, window: &MainWindow) {
        let network_controller = controller.clone();
        let network_view = window.as_weak();
        window.on_refresh_network(move || {
            if let (Some(controller), Some(window)) =
                (network_controller.upgrade(), network_view.upgrade())
            {
                controller.borrow_mut().refresh_network(&window);
            }
        });
        let reconnect_controller = controller.clone();
        let reconnect_view = window.as_weak();
        window.on_reconnect_requested(move |id| {
            if let (Some(controller), Some(window)) =
                (reconnect_controller.upgrade(), reconnect_view.upgrade())
            {
                let result = controller.borrow_mut().open_reconnect(id.as_str(), &window);
                crate::ui::shared::show_result(&window, result);
            }
        });
        let close_controller = controller.clone();
        let close_window = window.as_weak();
        window.on_close_reconnect_requested(move || {
            if let (Some(controller), Some(window)) =
                (close_controller.upgrade(), close_window.upgrade())
            {
                controller.borrow_mut().close_reconnect(&window);
            }
        });
        let poll_controller = controller.clone();
        let poll_window = window.as_weak();
        window.on_poll_connection_requested(move || {
            if let (Some(controller), Some(window)) =
                (poll_controller.upgrade(), poll_window.upgrade())
            {
                controller.borrow_mut().poll_connection(&window);
            }
        });
        let action_controller = controller.clone();
        let action_window = window.as_weak();
        window.on_device_action(move |id, action, endpoint| {
            if let (Some(controller), Some(window)) =
                (action_controller.upgrade(), action_window.upgrade())
            {
                let result = match DeviceAction::parse(action.as_str()) {
                    Some(action) => {
                        controller
                            .borrow_mut()
                            .act(id.as_str(), action, endpoint.as_str(), &window)
                    }
                    None => Err("Unknown device action.".into()),
                };
                crate::ui::shared::show_result(&window, result);
            }
        });
        let window_weak = window.as_weak();
        let add_controller = controller.clone();
        window.on_add_manual_endpoint(move |value| {
            let Some(controller) = add_controller.upgrade() else {
                return;
            };
            let Some(window) = window_weak.upgrade() else {
                return;
            };
            match controller.borrow_mut().add_manual_endpoint(value.as_str()) {
                Ok(true) => {}
                Ok(false) => {
                    window.invoke_show_notice(
                        crate::ui::shared::text(
                            window.get_rtl(),
                            "This endpoint is already in the list.",
                            "این نشانی از قبل در فهرست وجود دارد.",
                        )
                        .into(),
                    );
                }
                Err(error) => window.invoke_show_error_requested(error.to_string().into()),
            }
        });

        let refresh_controller = controller;
        let refresh_window = window.as_weak();
        window.on_refresh_devices_requested(move || {
            let Some(controller) = refresh_controller.upgrade() else {
                return;
            };
            let Some(window) = refresh_window.upgrade() else {
                return;
            };
            controller.borrow().refresh(&window);
        });
    }
}

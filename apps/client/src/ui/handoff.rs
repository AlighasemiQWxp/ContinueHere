use std::{cell::RefCell, rc::Rc};

use continuehere::{
    ContinueHere, DeviceId, HandoffChangedDelegate, HandoffChangedSubscription, HandoffHandle,
    HandoffPayload, HandoffState, IncomingHandoffChangedDelegate,
    IncomingHandoffChangedSubscription,
};
use slint::ComponentHandle;

use super::{
    ContentRow, MainWindow,
    support::{EventTarget, UiResult, model, notify, playback_position, show_result},
};

pub(super) struct HandoffUiController {
    core: Rc<ContinueHere>,
    handles: Vec<HandoffHandle>,
    next_handle: u64,
    _changed: HandoffChangedSubscription,
    _incoming: IncomingHandoffChangedSubscription,
}

impl HandoffUiController {
    pub(super) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let incoming_target = target.clone();
        let changed = core
            .handoff()
            .on_handoff_changed(HandoffChangedDelegate::new(move |_| {
                target.dispatch(|window| {
                    window.invoke_refresh_handoffs_requested();
                    notify(&window, 1);
                });
            }));
        let incoming = core
            .handoff()
            .on_incoming_changed(IncomingHandoffChangedDelegate::new(move |_| {
                incoming_target.dispatch(|window| {
                    window.invoke_refresh_handoffs_requested();
                    notify(&window, 0);
                });
            }));
        let controller = Rc::new(RefCell::new(Self {
            core,
            handles: Vec::new(),
            next_handle: 0,
            _changed: changed,
            _incoming: incoming,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_send_handoff(move |kind, device, value, position| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                if kind.as_str() == "preview" {
                    let result = (|| -> UiResult {
                        if let Some(path) =
                            crate::platform::select_file(crate::platform::SelectionKind::Video)?
                        {
                            window.invoke_open_file(
                                path.to_string_lossy().as_ref().into(),
                                "0".into(),
                            );
                        }
                        Ok(())
                    })();
                    show_result(&window, result);
                    return;
                }
                if kind.as_str() == "current-video" {
                    let result = controller.borrow_mut().send_current_video(
                        device.as_str(),
                        value.as_str(),
                        window.get_preview_position(),
                    );
                    if result.is_ok() {
                        window.invoke_show_notice(
                            super::support::text(
                                window.get_rtl(),
                                "Video handoff started.",
                                "ارسال ویدیو آغاز شد.",
                            )
                            .into(),
                        );
                    }
                    show_result(&window, result);
                    return;
                }
                let result = controller.borrow_mut().send(
                    kind.as_str(),
                    device.as_str(),
                    value.as_str(),
                    position.as_str(),
                );
                show_result(&window, result);
                controller.borrow_mut().refresh(&window);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_incoming_action(move |id, action| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result =
                    controller
                        .borrow()
                        .incoming_action(id.as_str(), action.as_str(), &window);
                show_result(&window, result);
            }
        });
        let weak = Rc::downgrade(&controller);
        window.on_cancel_video_transfer(move |id| {
            let Some(controller) = weak.upgrade() else {
                return false;
            };
            let mut controller = controller.borrow_mut();
            let index = controller.handles.iter().position(|handle| {
                handle.handoff().is_some_and(|handoff| {
                    handoff
                        .transfer()
                        .is_some_and(|transfer| transfer.id().as_str() == id.as_str())
                })
            });
            if let Some(index) = index {
                controller.handles.remove(index);
                return true;
            }
            false
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_refresh_handoffs_requested(move || {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                controller.borrow_mut().refresh(&window);
            }
        });
        controller.borrow_mut().refresh(window);
        controller
    }

    fn send(&mut self, kind: &str, device: &str, value: &str, position: &str) -> UiResult {
        let device = DeviceId::new(device.to_owned())?;
        if !self
            .core
            .transport()
            .connections()
            .iter()
            .any(|connection| connection.device_id() == &device)
        {
            return Err("Connect to the trusted device before sending.".into());
        }
        let position = playback_position(position)?;
        self.next_handle += 1;
        let handle = self
            .core
            .handoff()
            .get_handle(&format!("ui.handoff.{}", self.next_handle))?;
        match kind {
            "url" => handle.configure_url(device, value.trim())?,
            "youtube" => handle.configure_youtube(device, value.trim(), position)?,
            "video" => {
                let Some(path) =
                    crate::platform::select_file(crate::platform::SelectionKind::Video)?
                else {
                    return Ok(());
                };
                handle.configure_local_video(device, path, position)?;
            }
            _ => return Err("Unknown handoff kind.".into()),
        }
        handle.use_handle()?;
        self.handles.push(handle);
        Ok(())
    }

    fn send_current_video(&mut self, device: &str, path: &str, seconds: f32) -> UiResult {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("Invalid playback position.".into());
        }
        let device = DeviceId::new(device.to_owned())?;
        if !self
            .core
            .transport()
            .connections()
            .iter()
            .any(|value| value.device_id() == &device)
        {
            return Err("Connect to the trusted device before sending.".into());
        }
        self.next_handle += 1;
        let handle = self
            .core
            .handoff()
            .get_handle(&format!("ui.handoff.{}", self.next_handle))?;
        handle.configure_local_video(
            device,
            std::path::PathBuf::from(path),
            std::time::Duration::from_secs_f64(f64::from(seconds)),
        )?;
        handle.use_handle()?;
        self.handles.push(handle);
        Ok(())
    }

    fn incoming_action(&self, id: &str, action: &str, window: &MainWindow) -> UiResult {
        let handoff = self
            .core
            .handoff()
            .incoming()
            .into_iter()
            .find(|item| item.id().as_str() == id)
            .ok_or("This handoff is unavailable.")?;
        match action {
            "remove" => self.core.handoff().remove_incoming(handoff.id())?,
            "open" => match handoff.payload() {
                HandoffPayload::Url(value) => crate::platform::open_url(value.url())?,
                HandoffPayload::YouTube(value) => crate::platform::open_url(&value.resume_url())?,
                HandoffPayload::LocalVideo(value) => window.invoke_open_file(
                    value.file_path().to_string_lossy().as_ref().into(),
                    value.playback_position().as_millis().to_string().into(),
                ),
                _ => return Err("Unsupported handoff content.".into()),
            },
            _ => return Err("Unknown handoff action.".into()),
        }
        Ok(())
    }

    fn refresh(&mut self, window: &MainWindow) {
        self.handles.retain(|handle| {
            handle
                .handoff()
                .is_none_or(|handoff| handoff.state() == HandoffState::Sending)
        });
        let rows = self
            .core
            .handoff()
            .incoming()
            .into_iter()
            .map(|handoff| {
                let title = match handoff.payload() {
                    HandoffPayload::Url(value) => value.url().to_owned(),
                    HandoffPayload::YouTube(value) => value.resume_url(),
                    HandoffPayload::LocalVideo(value) => {
                        value.file_path().to_string_lossy().into_owned()
                    }
                    _ => String::new(),
                };
                ContentRow {
                    id: handoff.id().to_string().into(),
                    title: title.into(),
                    detail: handoff.sender_device_id().to_string().into(),
                    ..Default::default()
                }
            })
            .collect();
        window.set_incoming_handoffs(model(rows));
    }
}

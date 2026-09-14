use std::{cell::RefCell, path::PathBuf, rc::Rc};

use continuehere::{
    ContinueHere, DeviceId, DocumentContinuation, HandoffChangedDelegate,
    HandoffChangedSubscription, HandoffHandle, HandoffState, IncomingHandoffChange,
    IncomingHandoffChangedDelegate, IncomingHandoffChangedSubscription,
};
use slint::ComponentHandle;

use super::{
    MainWindow,
    support::{EventTarget, UiResult, notify, playback_position, show_result},
    transition::{UiTransition, UiTransitionController, UiTransitionHandle},
};

#[derive(Clone, Copy)]
enum DocumentKind {
    Pdf,
    PowerPoint,
}

struct PendingDocument {
    device: DeviceId,
    path: PathBuf,
    kind: DocumentKind,
    transition: UiTransitionHandle,
}

pub(super) struct HandoffUiController {
    core: Rc<ContinueHere>,
    handles: Vec<HandoffHandle>,
    pending_document: Option<PendingDocument>,
    next_handle: u64,
    _changed: HandoffChangedSubscription,
    _incoming: IncomingHandoffChangedSubscription,
}

impl HandoffUiController {
    pub(super) fn start(
        core: Rc<ContinueHere>,
        window: &MainWindow,
        transitions: Rc<UiTransitionController>,
    ) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let incoming_target = target.clone();
        let changed = core
            .handoff()
            .on_handoff_changed(HandoffChangedDelegate::new(move |_| {
                target.dispatch(|window| {
                    window.invoke_refresh_handoffs_requested();
                    window.invoke_refresh_transfers_requested();
                    notify(&window, 1);
                });
            }));
        let incoming = core
            .handoff()
            .on_incoming_changed(IncomingHandoffChangedDelegate::new(move |change| {
                if !matches!(change, IncomingHandoffChange::Added(_)) {
                    return;
                }
                incoming_target.dispatch(|window| {
                    window.invoke_refresh_handoffs_requested();
                    window.invoke_refresh_transfers_requested();
                    notify(&window, 0);
                });
            }));
        let controller = Rc::new(RefCell::new(Self {
            core,
            handles: Vec::new(),
            pending_document: None,
            next_handle: 0,
            _changed: changed,
            _incoming: incoming,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        let document_transitions = Rc::clone(&transitions);
        window.on_choose_document(move |device, kind| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = controller.borrow_mut().choose_document(
                    device.as_str(),
                    kind.as_str(),
                    &window,
                    document_transitions.as_ref(),
                );
                show_result(&window, result);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_document_action(move |action, position| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = controller.borrow_mut().document_action(
                    action.as_str(),
                    position.as_str(),
                    &window,
                );
                if result.is_ok() && action.as_str() == "send" {
                    window.invoke_show_notice(
                        super::support::text(
                            window.get_rtl(),
                            "Document handoff started.",
                            "ارسال سند آغاز شد.",
                        )
                        .into(),
                    );
                }
                show_result(&window, result);
            }
        });
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
                controller.borrow_mut().refresh();
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
        window.on_refresh_handoffs_requested(move || {
            if let Some(controller) = weak.upgrade() {
                controller.borrow_mut().refresh();
            }
        });
        controller.borrow_mut().refresh();
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

    fn choose_document(
        &mut self,
        device: &str,
        kind: &str,
        window: &MainWindow,
        transitions: &UiTransitionController,
    ) -> UiResult {
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
        let (kind, selection) = match kind {
            "pdf" => (DocumentKind::Pdf, crate::platform::SelectionKind::Pdf),
            "powerpoint" => (
                DocumentKind::PowerPoint,
                crate::platform::SelectionKind::PowerPoint,
            ),
            _ => return Err("Unknown document type.".into()),
        };
        let Some(path) = crate::platform::select_file(selection)? else {
            return Ok(());
        };
        let title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let transition = transitions.get_handle("document-continuation");
        transition.configure(UiTransition::Document)?;
        transition.use_handle()?;
        window.set_document_title(title.into());
        window.set_document_kind(
            match kind {
                DocumentKind::Pdf => "pdf",
                DocumentKind::PowerPoint => "powerpoint",
            }
            .into(),
        );
        window.set_document_position_input("1".into());
        self.pending_document = Some(PendingDocument {
            device,
            path,
            kind,
            transition,
        });
        Ok(())
    }

    fn document_action(&mut self, action: &str, position: &str, window: &MainWindow) -> UiResult {
        if action == "cancel" {
            self.close_document(window);
            return Ok(());
        }
        if action != "send" {
            return Err("Unknown document action.".into());
        }
        let position = position
            .trim()
            .parse::<u32>()
            .map_err(|_| "Enter a valid page or slide number.")?;
        let pending = self
            .pending_document
            .as_ref()
            .ok_or("No document is selected.")?;
        let continuation = match pending.kind {
            DocumentKind::Pdf => DocumentContinuation::pdf_page(position)?,
            DocumentKind::PowerPoint => DocumentContinuation::powerpoint_slide(position)?,
        };
        if !self
            .core
            .transport()
            .connections()
            .iter()
            .any(|connection| connection.device_id() == &pending.device)
        {
            return Err("Connect to the trusted device before sending.".into());
        }
        self.next_handle += 1;
        let handle = self
            .core
            .handoff()
            .get_handle(&format!("ui.handoff.{}", self.next_handle))?;
        handle.configure_document(pending.device.clone(), pending.path.clone(), continuation)?;
        handle.use_handle()?;
        self.handles.push(handle);
        self.close_document(window);
        Ok(())
    }

    fn close_document(&mut self, window: &MainWindow) {
        if let Some(pending) = self.pending_document.take() {
            pending.transition.release();
        }
        window.set_document_title("".into());
        window.set_document_kind("".into());
        window.set_document_position_input("".into());
    }

    fn refresh(&mut self) {
        self.handles.retain(|handle| {
            handle
                .handoff()
                .is_none_or(|handoff| handoff.state() == HandoffState::Sending)
        });
        let Ok(activities) = self.core.activity().activities() else {
            return;
        };
        for handoff in self.core.handoff().incoming() {
            let activity_id = format!("handoff.in.{}", handoff.id());
            if activities
                .entries()
                .iter()
                .any(|activity| activity.id() == activity_id.as_str())
            {
                let _result = self.core.handoff().remove_incoming(handoff.id());
            }
        }
    }
}

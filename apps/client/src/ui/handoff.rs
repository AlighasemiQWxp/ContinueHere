use std::{cell::RefCell, path::PathBuf, rc::Rc};

use continuehere::{
    ContinueHere, DeviceId, HandoffChangedDelegate, HandoffChangedSubscription, HandoffHandle,
    HandoffState, IncomingHandoffChange, IncomingHandoffChangedDelegate,
    IncomingHandoffChangedSubscription,
};
use slint::ComponentHandle;

mod document;

use document::{
    DocumentDraft, DocumentKind, DocumentPhase, DocumentPreparation, DocumentSelection,
};

use super::{
    MainWindow,
    phase::{PhaseChangedDelegate, PhaseChangedSubscription},
    selection::{ContentSelectionUiController, selected_path},
    support::{EventTarget, UiResult, notify, playback_position, show_result},
    transition::{UiTransition, UiTransitionController, UiTransitionHandle},
};

pub(super) struct HandoffUiController {
    core: Rc<ContinueHere>,
    handles: Vec<HandoffHandle>,
    document: DocumentPreparation,
    document_transition: Option<UiTransitionHandle>,
    _document_changed: PhaseChangedSubscription<DocumentPhase>,
    next_handle: u64,
    _changed: HandoffChangedSubscription,
    _incoming: IncomingHandoffChangedSubscription,
}

impl HandoffUiController {
    pub(super) fn start(
        core: Rc<ContinueHere>,
        window: &MainWindow,
        transitions: Rc<UiTransitionController>,
        selection: Rc<RefCell<ContentSelectionUiController>>,
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
        let document = DocumentPreparation::new();
        let document_target = EventTarget::new(window);
        let document_changed =
            document.on_phase_changed(PhaseChangedDelegate::new(move |change| {
                if change.previous != change.current {
                    document_target.dispatch(|window| window.invoke_refresh_handoffs_requested());
                }
            }));
        let controller = Rc::new(RefCell::new(Self {
            core,
            handles: Vec::new(),
            document,
            document_transition: None,
            _document_changed: document_changed,
            next_handle: 0,
            _changed: changed,
            _incoming: incoming,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        let document_transitions = Rc::clone(&transitions);
        let document_selection = Rc::clone(&selection);
        window.on_choose_document(move |device, kind| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = (|| -> UiResult {
                    let (device, kind, selection_kind) = controller
                        .borrow()
                        .document_selection(device.as_str(), kind.as_str())?;
                    let selected_controller = Rc::downgrade(&controller);
                    let selected_transitions = Rc::clone(&document_transitions);
                    let request = controller.borrow_mut().document.start_selection()?;
                    let result = document_selection.borrow_mut().choose_file(
                        &window,
                        selection_kind,
                        move |window, result| {
                            let Some(controller) = selected_controller.upgrade() else {
                                return;
                            };
                            let mut controller = controller.borrow_mut();
                            if !controller.document.accepts_selection(&request) {
                                return;
                            }
                            let result = selected_path(result).and_then(|path| {
                                let Some(path) = path else {
                                    controller.close_document(window);
                                    return Ok(());
                                };
                                controller.begin_document(
                                    &request,
                                    DocumentDraft { device, kind, path },
                                    window,
                                    selected_transitions.as_ref(),
                                )
                            });
                            if result.is_err() {
                                controller.close_document(window);
                            }
                            drop(controller);
                            show_result(window, result);
                        },
                    );
                    if result.is_err() {
                        controller.borrow_mut().close_document(&window);
                    }
                    result
                })();
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
        let handoff_selection = selection;
        window.on_send_handoff(move |kind, device, value, position| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                if kind.as_str() == "preview" {
                    let result = handoff_selection.borrow_mut().choose_file(
                        &window,
                        crate::platform::SelectionKind::Video,
                        move |window, result| {
                            let result = selected_path(result).map(|path| {
                                if let Some(path) = path {
                                    window.invoke_open_file(
                                        path.to_string_lossy().as_ref().into(),
                                        "0".into(),
                                    );
                                }
                            });
                            show_result(window, result);
                        },
                    );
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
                if kind.as_str() == "video" {
                    let result = (|| -> UiResult {
                        let device = controller.borrow().available_device(device.as_str())?;
                        let position = playback_position(position.as_str())?;
                        let selected_controller = Rc::downgrade(&controller);
                        handoff_selection.borrow_mut().choose_file(
                            &window,
                            crate::platform::SelectionKind::Video,
                            move |window, result| {
                                let result = selected_path(result).and_then(|path| {
                                    let Some(path) = path else {
                                        return Ok(());
                                    };
                                    let Some(controller) = selected_controller.upgrade() else {
                                        return Ok(());
                                    };
                                    controller
                                        .borrow_mut()
                                        .send_local_video(device, path, position)
                                });
                                show_result(window, result);
                            },
                        )
                    })();
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
        let device = self.available_device(device)?;
        let position = playback_position(position)?;
        self.next_handle += 1;
        let handle = self
            .core
            .handoff()
            .get_handle(&format!("ui.handoff.{}", self.next_handle))?;
        match kind {
            "url" => handle.configure_url(device, value.trim())?,
            "youtube" => handle.configure_youtube(device, value.trim(), position)?,
            _ => return Err("Unknown handoff kind.".into()),
        }
        handle.use_handle()?;
        self.handles.push(handle);
        Ok(())
    }

    fn available_device(&self, device: &str) -> UiResult<DeviceId> {
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
        Ok(device)
    }

    fn send_local_video(
        &mut self,
        device: DeviceId,
        path: PathBuf,
        position: std::time::Duration,
    ) -> UiResult {
        self.next_handle += 1;
        let handle = self
            .core
            .handoff()
            .get_handle(&format!("ui.handoff.{}", self.next_handle))?;
        handle.configure_local_video(device, path, position)?;
        handle.use_handle()?;
        self.handles.push(handle);
        Ok(())
    }

    fn send_current_video(&mut self, device: &str, path: &str, seconds: f32) -> UiResult {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("Invalid playback position.".into());
        }
        let device = self.available_device(device)?;
        self.send_local_video(
            device,
            std::path::PathBuf::from(path),
            std::time::Duration::from_secs_f64(f64::from(seconds)),
        )
    }

    fn document_selection(
        &self,
        device: &str,
        kind: &str,
    ) -> UiResult<(DeviceId, DocumentKind, crate::platform::SelectionKind)> {
        let device = self.available_device(device)?;
        let (kind, selection) = match kind {
            "pdf" => (DocumentKind::Pdf, crate::platform::SelectionKind::Pdf),
            "powerpoint" => (
                DocumentKind::PowerPoint,
                crate::platform::SelectionKind::PowerPoint,
            ),
            _ => return Err("Unknown document type.".into()),
        };
        Ok((device, kind, selection))
    }

    fn begin_document(
        &mut self,
        request: &DocumentSelection,
        draft: DocumentDraft,
        window: &MainWindow,
        transitions: &UiTransitionController,
    ) -> UiResult {
        if !self.document.accepts_selection(request) {
            return Ok(());
        }
        let transition = transitions.get_handle("document-continuation");
        transition.configure(UiTransition::Document)?;
        transition.use_handle()?;
        if self.document.edit(request, draft) {
            self.document_transition = Some(transition);
            window.set_document_position_input("1".into());
        } else {
            transition.release();
        }
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
        let continuation = self.document.continuation(position)?;
        let pending = self.document.draft()?;
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
        if let Some(transition) = self.document_transition.take() {
            transition.release();
        }
        window.set_document_title("".into());
        window.set_document_kind("".into());
        window.set_document_position_input("".into());
        self.document.cancel();
    }

    fn refresh(&mut self, window: &MainWindow) {
        if let Ok(draft) = self.document.draft() {
            window.set_document_title(
                draft
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .as_ref()
                    .into(),
            );
            window.set_document_kind(
                match draft.kind {
                    DocumentKind::Pdf => "pdf",
                    DocumentKind::PowerPoint => "powerpoint",
                }
                .into(),
            );
        } else {
            window.set_document_title("".into());
            window.set_document_kind("".into());
        }
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

impl Drop for HandoffUiController {
    fn drop(&mut self) {
        if let Some(transition) = self.document_transition.take() {
            transition.release();
        }
        self.document.cancel();
    }
}

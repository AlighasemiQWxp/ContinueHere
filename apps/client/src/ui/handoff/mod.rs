mod access;
mod bindings;

pub(in crate::ui) use access::HandoffAccess;

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use continuehere::{
    DeviceId, FileTransferId, HandoffChangedDelegate, HandoffChangedSubscription, HandoffHandle,
    HandoffState, IncomingHandoffChange, IncomingHandoffChangedDelegate,
    IncomingHandoffChangedSubscription,
};

mod document;

use document::{
    DocumentDraft, DocumentKind, DocumentPhase, DocumentPreparation, DocumentSelection,
};

use super::{
    MainWindow,
    preview::PreviewOpener,
    selection::ContentSelectionUiController,
    shared::{
        EventTarget, UiResult, notify,
        phase::{PhaseChangedDelegate, PhaseChangedSubscription},
        playback_position,
    },
    transition::{UiTransition, UiTransitionController, UiTransitionHandle},
};

pub(super) struct HandoffUiController {
    access: HandoffAccess,
    handles: Vec<HandoffHandle>,
    document: DocumentPreparation,
    document_transition: Option<UiTransitionHandle>,
    _document_changed: PhaseChangedSubscription<DocumentPhase>,
    next_handle: u64,
    _changed: HandoffChangedSubscription,
    _incoming: IncomingHandoffChangedSubscription,
}

pub(in crate::ui) struct HandoffTransfers {
    controller: std::rc::Weak<RefCell<HandoffUiController>>,
}

#[derive(Clone, Copy)]
pub(super) enum SendHandoffCommand {
    Url,
    YouTube,
    Preview,
    CurrentVideo,
    Video,
}

#[derive(Clone, Copy)]
pub(super) enum HandoffKind {
    Url,
    YouTube,
}

impl SendHandoffCommand {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "url" => Some(Self::Url),
            "youtube" => Some(Self::YouTube),
            "preview" => Some(Self::Preview),
            "current-video" => Some(Self::CurrentVideo),
            "video" => Some(Self::Video),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum DocumentAction {
    Send,
    Cancel,
}

impl DocumentAction {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "send" => Some(Self::Send),
            "cancel" => Some(Self::Cancel),
            _ => None,
        }
    }
}

impl HandoffUiController {
    pub(in crate::ui) fn transfers(controller: &Rc<RefCell<Self>>) -> HandoffTransfers {
        HandoffTransfers {
            controller: Rc::downgrade(controller),
        }
    }

    pub(super) fn start(
        access: HandoffAccess,
        window: &MainWindow,
        transitions: Rc<UiTransitionController>,
        selection: Rc<RefCell<ContentSelectionUiController>>,
        preview: PreviewOpener,
    ) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let incoming_target = target.clone();
        let changed = access
            .handoff()
            .on_handoff_changed(HandoffChangedDelegate::new(move |_| {
                target.dispatch(|window| {
                    window.invoke_refresh_handoffs_requested();
                    notify(&window, 1);
                });
            }));
        let incoming = access
            .handoff()
            .on_incoming_changed(IncomingHandoffChangedDelegate::new(move |change| {
                if !matches!(change, IncomingHandoffChange::Added(_)) {
                    return;
                }
                incoming_target.dispatch(|window| {
                    window.invoke_refresh_handoffs_requested();
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
            access,
            handles: Vec::new(),
            document,
            document_transition: None,
            _document_changed: document_changed,
            next_handle: 0,
            _changed: changed,
            _incoming: incoming,
        }));
        Self::bind_callbacks(
            Rc::clone(&controller),
            window,
            transitions,
            selection,
            preview,
        );
        controller.borrow_mut().refresh(window);
        controller
    }

    fn send(&mut self, kind: HandoffKind, device: &str, value: &str, position: &str) -> UiResult {
        let device = self.available_device(device)?;
        let position = playback_position(position)?;
        self.next_handle += 1;
        let handle = self
            .access
            .handoff()
            .get_handle(&format!("ui.handoff.{}", self.next_handle))?;
        match kind {
            HandoffKind::Url => handle.configure_url(device, value.trim())?,
            HandoffKind::YouTube => handle.configure_youtube(device, value.trim(), position)?,
        }
        handle.use_handle()?;
        self.handles.push(handle);
        Ok(())
    }

    fn available_device(&self, device: &str) -> UiResult<DeviceId> {
        let device = DeviceId::new(device.to_owned())?;
        if !self
            .access
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
            .access
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
        kind: DocumentKind,
    ) -> UiResult<(DeviceId, DocumentKind, crate::platform::SelectionKind)> {
        let device = self.available_device(device)?;
        let (kind, selection) = match kind {
            DocumentKind::Pdf => (DocumentKind::Pdf, crate::platform::SelectionKind::Pdf),
            DocumentKind::PowerPoint => (
                DocumentKind::PowerPoint,
                crate::platform::SelectionKind::PowerPoint,
            ),
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
        let device_name = self
            .access
            .transport()
            .connections()
            .into_iter()
            .find(|connection| connection.device_id() == &draft.device)
            .map(|connection| connection.display_name().to_owned())
            .unwrap_or_default();
        if self.document.edit(request, draft) {
            self.document_transition = Some(transition);
            window.set_document_device_name(device_name.into());
            window.set_document_position_input("1".into());
        } else {
            transition.release();
        }
        Ok(())
    }

    fn document_action(
        &mut self,
        action: DocumentAction,
        position: &str,
        window: &MainWindow,
    ) -> UiResult {
        if matches!(action, DocumentAction::Cancel) {
            self.close_document(window);
            return Ok(());
        }
        let continuation = self.document.continuation(position)?;
        let pending = self.document.draft()?;
        if !self
            .access
            .transport()
            .connections()
            .iter()
            .any(|connection| connection.device_id() == &pending.device)
        {
            return Err("Connect to the trusted device before sending.".into());
        }
        self.next_handle += 1;
        let handle = self
            .access
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
        window.set_document_device_name("".into());
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
        let Ok(activities) = self.access.activity().activities() else {
            return;
        };
        for handoff in self.access.handoff().incoming() {
            let activity_id = format!("handoff.in.{}", handoff.id());
            if activities
                .entries()
                .iter()
                .any(|activity| activity.id() == activity_id.as_str())
            {
                let _result = self.access.handoff().remove_incoming(handoff.id());
            }
        }
    }
}

impl HandoffTransfers {
    pub(in crate::ui) fn cancel_video_transfer(&self, id: &FileTransferId) -> bool {
        let Some(controller) = self.controller.upgrade() else {
            return false;
        };
        let mut controller = controller.borrow_mut();
        let index = controller.handles.iter().position(|handle| {
            handle.handoff().is_some_and(|handoff| {
                handoff
                    .transfer()
                    .is_some_and(|transfer| transfer.id() == id)
            })
        });
        if let Some(index) = index {
            controller.handles.remove(index);
            return true;
        }
        false
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

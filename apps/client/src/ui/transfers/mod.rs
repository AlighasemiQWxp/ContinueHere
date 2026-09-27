mod access;

pub(in crate::ui) use access::TransfersAccess;

use std::{cell::RefCell, rc::Rc};

use continuehere::{
    DeviceId, FileTransferChangedDelegate, FileTransferChangedSubscription, FileTransferDirection,
    FileTransferHandle, FileTransferState, LanguageChangedDelegate, LanguageChangedSubscription,
};
use slint::ComponentHandle;

use super::{
    ContentRow, MainWindow,
    handoff::HandoffTransfers,
    preview::PreviewOpener,
    selection::{ContentSelectionUiController, selected_path},
    shared::{EventTarget, UiResult, model, notify, show_result, text},
};

pub(super) struct TransferUiController {
    access: TransfersAccess,
    handoff_transfers: HandoffTransfers,
    preview: PreviewOpener,
    handles: Vec<FileTransferHandle>,
    next_handle: u64,
    _changed: FileTransferChangedSubscription,
    _language_changed: LanguageChangedSubscription,
}

#[derive(Clone, Copy)]
enum TransferAction {
    Accept,
    Folder,
    Reject,
    Remove,
    Open,
}

impl TransferAction {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "accept" => Some(Self::Accept),
            "folder" => Some(Self::Folder),
            "reject" => Some(Self::Reject),
            "remove" => Some(Self::Remove),
            "open" => Some(Self::Open),
            _ => None,
        }
    }
}

impl TransferUiController {
    pub(super) fn start(
        access: TransfersAccess,
        handoff_transfers: HandoffTransfers,
        window: &MainWindow,
        selection: Rc<RefCell<ContentSelectionUiController>>,
        preview: PreviewOpener,
    ) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let language_target = target.clone();
        let changed =
            access
                .file_transfers()
                .on_transfer_changed(FileTransferChangedDelegate::new(move |change| {
                    let transfer = match change {
                        continuehere::FileTransferChange::Added(value)
                        | continuehere::FileTransferChange::Updated(value)
                        | continuehere::FileTransferChange::Removed(value) => value,
                        _ => return,
                    };
                    let page = if transfer.direction() == FileTransferDirection::Incoming {
                        0
                    } else {
                        1
                    };
                    target.dispatch(move |window| {
                        window.invoke_refresh_transfers_requested();
                        notify(&window, page);
                    });
                }));
        let language_changed =
            access
                .localization()
                .on_language_changed(LanguageChangedDelegate::new(move |_| {
                    language_target.dispatch(|window| window.invoke_refresh_transfers_requested());
                }));
        let controller = Rc::new(RefCell::new(Self {
            access,
            handoff_transfers,
            preview,
            handles: Vec::new(),
            next_handle: 0,
            _changed: changed,
            _language_changed: language_changed,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        let file_selection = Rc::clone(&selection);
        window.on_send_file(move |device, kind| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = (|| -> UiResult {
                    let kind = crate::platform::SelectionKind::parse(kind.as_str())?;
                    let device = controller.borrow().available_device(device.as_str())?;
                    let selected_controller = Rc::downgrade(&controller);
                    file_selection
                        .borrow_mut()
                        .choose_file(&window, kind, move |window, result| {
                            let result = selected_path(result).and_then(|path| {
                                let Some(path) = path else {
                                    return Ok(());
                                };
                                let Some(controller) = selected_controller.upgrade() else {
                                    return Ok(());
                                };
                                controller.borrow_mut().send_selected(device, kind, path)
                            });
                            show_result(window, result);
                        })
                })();
                show_result(&window, result);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        let directory_selection = selection;
        window.on_transfer_action(move |id, action| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let action = match TransferAction::parse(action.as_str()) {
                    Some(action) => action,
                    None => {
                        let result: UiResult = Err("Unknown transfer action.".into());
                        show_result(&window, result);
                        return;
                    }
                };
                if matches!(action, TransferAction::Folder) {
                    let transfer_id = id.to_string();
                    let selected_controller = Rc::downgrade(&controller);
                    let refresh_controller = selected_controller.clone();
                    let result = directory_selection.borrow_mut().choose_directory(
                        &window,
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
                                    .accept_incoming(transfer_id.as_str(), Some(&path))
                            });
                            show_result(window, result);
                            if let Some(controller) = refresh_controller.upgrade() {
                                controller.borrow_mut().refresh(window);
                            }
                        },
                    );
                    show_result(&window, result);
                    return;
                }
                let result = controller.borrow_mut().act(id.as_str(), action);
                show_result(&window, result);
                controller.borrow_mut().refresh(&window);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_refresh_transfers_requested(move || {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                controller.borrow_mut().refresh(&window);
            }
        });
        controller.borrow_mut().refresh(window);
        controller
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

    fn send_selected(
        &mut self,
        device: DeviceId,
        kind: crate::platform::SelectionKind,
        path: std::path::PathBuf,
    ) -> UiResult {
        if kind == crate::platform::SelectionKind::File
            && matches!(
                crate::platform::extension(&path).as_str(),
                "pdf" | "ppt" | "pptx"
            )
        {
            return Err(
                "Use the PDF or PowerPoint choice to preserve your continuation point.".into(),
            );
        }
        self.next_handle += 1;
        let handle = self
            .access
            .file_transfers()
            .get_handle(&format!("ui.transfer.{}", self.next_handle))?;
        if kind == crate::platform::SelectionKind::Folder {
            handle.configure_folder(device, path)?;
        } else {
            handle.configure(device, path)?;
        }
        handle.use_handle()?;
        self.handles.push(handle);
        Ok(())
    }

    fn accept_incoming(&mut self, id: &str, directory: Option<&std::path::Path>) -> UiResult {
        let transfer = self
            .access
            .file_transfers()
            .transfers()
            .into_iter()
            .find(|item| item.id().as_str() == id)
            .ok_or("This transfer is unavailable.")?;
        self.access
            .file_transfers()
            .accept_incoming(transfer.id(), directory)?;
        Ok(())
    }

    fn act(&mut self, id: &str, action: TransferAction) -> UiResult {
        let transfer = self
            .access
            .file_transfers()
            .transfers()
            .into_iter()
            .find(|item| item.id().as_str() == id)
            .ok_or("This transfer is unavailable.")?;
        match action {
            TransferAction::Accept => self
                .access
                .file_transfers()
                .accept_incoming(transfer.id(), None)?,
            TransferAction::Folder => {
                return Err("Folder selection did not complete.".into());
            }
            TransferAction::Reject => {
                self.access
                    .file_transfers()
                    .reject_incoming(transfer.id())?;
            }
            TransferAction::Remove => {
                if let Some(index) = self.handles.iter().position(|handle| {
                    handle
                        .transfer()
                        .is_some_and(|item| item.id().as_str() == id)
                }) {
                    self.handles.remove(index);
                } else if !self.handoff_transfers.cancel_video_transfer(transfer.id()) {
                    self.access.file_transfers().remove(transfer.id())?;
                }
            }
            TransferAction::Open => {
                if transfer.direction() != FileTransferDirection::Incoming
                    || transfer.state() != FileTransferState::Completed
                {
                    return Err("This transferred file cannot be opened.".into());
                }
                let path = transfer
                    .destination()
                    .ok_or("The received file is unavailable.")?;
                let continuation = self.access.handoff().incoming().iter().find_map(|handoff| {
                    match handoff.payload() {
                        continuehere::HandoffPayload::LocalVideo(video)
                            if video.transfer_id() == Some(transfer.id()) =>
                        {
                            Some((None, video.playback_position().as_millis()))
                        }
                        continuehere::HandoffPayload::LocalDocument(document)
                            if document.transfer_id() == Some(transfer.id()) =>
                        {
                            Some((Some(document.continuation()), 0))
                        }
                        _ => None,
                    }
                });
                if let Some((Some(continuation), _)) = continuation {
                    crate::platform::open_document(path, continuation)?;
                } else {
                    if matches!(
                        crate::platform::extension(path).as_str(),
                        "pdf" | "ppt" | "pptx"
                    ) {
                        return Err("The document is verified, but its continuation metadata is not ready yet.".into());
                    }
                    let position = continuation.map_or(0, |(_, position)| position);
                    self.preview.open(path.to_path_buf(), position)?;
                }
            }
        }
        Ok(())
    }

    fn refresh(&mut self, window: &MainWindow) {
        let rtl = window.get_rtl();
        let rows: Vec<ContentRow> = self
            .access
            .file_transfers()
            .transfers()
            .into_iter()
            .map(|transfer| {
                let state = match transfer.state() {
                    FileTransferState::Preparing => text(rtl, "Preparing", "در حال آماده‌سازی"),
                    FileTransferState::WaitingForAcceptance => {
                        text(rtl, "Waiting for acceptance", "در انتظار پذیرش")
                    }
                    FileTransferState::Offered => {
                        text(rtl, "Awaiting your approval", "در انتظار تأیید شما")
                    }
                    FileTransferState::Transferring => text(rtl, "Transferring", "در حال انتقال"),
                    FileTransferState::Verifying => text(rtl, "Verifying", "در حال بررسی"),
                    FileTransferState::Completed => text(rtl, "Completed", "تکمیل شد"),
                    FileTransferState::Rejected => text(rtl, "Rejected", "رد شد"),
                    FileTransferState::Cancelled => text(rtl, "Cancelled", "لغو شد"),
                    _ => text(rtl, "Failed", "ناموفق"),
                };
                let direction = if transfer.direction() == FileTransferDirection::Incoming {
                    text(rtl, "Incoming", "دریافتی")
                } else {
                    text(rtl, "Outgoing", "ارسالی")
                };
                let mut detail = format!(
                    "{} · {}\n{} / {} bytes",
                    direction,
                    transfer.peer_device_id(),
                    transfer.transferred_bytes(),
                    transfer.file_size()
                );
                if let Some(failure) = transfer.failure() {
                    detail.push_str(&format!("\n{failure:?}"));
                }
                if let Some(path) = transfer.destination() {
                    detail.push_str(&format!("\n{}", path.display()));
                }
                let progress = if transfer.file_size() == 0 {
                    0.0
                } else {
                    transfer.transferred_bytes() as f32 / transfer.file_size() as f32
                };
                ContentRow {
                    incoming: transfer.direction() == FileTransferDirection::Incoming,
                    id: transfer.id().to_string().into(),
                    title: transfer.file_name().into(),
                    detail: detail.into(),
                    status: state.into(),
                    progress,
                    offered: transfer.state() == FileTransferState::Offered
                        && transfer.direction() == FileTransferDirection::Incoming,
                    can_open: transfer.state() == FileTransferState::Completed
                        && transfer.direction() == FileTransferDirection::Incoming
                        && transfer.destination().is_some(),
                    active: matches!(
                        transfer.state(),
                        FileTransferState::Preparing
                            | FileTransferState::WaitingForAcceptance
                            | FileTransferState::Offered
                            | FileTransferState::Transferring
                            | FileTransferState::Verifying
                    ),
                    ..Default::default()
                }
            })
            .collect();
        window.set_incoming_transfers(model(
            rows.iter()
                .filter(|item| item.incoming && item.active)
                .cloned()
                .collect(),
        ));
        window.set_outgoing_transfers(model(
            rows.iter().filter(|item| !item.incoming).cloned().collect(),
        ));
        self.handles.retain(|handle| {
            handle.transfer().is_some_and(|transfer| {
                !matches!(
                    transfer.state(),
                    FileTransferState::Completed
                        | FileTransferState::Rejected
                        | FileTransferState::Cancelled
                        | FileTransferState::Failed
                )
            })
        });
    }
}

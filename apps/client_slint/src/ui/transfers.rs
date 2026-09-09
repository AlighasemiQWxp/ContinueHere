use std::{cell::RefCell, rc::Rc};

use continuehere::{
    ContinueHere, DeviceId, FileTransferChangedDelegate, FileTransferChangedSubscription,
    FileTransferDirection, FileTransferHandle, FileTransferState,
};
use slint::ComponentHandle;

use super::{
    ContentRow, MainWindow,
    support::{EventTarget, UiResult, model, notify, show_result, text},
};

pub(super) struct TransferUiController {
    core: Rc<ContinueHere>,
    handles: Vec<FileTransferHandle>,
    next_handle: u64,
    _changed: FileTransferChangedSubscription,
}

impl TransferUiController {
    pub(super) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let changed = core
            .file_transfers()
            .on_transfer_changed(FileTransferChangedDelegate::new(move |_| {
                target.dispatch(|window| {
                    window.invoke_refresh_transfers_requested();
                    notify(&window, 2);
                });
            }));
        let controller = Rc::new(RefCell::new(Self {
            core,
            handles: Vec::new(),
            next_handle: 0,
            _changed: changed,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_send_file(move |device| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = controller.borrow_mut().send(device.as_str());
                show_result(&window, result);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_transfer_action(move |id, action| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = controller
                    .borrow_mut()
                    .act(id.as_str(), action.as_str(), &window);
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

    fn send(&mut self, device: &str) -> UiResult {
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
        let Some(path) = crate::platform::select_file(false)? else {
            return Ok(());
        };
        self.next_handle += 1;
        let handle = self
            .core
            .file_transfers()
            .get_handle(&format!("ui.transfer.{}", self.next_handle))?;
        handle.configure(device, path)?;
        handle.use_handle()?;
        self.handles.push(handle);
        Ok(())
    }

    fn act(&mut self, id: &str, action: &str, window: &MainWindow) -> UiResult {
        let transfer = self
            .core
            .file_transfers()
            .transfers()
            .into_iter()
            .find(|item| item.id().as_str() == id)
            .ok_or("This transfer is unavailable.")?;
        match action {
            "accept" => self
                .core
                .file_transfers()
                .accept_incoming(transfer.id(), None)?,
            "folder" => {
                let Some(path) = crate::platform::select_directory()? else {
                    return Ok(());
                };
                self.core
                    .file_transfers()
                    .accept_incoming(transfer.id(), Some(&path))?;
            }
            "reject" => self.core.file_transfers().reject_incoming(transfer.id())?,
            "remove" => {
                if let Some(index) = self.handles.iter().position(|handle| {
                    handle
                        .transfer()
                        .is_some_and(|item| item.id().as_str() == id)
                }) {
                    self.handles.remove(index);
                } else if !window.invoke_cancel_video_transfer(id.into()) {
                    self.core.file_transfers().remove(transfer.id())?;
                }
            }
            "open" => {
                if transfer.direction() != FileTransferDirection::Incoming
                    || transfer.state() != FileTransferState::Completed
                {
                    return Err("This transferred file cannot be opened.".into());
                }
                let path = transfer
                    .destination()
                    .ok_or("The received file is unavailable.")?;
                window.invoke_open_file(path.to_string_lossy().as_ref().into(), "0".into());
            }
            _ => return Err("Unknown transfer action.".into()),
        }
        Ok(())
    }

    fn refresh(&mut self, window: &MainWindow) {
        let rtl = window.get_rtl();
        let rows = self
            .core
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
        window.set_transfers(model(rows));
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

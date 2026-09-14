use std::{
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::{
    models::{Capability, DeviceId},
    transfer::{
        FileTransferChangedDelegate, FileTransferFailure, FileTransferHandle, FileTransferState,
    },
    transport::{DocumentTransportKind, HandoffTransportPayload},
};

use super::{
    DocumentContinuation, HandoffController, HandoffFailure, HandoffId, LocalDocumentHandoff,
    LocalVideoHandoff,
    controller::{OperationCommand, OperationResult, map_transport_error},
};

const WAKE_INTERVAL: Duration = Duration::from_millis(25);
const SUPPORT_TIMEOUT: Duration = Duration::from_secs(15);

struct FilePreparation<'a> {
    controller: &'a HandoffController,
    handle_identifier: &'a str,
    handoff_id: &'a HandoffId,
    device_id: DeviceId,
    capability: Capability,
    handle_prefix: &'a str,
    source: &'a Path,
    commands: &'a mpsc::Receiver<OperationCommand>,
}

pub(super) fn prepare(
    controller: &HandoffController,
    handle_identifier: &str,
    handoff_id: &HandoffId,
    device_id: DeviceId,
    video: LocalVideoHandoff,
    commands: &mpsc::Receiver<OperationCommand>,
) -> Result<(FileTransferHandle, HandoffTransportPayload), OperationResult> {
    let position = video.playback_position().as_millis();
    prepare_file(
        FilePreparation {
            controller,
            handle_identifier,
            handoff_id,
            device_id,
            capability: Capability::LocalVideoHandoff,
            handle_prefix: "local-video",
            source: video.file_path(),
            commands,
        },
        move |transfer_id| HandoffTransportPayload::LocalVideo {
            transfer_id,
            playback_position_millis: position,
        },
    )
}

pub(super) fn prepare_document(
    controller: &HandoffController,
    handle_identifier: &str,
    handoff_id: &HandoffId,
    device_id: DeviceId,
    document: LocalDocumentHandoff,
    commands: &mpsc::Receiver<OperationCommand>,
) -> Result<(FileTransferHandle, HandoffTransportPayload), OperationResult> {
    let continuation = document.continuation();
    let document_kind = match continuation {
        DocumentContinuation::PdfPage(_) => DocumentTransportKind::Pdf,
        DocumentContinuation::PowerPointSlide(_) => DocumentTransportKind::PowerPoint,
    };
    prepare_file(
        FilePreparation {
            controller,
            handle_identifier,
            handoff_id,
            device_id,
            capability: Capability::LocalDocumentHandoff,
            handle_prefix: "local-document",
            source: document.file_path(),
            commands,
        },
        move |transfer_id| HandoffTransportPayload::LocalDocument {
            transfer_id,
            document_kind,
            position: continuation.position(),
        },
    )
}

fn prepare_file(
    preparation: FilePreparation<'_>,
    payload: impl FnOnce([u8; 16]) -> HandoffTransportPayload,
) -> Result<(FileTransferHandle, HandoffTransportPayload), OperationResult> {
    let response = preparation
        .controller
        .transport_capability()
        .check_transfer_backed_support(preparation.device_id.clone(), preparation.capability)
        .map_err(map_transport_error)?;
    let started = Instant::now();
    loop {
        check_cancelled(preparation.commands)?;
        match response.try_recv() {
            Ok(result) => {
                result.map_err(map_transport_error)?;
                break;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(OperationResult::Failed(HandoffFailure::Transport));
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if started.elapsed() >= SUPPORT_TIMEOUT {
            return Err(OperationResult::Failed(HandoffFailure::TimedOut));
        }
        wait_for_cancel(preparation.commands)?;
    }

    let transfers = preparation.controller.transfer_capability();
    let (wake, changed) = mpsc::sync_channel(1);
    let _subscription = transfers.on_changed(FileTransferChangedDelegate::new(move |_| {
        let _ = wake.try_send(());
    }));
    let handle = transfers
        .get_handle(&format!(
            "{}-{}",
            preparation.handle_prefix,
            preparation.handoff_id.as_str()
        ))
        .map_err(|_| OperationResult::Failed(HandoffFailure::FileTransfer))?;
    handle
        .configure(preparation.device_id, preparation.source)
        .map_err(|_| OperationResult::Failed(HandoffFailure::Invalid))?;
    check_cancelled(preparation.commands)?;
    handle
        .use_handle()
        .map_err(|_| OperationResult::Failed(HandoffFailure::FileTransfer))?;
    let mut refresh = true;
    loop {
        check_cancelled(preparation.commands)?;
        if refresh {
            let transfer = handle.transfer().ok_or(OperationResult::Cancelled)?;
            preparation
                .controller
                .update_transfer(preparation.handle_identifier, transfer.clone());
            match transfer.state() {
                FileTransferState::Completed => {
                    return Ok((handle, payload(transfer.id().bytes())));
                }
                FileTransferState::Rejected => {
                    return Err(OperationResult::Rejected(map_failure(transfer.failure())));
                }
                FileTransferState::Failed => {
                    return Err(OperationResult::Failed(map_failure(transfer.failure())));
                }
                FileTransferState::Cancelled => return Err(OperationResult::Cancelled),
                _ => {}
            }
        }
        wait_for_cancel(preparation.commands)?;
        refresh = changed.try_recv().is_ok();
    }
}

fn check_cancelled(commands: &mpsc::Receiver<OperationCommand>) -> Result<(), OperationResult> {
    match commands.try_recv() {
        Err(mpsc::TryRecvError::Empty) => Ok(()),
        _ => Err(OperationResult::Cancelled),
    }
}

fn wait_for_cancel(commands: &mpsc::Receiver<OperationCommand>) -> Result<(), OperationResult> {
    match commands.recv_timeout(WAKE_INTERVAL) {
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(()),
        _ => Err(OperationResult::Cancelled),
    }
}

fn map_failure(failure: Option<FileTransferFailure>) -> HandoffFailure {
    match failure {
        Some(FileTransferFailure::NotConnected) => HandoffFailure::NotConnected,
        Some(FileTransferFailure::Unsupported) => HandoffFailure::Unsupported,
        Some(FileTransferFailure::Invalid) => HandoffFailure::Invalid,
        Some(FileTransferFailure::Busy) => HandoffFailure::Busy,
        Some(FileTransferFailure::TimedOut) => HandoffFailure::TimedOut,
        Some(FileTransferFailure::Transport) => HandoffFailure::Transport,
        _ => HandoffFailure::FileTransfer,
    }
}

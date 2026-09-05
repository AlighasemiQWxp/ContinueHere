use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::{
    models::DeviceId,
    transfer::{
        FileTransferChangedDelegate, FileTransferFailure, FileTransferHandle, FileTransferState,
    },
    transport::HandoffTransportPayload,
};

use super::{
    HandoffController, HandoffFailure, HandoffId, LocalVideoHandoff,
    controller::{OperationCommand, OperationResult, map_transport_error},
};

const WAKE_INTERVAL: Duration = Duration::from_millis(25);
const SUPPORT_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) fn prepare(
    controller: &HandoffController,
    handle_identifier: &str,
    handoff_id: &HandoffId,
    device_id: DeviceId,
    video: LocalVideoHandoff,
    commands: &mpsc::Receiver<OperationCommand>,
) -> Result<(FileTransferHandle, HandoffTransportPayload), OperationResult> {
    let response = controller
        .transport_capability()
        .check_local_video_support(device_id.clone())
        .map_err(map_transport_error)?;
    let started = Instant::now();
    loop {
        check_cancelled(commands)?;
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
        wait_for_cancel(commands)?;
    }

    let transfers = controller.transfer_capability();
    let (wake, changed) = mpsc::sync_channel(1);
    let _subscription = transfers.on_changed(FileTransferChangedDelegate::new(move |_| {
        let _ = wake.try_send(());
    }));
    let handle = transfers
        .get_handle(&format!("local-video-{}", handoff_id.as_str()))
        .map_err(|_| OperationResult::Failed(HandoffFailure::FileTransfer))?;
    handle
        .configure(device_id, video.file_path())
        .map_err(|_| OperationResult::Failed(HandoffFailure::Invalid))?;
    check_cancelled(commands)?;
    handle
        .use_handle()
        .map_err(|_| OperationResult::Failed(HandoffFailure::FileTransfer))?;
    let mut refresh = true;
    loop {
        check_cancelled(commands)?;
        if refresh {
            let transfer = handle.transfer().ok_or(OperationResult::Cancelled)?;
            controller.update_transfer(handle_identifier, transfer.clone());
            match transfer.state() {
                FileTransferState::Completed => {
                    return Ok((
                        handle,
                        HandoffTransportPayload::LocalVideo {
                            transfer_id: transfer.id().bytes(),
                            playback_position_millis: video.playback_position().as_millis(),
                        },
                    ));
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
        wait_for_cancel(commands)?;
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

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    models::DeviceId,
    settings::DirectorySettings,
    transport::{
        InboundTransfer, InboundTransferHandler, TransferDisposition, TransferRejection,
        TransferTransportCapability, TransferTransportMessage, TransportError,
    },
};

use super::{
    FileTransfer, FileTransferChange, FileTransferChangedEvent, FileTransferConfig,
    FileTransferError, FileTransferFailure, FileTransferId, FileTransferState,
    model::{MAX_FILE_SIZE, TRANSFER_CHUNK_SIZE, validate_file_name},
};

const MAX_ACTIVE_TRANSFERS: usize = 4;
const MAX_INCOMING_OFFERS: usize = 16;
const RESULT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const OFFER_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) struct FileTransferController {
    state: Mutex<ControllerState>,
    transport: TransferTransportCapability,
    directories: DirectorySettings,
    changed: FileTransferChangedEvent,
}

struct ControllerState {
    running: bool,
    transfers: BTreeMap<FileTransferId, FileTransfer>,
    outgoing: HashMap<String, OutgoingOperation>,
    incoming: BTreeMap<FileTransferId, IncomingOperation>,
    orphan_workers: Vec<JoinHandle<()>>,
}

struct OutgoingOperation {
    transfer_id: FileTransferId,
    commands: mpsc::Sender<OperationCommand>,
    worker: Option<JoinHandle<()>>,
}

struct IncomingOperation {
    sender_device_id: DeviceId,
    decision: Option<mpsc::Sender<TransferDisposition>>,
    session: Option<ReceiveSession>,
}

struct ReceiveSession {
    file: File,
    hasher: Sha256,
    temporary_path: PathBuf,
    destination_path: PathBuf,
}

enum OperationCommand {
    Cancel,
}

impl FileTransferController {
    pub(crate) fn new(
        transport: TransferTransportCapability,
        directories: DirectorySettings,
        changed: FileTransferChangedEvent,
    ) -> Arc<Self> {
        let controller = Arc::new(Self {
            state: Mutex::new(ControllerState {
                running: false,
                transfers: BTreeMap::new(),
                outgoing: HashMap::new(),
                incoming: BTreeMap::new(),
                orphan_workers: Vec::new(),
            }),
            transport: transport.clone(),
            directories,
            changed,
        });
        let handler: Arc<dyn InboundTransferHandler> = controller.clone();
        transport.set_handler(Arc::downgrade(&handler));
        controller
    }

    pub(crate) fn start(self: &Arc<Self>) -> Result<(), FileTransferError> {
        let handler: Arc<dyn InboundTransferHandler> = self.clone();
        self.transport.set_handler(Arc::downgrade(&handler));
        lock(&self.state)?.running = true;
        Ok(())
    }

    pub(crate) fn is_running(&self) -> bool {
        lock(&self.state)
            .map(|state| state.running)
            .unwrap_or(false)
    }

    pub(crate) fn stop(&self) -> Result<(), FileTransferError> {
        let (transfers, workers, temporary_paths) = {
            let mut state = lock(&self.state)?;
            state.running = false;
            for operation in state.outgoing.values() {
                let _ = operation.commands.send(OperationCommand::Cancel);
            }
            for operation in state.incoming.values_mut() {
                if let Some(decision) = operation.decision.take() {
                    let _ = decision.send(TransferDisposition::Rejected(
                        TransferRejection::Unavailable,
                    ));
                }
            }
            let mut workers = state
                .outgoing
                .values_mut()
                .filter_map(|operation| operation.worker.take())
                .collect::<Vec<_>>();
            workers.append(&mut state.orphan_workers);
            let temporary_paths = state
                .incoming
                .values_mut()
                .filter_map(|operation| operation.session.take())
                .map(|session| session.temporary_path)
                .collect::<Vec<_>>();
            state.outgoing.clear();
            state.incoming.clear();
            let transfers = std::mem::take(&mut state.transfers)
                .into_values()
                .collect::<Vec<_>>();
            (transfers, workers, temporary_paths)
        };
        self.transport.clear_handler();
        for path in temporary_paths {
            let _ = fs::remove_file(path);
        }
        for transfer in transfers {
            self.changed.publish(FileTransferChange::Removed(transfer));
        }
        for worker in workers {
            worker
                .join()
                .map_err(|_| FileTransferError::WorkerStopFailed)?;
        }
        Ok(())
    }

    pub(crate) fn begin(
        self: &Arc<Self>,
        handle_identifier: String,
        config: FileTransferConfig,
    ) -> Result<(), FileTransferError> {
        validate_source(&config)?;
        let transfer_id = FileTransferId::new();
        let transfer = FileTransfer::outgoing(transfer_id.clone(), &config);
        let (commands, receiver) = mpsc::channel();
        {
            let mut state = lock(&self.state)?;
            if !state.running {
                return Err(FileTransferError::ManagerUnavailable);
            }
            if active_transfer_count(&state) >= MAX_ACTIVE_TRANSFERS {
                return Err(FileTransferError::OperationLimit);
            }
            state
                .transfers
                .insert(transfer_id.clone(), transfer.clone());
            state.outgoing.insert(
                handle_identifier.clone(),
                OutgoingOperation {
                    transfer_id: transfer_id.clone(),
                    commands,
                    worker: None,
                },
            );
        }
        self.changed.publish(FileTransferChange::Added(transfer));

        let controller = Arc::clone(self);
        let worker_identifier = handle_identifier.clone();
        let spawn_result = thread::Builder::new()
            .name("continuehere-file-transfer".to_owned())
            .spawn(move || {
                run_outgoing(controller, worker_identifier, transfer_id, config, receiver);
            });
        let worker = match spawn_result {
            Ok(worker) => worker,
            Err(_) => {
                self.remove_for_handle(&handle_identifier);
                return Err(FileTransferError::CommandUnavailable);
            }
        };
        let mut state = lock(&self.state)?;
        match state.outgoing.get_mut(&handle_identifier) {
            Some(operation) => operation.worker = Some(worker),
            None => state.orphan_workers.push(worker),
        }
        Ok(())
    }

    pub(crate) fn cancel(&self, handle_identifier: &str) {
        let changed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let transfer_id = match state.outgoing.get(handle_identifier) {
                Some(operation) => {
                    let _ = operation.commands.send(OperationCommand::Cancel);
                    operation.transfer_id.clone()
                }
                None => return,
            };
            let Some(transfer) = state.transfers.get_mut(&transfer_id) else {
                return;
            };
            if is_terminal(transfer.state()) {
                return;
            }
            transfer.cancel();
            transfer.clone()
        };
        self.changed.publish(FileTransferChange::Updated(changed));
    }

    pub(crate) fn remove_for_handle(&self, handle_identifier: &str) {
        let removed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let Some(mut operation) = state.outgoing.remove(handle_identifier) else {
                return;
            };
            let _ = operation.commands.send(OperationCommand::Cancel);
            if let Some(worker) = operation.worker.take() {
                state.orphan_workers.push(worker);
            }
            state.transfers.remove(&operation.transfer_id)
        };
        if let Some(transfer) = removed {
            self.changed.publish(FileTransferChange::Removed(transfer));
        }
    }

    pub(crate) fn transfer_for_handle(&self, handle_identifier: &str) -> Option<FileTransfer> {
        let state = lock(&self.state).ok()?;
        let transfer_id = &state.outgoing.get(handle_identifier)?.transfer_id;
        state.transfers.get(transfer_id).cloned()
    }

    pub(crate) fn transfers(&self) -> Vec<FileTransfer> {
        lock(&self.state)
            .map(|state| state.transfers.values().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn accept_incoming(
        &self,
        transfer_id: &FileTransferId,
        selected_directory: Option<&Path>,
    ) -> Result<(), FileTransferError> {
        let file_name = {
            let state = lock(&self.state)?;
            let transfer = state
                .transfers
                .get(transfer_id)
                .ok_or(FileTransferError::TransferNotFound)?;
            if transfer.state() != FileTransferState::Offered {
                return Err(FileTransferError::IncomingTransferNotPending);
            }
            transfer.file_name().to_owned()
        };
        let directory = self.resolve_destination_directory(selected_directory)?;
        let destination_path = directory.join(&file_name);
        if destination_path.exists() {
            return Err(FileTransferError::DestinationConflict);
        }
        let temporary_path = directory.join(format!(
            ".continuehere-{}-{}.part",
            transfer_id,
            Uuid::new_v4()
        ));
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary_path)
            .map_err(|_| FileTransferError::InvalidDestinationDirectory)?;
        let changed_and_decision = (|| -> Result<_, FileTransferError> {
            let mut state = lock(&self.state)?;
            let ControllerState {
                transfers,
                incoming,
                ..
            } = &mut *state;
            let operation = incoming
                .get_mut(transfer_id)
                .ok_or(FileTransferError::IncomingTransferNotPending)?;
            let decision = operation
                .decision
                .take()
                .ok_or(FileTransferError::IncomingTransferNotPending)?;
            operation.session = Some(ReceiveSession {
                file,
                hasher: Sha256::new(),
                temporary_path: temporary_path.clone(),
                destination_path: destination_path.clone(),
            });
            let transfer = transfers
                .get_mut(transfer_id)
                .ok_or(FileTransferError::TransferNotFound)?;
            transfer.set_destination(destination_path);
            transfer.set_state(FileTransferState::Transferring);
            Ok((transfer.clone(), decision))
        })();
        let changed_and_decision = match changed_and_decision {
            Ok(value) => value,
            Err(error) => {
                let _ = fs::remove_file(temporary_path);
                return Err(error);
            }
        };
        self.changed
            .publish(FileTransferChange::Updated(changed_and_decision.0));
        if changed_and_decision
            .1
            .send(TransferDisposition::Accepted)
            .is_err()
        {
            self.fail_incoming(transfer_id, FileTransferFailure::Transport);
            return Err(FileTransferError::CommandUnavailable);
        }
        Ok(())
    }

    pub(crate) fn reject_incoming(
        &self,
        transfer_id: &FileTransferId,
    ) -> Result<(), FileTransferError> {
        let (changed, decision) = {
            let mut state = lock(&self.state)?;
            let ControllerState {
                transfers,
                incoming,
                ..
            } = &mut *state;
            let operation = incoming
                .get_mut(transfer_id)
                .ok_or(FileTransferError::TransferNotFound)?;
            let decision = operation
                .decision
                .take()
                .ok_or(FileTransferError::IncomingTransferNotPending)?;
            let transfer = transfers
                .get_mut(transfer_id)
                .ok_or(FileTransferError::TransferNotFound)?;
            if transfer.state() != FileTransferState::Offered {
                return Err(FileTransferError::IncomingTransferNotPending);
            }
            transfer.reject(FileTransferFailure::Declined);
            (transfer.clone(), decision)
        };
        self.changed.publish(FileTransferChange::Updated(changed));
        let _ = decision.send(TransferDisposition::Rejected(TransferRejection::Declined));
        Ok(())
    }

    pub(crate) fn remove(&self, transfer_id: &FileTransferId) -> Result<(), FileTransferError> {
        let (removed, temporary_path) = {
            let mut state = lock(&self.state)?;
            if state
                .outgoing
                .values()
                .any(|operation| &operation.transfer_id == transfer_id)
            {
                return Err(FileTransferError::InvalidHandleState);
            }
            let temporary_path = state
                .incoming
                .remove(transfer_id)
                .and_then(|mut operation| operation.session.take())
                .map(|session| session.temporary_path);
            let removed = state
                .transfers
                .remove(transfer_id)
                .ok_or(FileTransferError::TransferNotFound)?;
            (removed, temporary_path)
        };
        if let Some(path) = temporary_path {
            let _ = fs::remove_file(path);
        }
        self.changed.publish(FileTransferChange::Removed(removed));
        Ok(())
    }

    fn resolve_destination_directory(
        &self,
        selected_directory: Option<&Path>,
    ) -> Result<PathBuf, FileTransferError> {
        let directory = match selected_directory {
            Some(directory) => directory.to_path_buf(),
            None => self.directories.default_transfer_directory(),
        };
        if !directory.is_absolute() || !directory.is_dir() {
            return Err(FileTransferError::InvalidDestinationDirectory);
        }
        Ok(directory)
    }

    fn update_outgoing_state(&self, handle_identifier: &str, state_value: FileTransferState) {
        let changed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let transfer_id = match state.outgoing.get(handle_identifier) {
                Some(operation) => operation.transfer_id.clone(),
                None => return,
            };
            let Some(transfer) = state.transfers.get_mut(&transfer_id) else {
                return;
            };
            if transfer.state() == FileTransferState::Cancelled {
                return;
            }
            transfer.set_state(state_value);
            transfer.clone()
        };
        self.changed.publish(FileTransferChange::Updated(changed));
    }

    fn update_outgoing_progress(&self, handle_identifier: &str, transferred_bytes: u64) {
        let changed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let transfer_id = match state.outgoing.get(handle_identifier) {
                Some(operation) => operation.transfer_id.clone(),
                None => return,
            };
            let Some(transfer) = state.transfers.get_mut(&transfer_id) else {
                return;
            };
            if transfer.state() != FileTransferState::Transferring {
                return;
            }
            transfer.set_progress(transferred_bytes);
            transfer.clone()
        };
        self.changed.publish(FileTransferChange::Updated(changed));
    }

    fn finish_outgoing(&self, handle_identifier: &str, result: OperationResult) {
        let changed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let transfer_id = match state.outgoing.get(handle_identifier) {
                Some(operation) => operation.transfer_id.clone(),
                None => return,
            };
            let Some(transfer) = state.transfers.get_mut(&transfer_id) else {
                return;
            };
            if transfer.state() == FileTransferState::Cancelled {
                return;
            }
            match result {
                OperationResult::Completed => {
                    transfer.set_progress(transfer.file_size());
                    transfer.set_state(FileTransferState::Completed);
                }
                OperationResult::Rejected(failure) => transfer.reject(failure),
                OperationResult::Failed(failure) => transfer.fail(failure),
            }
            transfer.clone()
        };
        self.changed.publish(FileTransferChange::Updated(changed));
    }

    fn receive_offer(
        &self,
        transfer_id: FileTransferId,
        sender_device_id: DeviceId,
        file_name: String,
        file_size: u64,
    ) -> TransferDisposition {
        if validate_file_name(&file_name).is_err() || file_size > MAX_FILE_SIZE {
            return TransferDisposition::Rejected(TransferRejection::Invalid);
        }
        let (decision, result) = mpsc::channel();
        let transfer = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => {
                    return TransferDisposition::Rejected(TransferRejection::Unavailable);
                }
            };
            if !state.running {
                return TransferDisposition::Rejected(TransferRejection::Unavailable);
            }
            if state.transfers.contains_key(&transfer_id) {
                return TransferDisposition::Rejected(TransferRejection::Invalid);
            }
            if state.incoming.len() >= MAX_INCOMING_OFFERS
                || active_transfer_count(&state) >= MAX_ACTIVE_TRANSFERS
            {
                return TransferDisposition::Rejected(TransferRejection::Busy);
            }
            let transfer = FileTransfer::incoming(
                transfer_id.clone(),
                sender_device_id.clone(),
                file_name,
                file_size,
            );
            state
                .transfers
                .insert(transfer_id.clone(), transfer.clone());
            state.incoming.insert(
                transfer_id.clone(),
                IncomingOperation {
                    sender_device_id,
                    decision: Some(decision),
                    session: None,
                },
            );
            transfer
        };
        self.changed.publish(FileTransferChange::Added(transfer));
        match result.recv_timeout(OFFER_TIMEOUT) {
            Ok(disposition) => disposition,
            Err(_) => {
                self.fail_incoming(&transfer_id, FileTransferFailure::TimedOut);
                TransferDisposition::Rejected(TransferRejection::Unavailable)
            }
        }
    }

    fn receive_chunk(
        &self,
        transfer_id: &FileTransferId,
        sender_device_id: &DeviceId,
        offset: u64,
        bytes: &[u8],
    ) -> TransferDisposition {
        let result = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => {
                    return TransferDisposition::Rejected(TransferRejection::Unavailable);
                }
            };
            let ControllerState {
                transfers,
                incoming,
                ..
            } = &mut *state;
            let current_size = match transfers.get(transfer_id) {
                Some(transfer) => transfer.transferred_bytes(),
                None => return TransferDisposition::Rejected(TransferRejection::Invalid),
            };
            let file_size = transfers
                .get(transfer_id)
                .map(FileTransfer::file_size)
                .unwrap_or(0);
            let operation = match incoming.get_mut(transfer_id) {
                Some(operation) if &operation.sender_device_id == sender_device_id => operation,
                _ => return TransferDisposition::Rejected(TransferRejection::Invalid),
            };
            let session = match operation.session.as_mut() {
                Some(session) => session,
                None => return TransferDisposition::Rejected(TransferRejection::Invalid),
            };
            let new_size = match current_size.checked_add(bytes.len() as u64) {
                Some(size) => size,
                None => return TransferDisposition::Rejected(TransferRejection::Invalid),
            };
            if bytes.is_empty()
                || bytes.len() > TRANSFER_CHUNK_SIZE
                || offset != current_size
                || new_size > file_size
            {
                return TransferDisposition::Rejected(TransferRejection::Invalid);
            }
            if session.file.write_all(bytes).is_err() {
                Err(FileTransferFailure::FileSystem)
            } else {
                session.hasher.update(bytes);
                let transfer = transfers
                    .get_mut(transfer_id)
                    .expect("validated transfer should remain available");
                transfer.set_progress(new_size);
                Ok(transfer.clone())
            }
        };
        match result {
            Ok(changed) => {
                self.changed.publish(FileTransferChange::Updated(changed));
                TransferDisposition::Accepted
            }
            Err(failure) => {
                self.fail_incoming(transfer_id, failure);
                TransferDisposition::Rejected(TransferRejection::FileSystem)
            }
        }
    }

    fn receive_finish(
        &self,
        transfer_id: &FileTransferId,
        sender_device_id: &DeviceId,
        digest: [u8; 32],
    ) -> TransferDisposition {
        let (mut session, verifying) = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => {
                    return TransferDisposition::Rejected(TransferRejection::Unavailable);
                }
            };
            let valid_size = state
                .transfers
                .get(transfer_id)
                .is_some_and(|transfer| transfer.transferred_bytes() == transfer.file_size());
            if !valid_size {
                return TransferDisposition::Rejected(TransferRejection::Integrity);
            }
            let session = match state.incoming.get_mut(transfer_id) {
                Some(operation) if &operation.sender_device_id == sender_device_id => {
                    operation.session.take()
                }
                _ => return TransferDisposition::Rejected(TransferRejection::Integrity),
            };
            let Some(session) = session else {
                return TransferDisposition::Rejected(TransferRejection::Invalid);
            };
            let transfer = state
                .transfers
                .get_mut(transfer_id)
                .expect("validated transfer should remain available");
            transfer.set_state(FileTransferState::Verifying);
            (session, transfer.clone())
        };
        self.changed.publish(FileTransferChange::Updated(verifying));

        let calculated: [u8; 32] = session.hasher.clone().finalize().into();
        if calculated != digest {
            let _ = fs::remove_file(&session.temporary_path);
            self.fail_incoming(transfer_id, FileTransferFailure::Integrity);
            return TransferDisposition::Rejected(TransferRejection::Integrity);
        }
        if session.file.flush().is_err() || session.file.sync_all().is_err() {
            let _ = fs::remove_file(&session.temporary_path);
            self.fail_incoming(transfer_id, FileTransferFailure::FileSystem);
            return TransferDisposition::Rejected(TransferRejection::FileSystem);
        }
        drop(session.file);
        if fs::hard_link(&session.temporary_path, &session.destination_path).is_err() {
            let failure = if session.destination_path.exists() {
                FileTransferFailure::DestinationConflict
            } else {
                FileTransferFailure::FileSystem
            };
            let rejection = if failure == FileTransferFailure::DestinationConflict {
                TransferRejection::DestinationConflict
            } else {
                TransferRejection::FileSystem
            };
            let _ = fs::remove_file(&session.temporary_path);
            self.fail_incoming(transfer_id, failure);
            return TransferDisposition::Rejected(rejection);
        }
        let _ = fs::remove_file(&session.temporary_path);
        let changed = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => {
                    return TransferDisposition::Rejected(TransferRejection::Unavailable);
                }
            };
            let Some(transfer) = state.transfers.get_mut(transfer_id) else {
                return TransferDisposition::Rejected(TransferRejection::Unavailable);
            };
            transfer.set_state(FileTransferState::Completed);
            transfer.clone()
        };
        self.changed.publish(FileTransferChange::Updated(changed));
        TransferDisposition::Accepted
    }

    fn receive_cancel(
        &self,
        transfer_id: &FileTransferId,
        sender_device_id: &DeviceId,
    ) -> TransferDisposition {
        let (changed, temporary_path, decision) = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => {
                    return TransferDisposition::Rejected(TransferRejection::Unavailable);
                }
            };
            let ControllerState {
                transfers,
                incoming,
                ..
            } = &mut *state;
            let operation = match incoming.get_mut(transfer_id) {
                Some(operation) if &operation.sender_device_id == sender_device_id => operation,
                _ => return TransferDisposition::Rejected(TransferRejection::Invalid),
            };
            let decision = operation.decision.take();
            let temporary_path = operation
                .session
                .take()
                .map(|session| session.temporary_path);
            let Some(transfer) = transfers.get_mut(transfer_id) else {
                return TransferDisposition::Rejected(TransferRejection::Invalid);
            };
            if !is_terminal(transfer.state()) {
                transfer.cancel();
            }
            (transfer.clone(), temporary_path, decision)
        };
        if let Some(path) = temporary_path {
            let _ = fs::remove_file(path);
        }
        if let Some(decision) = decision {
            let _ = decision.send(TransferDisposition::Rejected(TransferRejection::Declined));
        }
        self.changed.publish(FileTransferChange::Updated(changed));
        TransferDisposition::Accepted
    }

    fn fail_incoming(&self, transfer_id: &FileTransferId, failure: FileTransferFailure) {
        let result = {
            let mut state = match lock(&self.state) {
                Ok(state) => state,
                Err(_) => return,
            };
            let temporary_path = state
                .incoming
                .get_mut(transfer_id)
                .and_then(|operation| operation.session.take())
                .map(|session| session.temporary_path);
            let changed = state.transfers.get_mut(transfer_id).map(|transfer| {
                if !is_terminal(transfer.state()) {
                    transfer.fail(failure);
                }
                transfer.clone()
            });
            (changed, temporary_path)
        };
        if let Some(path) = result.1 {
            let _ = fs::remove_file(path);
        }
        if let Some(changed) = result.0 {
            self.changed.publish(FileTransferChange::Updated(changed));
        }
    }
}

impl InboundTransferHandler for FileTransferController {
    fn receive(&self, transfer: InboundTransfer) -> TransferDisposition {
        let (transfer_id, sender_device_id, message) = transfer.into_parts();
        let transfer_id = FileTransferId::from_bytes(transfer_id);
        match message {
            TransferTransportMessage::Offer {
                file_name,
                file_size,
            } => self.receive_offer(transfer_id, sender_device_id, file_name, file_size),
            TransferTransportMessage::Chunk { offset, bytes } => {
                self.receive_chunk(&transfer_id, &sender_device_id, offset, &bytes)
            }
            TransferTransportMessage::Finish { digest } => {
                self.receive_finish(&transfer_id, &sender_device_id, digest)
            }
            TransferTransportMessage::Cancel => {
                self.receive_cancel(&transfer_id, &sender_device_id)
            }
        }
    }
}

enum OperationResult {
    Completed,
    Rejected(FileTransferFailure),
    Failed(FileTransferFailure),
}

fn run_outgoing(
    controller: Arc<FileTransferController>,
    handle_identifier: String,
    transfer_id: FileTransferId,
    config: FileTransferConfig,
    commands: mpsc::Receiver<OperationCommand>,
) {
    controller.update_outgoing_state(&handle_identifier, FileTransferState::WaitingForAcceptance);
    let offer = TransferTransportMessage::Offer {
        file_name: config.file_name.clone(),
        file_size: config.file_size,
    };
    match send_and_wait(
        &controller,
        &config.device_id,
        &transfer_id,
        offer,
        &commands,
    ) {
        WaitResult::Accepted => {}
        WaitResult::Rejected(reason) => {
            controller.finish_outgoing(
                &handle_identifier,
                OperationResult::Rejected(map_rejection(reason)),
            );
            return;
        }
        WaitResult::Failed(error) => {
            controller.finish_outgoing(&handle_identifier, map_transport_error(error));
            return;
        }
        WaitResult::Cancelled => {
            send_cancel(&controller, &config.device_id, &transfer_id);
            return;
        }
    }
    controller.update_outgoing_state(&handle_identifier, FileTransferState::Transferring);

    let mut file = match File::open(&config.source) {
        Ok(file) => file,
        Err(_) => {
            controller.finish_outgoing(
                &handle_identifier,
                OperationResult::Failed(FileTransferFailure::FileSystem),
            );
            send_cancel(&controller, &config.device_id, &transfer_id);
            return;
        }
    };
    let mut hasher = Sha256::new();
    let mut transferred = 0_u64;
    let mut buffer = vec![0_u8; TRANSFER_CHUNK_SIZE];
    loop {
        let read = match file.read(&mut buffer) {
            Ok(read) => read,
            Err(_) => {
                controller.finish_outgoing(
                    &handle_identifier,
                    OperationResult::Failed(FileTransferFailure::FileSystem),
                );
                send_cancel(&controller, &config.device_id, &transfer_id);
                return;
            }
        };
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        let message = TransferTransportMessage::Chunk {
            offset: transferred,
            bytes: chunk.to_vec(),
        };
        match send_and_wait(
            &controller,
            &config.device_id,
            &transfer_id,
            message,
            &commands,
        ) {
            WaitResult::Accepted => {}
            WaitResult::Rejected(reason) => {
                controller.finish_outgoing(
                    &handle_identifier,
                    OperationResult::Rejected(map_rejection(reason)),
                );
                send_cancel(&controller, &config.device_id, &transfer_id);
                return;
            }
            WaitResult::Failed(error) => {
                controller.finish_outgoing(&handle_identifier, map_transport_error(error));
                send_cancel(&controller, &config.device_id, &transfer_id);
                return;
            }
            WaitResult::Cancelled => {
                send_cancel(&controller, &config.device_id, &transfer_id);
                return;
            }
        }
        hasher.update(chunk);
        transferred += read as u64;
        controller.update_outgoing_progress(&handle_identifier, transferred);
    }
    if transferred != config.file_size {
        controller.finish_outgoing(
            &handle_identifier,
            OperationResult::Failed(FileTransferFailure::Invalid),
        );
        send_cancel(&controller, &config.device_id, &transfer_id);
        return;
    }
    let digest: [u8; 32] = hasher.finalize().into();
    match send_and_wait(
        &controller,
        &config.device_id,
        &transfer_id,
        TransferTransportMessage::Finish { digest },
        &commands,
    ) {
        WaitResult::Accepted => {
            controller.finish_outgoing(&handle_identifier, OperationResult::Completed);
        }
        WaitResult::Rejected(reason) => controller.finish_outgoing(
            &handle_identifier,
            OperationResult::Rejected(map_rejection(reason)),
        ),
        WaitResult::Failed(error) => {
            controller.finish_outgoing(&handle_identifier, map_transport_error(error));
        }
        WaitResult::Cancelled => send_cancel(&controller, &config.device_id, &transfer_id),
    }
}

enum WaitResult {
    Accepted,
    Rejected(TransferRejection),
    Failed(TransportError),
    Cancelled,
}

fn send_and_wait(
    controller: &FileTransferController,
    device_id: &DeviceId,
    transfer_id: &FileTransferId,
    message: TransferTransportMessage,
    commands: &mpsc::Receiver<OperationCommand>,
) -> WaitResult {
    let response = match controller
        .transport
        .send(device_id.clone(), transfer_id.bytes(), message)
    {
        Ok(response) => response,
        Err(error) => return WaitResult::Failed(error),
    };
    loop {
        match response.try_recv() {
            Ok(Ok(TransferDisposition::Accepted)) => return WaitResult::Accepted,
            Ok(Ok(TransferDisposition::Rejected(reason))) => {
                return WaitResult::Rejected(reason);
            }
            Ok(Err(error)) => return WaitResult::Failed(error),
            Err(mpsc::TryRecvError::Disconnected) => {
                return WaitResult::Failed(TransportError::ConnectionFailed);
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        match commands.recv_timeout(RESULT_POLL_INTERVAL) {
            Ok(OperationCommand::Cancel) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                return WaitResult::Cancelled;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

fn send_cancel(
    controller: &FileTransferController,
    device_id: &DeviceId,
    transfer_id: &FileTransferId,
) {
    let _ = controller.transport.send(
        device_id.clone(),
        transfer_id.bytes(),
        TransferTransportMessage::Cancel,
    );
}

fn validate_source(config: &FileTransferConfig) -> Result<(), FileTransferError> {
    let metadata = config
        .source
        .symlink_metadata()
        .map_err(|_| FileTransferError::InvalidSourceFile)?;
    if !metadata.file_type().is_file() || metadata.len() != config.file_size {
        return Err(FileTransferError::InvalidSourceFile);
    }
    Ok(())
}

fn active_transfer_count(state: &ControllerState) -> usize {
    state
        .transfers
        .values()
        .filter(|transfer| !is_terminal(transfer.state()))
        .count()
}

fn is_terminal(state: FileTransferState) -> bool {
    matches!(
        state,
        FileTransferState::Completed
            | FileTransferState::Rejected
            | FileTransferState::Cancelled
            | FileTransferState::Failed
    )
}

fn map_rejection(reason: TransferRejection) -> FileTransferFailure {
    match reason {
        TransferRejection::Invalid => FileTransferFailure::Invalid,
        TransferRejection::Busy => FileTransferFailure::Busy,
        TransferRejection::Unavailable => FileTransferFailure::Unsupported,
        TransferRejection::Declined => FileTransferFailure::Declined,
        TransferRejection::DestinationConflict => FileTransferFailure::DestinationConflict,
        TransferRejection::Integrity => FileTransferFailure::Integrity,
        TransferRejection::FileSystem => FileTransferFailure::FileSystem,
    }
}

fn map_transport_error(error: TransportError) -> OperationResult {
    match error {
        TransportError::NotConnected => OperationResult::Failed(FileTransferFailure::NotConnected),
        TransportError::UnsupportedCapability => {
            OperationResult::Rejected(FileTransferFailure::Unsupported)
        }
        TransportError::TimedOut => OperationResult::Failed(FileTransferFailure::TimedOut),
        _ => OperationResult::Failed(FileTransferFailure::Transport),
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, FileTransferError> {
    value
        .lock()
        .map_err(|_| FileTransferError::SynchronizationFailed)
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc, thread, time::Duration};

    use sha2::{Digest, Sha256};
    use tempfile::tempdir;

    use super::FileTransferController;
    use crate::{
        models::DeviceId,
        settings::SettingsManager,
        transfer::{FileTransferChangedEvent, FileTransferState},
        transport::{
            InboundTransfer, InboundTransferHandler, TransferDisposition,
            TransferTransportCapability, TransferTransportMessage,
        },
    };

    #[test]
    fn incoming_file_commits_only_after_acceptance_and_integrity_check() {
        let project = tempdir().expect("project directory should be available");
        let destination = project.path().join("received");
        fs::create_dir(&destination).expect("destination should be created");
        let settings =
            SettingsManager::new(project.path().to_path_buf()).expect("settings should be created");
        settings
            .directories()
            .set_default_transfer_directory(destination.clone())
            .expect("destination should save");
        let controller = FileTransferController::new(
            TransferTransportCapability::new(),
            settings.directories().shared(),
            FileTransferChangedEvent::default(),
        );
        controller.start().expect("controller should start");
        let sender = DeviceId::new("sender").expect("device identifier should be valid");
        let transfer_bytes = [5_u8; 64 * 1024];
        let transfer_id = [7_u8; 16];
        let receiving = Arc::clone(&controller);
        let receiving_sender = sender.clone();
        let offer = thread::spawn(move || {
            receiving.receive(InboundTransfer::new(
                transfer_id,
                receiving_sender,
                TransferTransportMessage::Offer {
                    file_name: "example.bin".to_owned(),
                    file_size: transfer_bytes.len() as u64,
                },
            ))
        });
        let incoming_id = wait_for_offer(&controller);
        assert!(!destination.join("example.bin").exists());
        controller
            .accept_incoming(&incoming_id, None)
            .expect("incoming transfer should be accepted");
        assert_eq!(
            offer.join().expect("offer worker should stop"),
            TransferDisposition::Accepted
        );

        for (index, bytes) in transfer_bytes.chunks(32 * 1024).enumerate() {
            assert_eq!(
                controller.receive(InboundTransfer::new(
                    transfer_id,
                    sender.clone(),
                    TransferTransportMessage::Chunk {
                        offset: (index * 32 * 1024) as u64,
                        bytes: bytes.to_vec(),
                    },
                )),
                TransferDisposition::Accepted
            );
        }
        let digest: [u8; 32] = Sha256::digest(transfer_bytes).into();
        assert_eq!(
            controller.receive(InboundTransfer::new(
                transfer_id,
                sender,
                TransferTransportMessage::Finish { digest },
            )),
            TransferDisposition::Accepted
        );
        assert_eq!(
            fs::read(destination.join("example.bin")).expect("file should be committed"),
            transfer_bytes
        );
        assert_eq!(
            controller.transfers()[0].state(),
            FileTransferState::Completed
        );
        controller.stop().expect("controller should stop");
    }

    fn wait_for_offer(controller: &FileTransferController) -> crate::transfer::FileTransferId {
        for _ in 0..100 {
            if let Some(transfer) = controller.transfers().first() {
                return transfer.id().clone();
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("incoming offer should become available");
    }
}

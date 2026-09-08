use super::{
    ActivityChangedDelegate, ActivityChangedSubscription, ActivityError, ActivitySnapshot,
    controller::ActivityController, event::ActivityChangedEvent, retry::RetryController,
};
use crate::{
    core::{error::ModuleError, module::Module},
    handoff::{
        HandoffCapability, HandoffChangedDelegate, HandoffChangedSubscription,
        IncomingHandoffChangedDelegate, IncomingHandoffChangedSubscription,
    },
    pairing::TrustedPeerLookup,
    transfer::{
        FileTransferCapability, FileTransferChangedDelegate, FileTransferChangedSubscription,
    },
    transport::{ConnectionCapability, ConnectionChangedDelegate, ConnectionChangedSubscription},
};
use async_trait::async_trait;
use std::{path::PathBuf, sync::Arc};

pub struct ActivityManager {
    controller: Arc<ActivityController>,
    retry: Arc<RetryController>,
    changed: ActivityChangedEvent,
    _handoff: HandoffChangedSubscription,
    _incoming: IncomingHandoffChangedSubscription,
    _transfer: FileTransferChangedSubscription,
    _connection: ConnectionChangedSubscription,
}

impl ActivityManager {
    pub(crate) fn new(
        directory: PathBuf,
        handoff: HandoffCapability,
        transfer: FileTransferCapability,
        connections: ConnectionCapability,
        trusted: TrustedPeerLookup,
    ) -> Self {
        let changed = ActivityChangedEvent::default();
        let controller = ActivityController::new(directory, trusted.clone(), changed.clone());
        let retry = RetryController::new(
            handoff.clone(),
            transfer.clone(),
            connections.clone(),
            trusted,
        );
        let history = Arc::clone(&controller);
        let retries = Arc::clone(&retry);
        let outgoing = handoff.on_changed(HandoffChangedDelegate::new(move |change| {
            history.handoff(change);
            retries.changed();
        }));
        let history = Arc::clone(&controller);
        let incoming =
            handoff.on_incoming_changed(IncomingHandoffChangedDelegate::new(move |change| {
                history.incoming(change)
            }));
        let history = Arc::clone(&controller);
        let retries = Arc::clone(&retry);
        let transfers = transfer.on_changed(FileTransferChangedDelegate::new(move |change| {
            history.transfer(change);
            retries.changed();
        }));
        let history = Arc::clone(&controller);
        let connection = connections.on_changed(ConnectionChangedDelegate::new(move |change| {
            history.connection(change)
        }));
        Self {
            controller,
            retry,
            changed,
            _handoff: outgoing,
            _incoming: incoming,
            _transfer: transfers,
            _connection: connection,
        }
    }

    pub fn activities(&self) -> Result<ActivitySnapshot, ActivityError> {
        self.controller.snapshot()
    }
    pub fn retry(&self, activity_id: &str) -> Result<(), ActivityError> {
        self.retry
            .retry(self.controller.find(activity_id)?, &self.controller)
    }
    pub fn remove(&self, activity_id: &str) -> Result<(), ActivityError> {
        self.controller.remove(Some(activity_id))
    }
    pub fn clear(&self) -> Result<(), ActivityError> {
        self.controller.remove(None)
    }
    pub fn on_changed(&self, delegate: ActivityChangedDelegate) -> ActivityChangedSubscription {
        self.changed.subscribe(delegate)
    }
}

#[async_trait]
impl Module for ActivityManager {
    fn name(&self) -> &'static str {
        "activity"
    }
    async fn start(&mut self) -> Result<(), ModuleError> {
        self.controller.start()?;
        if let Err(error) = self.retry.start() {
            let _ = self.controller.stop();
            return Err(Box::new(error));
        }
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), ModuleError> {
        let retry_error = self.retry.stop().err();
        let store_error = self.controller.stop().err();
        match retry_error.or(store_error) {
            Some(error) => Err(Box::new(error)),
            None => Ok(()),
        }
    }
}

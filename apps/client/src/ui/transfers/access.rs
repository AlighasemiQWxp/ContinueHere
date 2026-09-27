use std::rc::Rc;

use continuehere::{
    ContinueHere, FileTransferManager, HandoffManager, LocalizationManager, TransportManager,
};

pub(in crate::ui) struct TransfersAccess {
    core: Rc<ContinueHere>,
}

impl TransfersAccess {
    pub(in crate::ui) fn new(core: Rc<ContinueHere>) -> Self {
        Self { core }
    }

    pub(super) fn file_transfers(&self) -> &FileTransferManager {
        self.core.file_transfers()
    }

    pub(super) fn handoff(&self) -> &HandoffManager {
        self.core.handoff()
    }

    pub(super) fn localization(&self) -> &LocalizationManager {
        self.core.localization()
    }

    pub(super) fn transport(&self) -> &TransportManager {
        self.core.transport()
    }
}

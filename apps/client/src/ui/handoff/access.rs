use std::rc::Rc;

use continuehere::{ActivityManager, ContinueHere, HandoffManager, TransportManager};

pub(in crate::ui) struct HandoffAccess {
    core: Rc<ContinueHere>,
}

impl HandoffAccess {
    pub(in crate::ui) fn new(core: Rc<ContinueHere>) -> Self {
        Self { core }
    }

    pub(super) fn activity(&self) -> &ActivityManager {
        self.core.activity()
    }

    pub(super) fn handoff(&self) -> &HandoffManager {
        self.core.handoff()
    }

    pub(super) fn transport(&self) -> &TransportManager {
        self.core.transport()
    }
}

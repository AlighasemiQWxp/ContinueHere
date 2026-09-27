use std::rc::Rc;

use continuehere::{ActivityManager, ContinueHere, LocalizationSettings, TransportManager};

pub(in crate::ui) struct HistoryAccess {
    core: Rc<ContinueHere>,
}

impl HistoryAccess {
    pub(in crate::ui) fn new(core: Rc<ContinueHere>) -> Self {
        Self { core }
    }

    pub(super) fn activity(&self) -> &ActivityManager {
        self.core.activity()
    }

    pub(super) fn transport(&self) -> &TransportManager {
        self.core.transport()
    }

    pub(super) fn localization(&self) -> &LocalizationSettings {
        self.core.settings().localization()
    }
}

use std::rc::Rc;

use continuehere::{ContinueHere, LocalizationManager, PairingManager};

pub(in crate::ui) struct PairingAccess {
    core: Rc<ContinueHere>,
}

impl PairingAccess {
    pub(in crate::ui) fn new(core: Rc<ContinueHere>) -> Self {
        Self { core }
    }

    pub(super) fn pairing(&self) -> &PairingManager {
        self.core.pairing()
    }

    pub(super) fn localization(&self) -> &LocalizationManager {
        self.core.localization()
    }
}

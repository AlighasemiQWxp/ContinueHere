use std::rc::Rc;

use continuehere::{
    ContinueHere, DeviceManager, DiscoveryManager, PairingManager, TransportManager,
};

#[derive(Clone)]
pub(in crate::ui) struct DevicesAccess {
    core: Rc<ContinueHere>,
}

impl DevicesAccess {
    pub(in crate::ui) fn new(core: Rc<ContinueHere>) -> Self {
        Self { core }
    }

    pub(super) fn devices(&self) -> &DeviceManager {
        self.core.devices()
    }

    pub(super) fn discovery(&self) -> &DiscoveryManager {
        self.core.discovery()
    }

    pub(super) fn pairing(&self) -> &PairingManager {
        self.core.pairing()
    }

    pub(super) fn transport(&self) -> &TransportManager {
        self.core.transport()
    }
}

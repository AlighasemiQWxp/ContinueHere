mod access;

pub(in crate::ui) use access::PairingAccess;

use std::{cell::RefCell, rc::Rc};

use continuehere::{
    LanguageChangedDelegate, LanguageChangedSubscription, PairingHandle, PairingMode, PairingRole,
    PairingSession, PairingSessionChangedDelegate, PairingSessionChangedSubscription, PairingState,
};
use slint::ComponentHandle;

use super::{
    MainWindow,
    devices::PairingDevices,
    shared::{EventTarget, UiResult, notify, show_result, text},
};

pub(super) struct PairingUiController {
    access: PairingAccess,
    devices: PairingDevices,
    handle: Option<PairingHandle>,
    next_handle: u64,
    last_session: Option<continuehere::PairingSession>,
    _changed: PairingSessionChangedSubscription,
    _language_changed: LanguageChangedSubscription,
}

#[derive(Clone, Copy)]
enum PairingAction {
    Receive,
    Pair,
    Approve,
    Reject,
    Cancel,
}

impl PairingAction {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "receive" => Some(Self::Receive),
            "pair" => Some(Self::Pair),
            "approve" => Some(Self::Approve),
            "reject" => Some(Self::Reject),
            "cancel" => Some(Self::Cancel),
            _ => None,
        }
    }
}

impl PairingUiController {
    pub(super) fn start(
        access: PairingAccess,
        devices: PairingDevices,
        window: &MainWindow,
    ) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let language_target = target.clone();
        let changed = access
            .pairing()
            .on_session_changed(PairingSessionChangedDelegate::new(move |_| {
                target.dispatch(|window| {
                    window.invoke_refresh_pairing_requested();
                    notify(&window, 0);
                });
            }));
        let language_changed =
            access
                .localization()
                .on_language_changed(LanguageChangedDelegate::new(move |_| {
                    language_target.dispatch(|window| window.invoke_refresh_pairing_requested());
                }));
        let controller = Rc::new(RefCell::new(Self {
            access,
            devices,
            handle: None,
            next_handle: 0,
            last_session: None,
            _changed: changed,
            _language_changed: language_changed,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_pairing_action(move |action, endpoint| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = match PairingAction::parse(action.as_str()) {
                    Some(action) => controller.borrow_mut().act(action, endpoint.as_str()),
                    None => Err("Unknown pairing action.".into()),
                };
                show_result(&window, result);
                controller.borrow_mut().refresh(&window);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_refresh_pairing_requested(move || {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                controller.borrow_mut().refresh(&window);
            }
        });
        let result = controller.borrow_mut().ensure_receive();
        show_result(window, result);
        controller.borrow_mut().refresh(window);
        controller
    }

    fn act(&mut self, action: PairingAction, endpoint: &str) -> UiResult {
        match action {
            PairingAction::Receive => self.ensure_receive()?,
            PairingAction::Pair => {
                let previous = self.handle.take();
                self.last_session = None;
                let result = self.start_operation(PairingMode::Initiate(endpoint.parse()?));
                drop(previous);
                result?;
            }
            PairingAction::Approve => self
                .handle
                .as_ref()
                .ok_or("No active pairing session.")?
                .approve()?,
            PairingAction::Reject => self
                .handle
                .as_ref()
                .ok_or("No active pairing session.")?
                .reject()?,
            PairingAction::Cancel => {
                let previous = self.handle.take();
                self.last_session = None;
                let result = self.ensure_receive();
                drop(previous);
                result?;
            }
        }
        Ok(())
    }

    fn ensure_receive(&mut self) -> UiResult {
        if self.handle.is_some() {
            return Ok(());
        }
        self.start_operation(PairingMode::Receive)
    }

    fn start_operation(&mut self, mode: PairingMode) -> UiResult {
        self.next_handle += 1;
        let handle = self
            .access
            .pairing()
            .get_handle(&format!("ui.pairing.{}", self.next_handle))?;
        handle.configure(mode)?;
        handle.use_handle()?;
        self.handle = Some(handle);
        Ok(())
    }

    fn refresh(&mut self, window: &MainWindow) {
        let rtl = window.get_rtl();
        window.set_pairing_verifiable(false);
        window.set_pairing_code("".into());
        let current = self.handle.as_ref().and_then(PairingHandle::session);
        if let Some(session) = current.as_ref().filter(|session| terminal(session.state())) {
            let connect_request = if session.state() == PairingState::Trusted
                && session.role() == PairingRole::Initiator
            {
                session
                    .peer_device_id()
                    .zip(session.connection_endpoint())
                    .map(|(device_id, endpoint)| (device_id.clone(), endpoint.clone()))
            } else {
                None
            };
            if session.role() != PairingRole::Receiver || session.peer_device_id().is_some() {
                self.last_session = Some(session.clone());
            }
            let previous = self.handle.take();
            if let Err(error) = self.ensure_receive() {
                window.invoke_show_error_requested(error.to_string().into());
            }
            drop(previous);
            if let Some((device_id, endpoint)) = connect_request {
                self.devices.connect(device_id, endpoint);
            }
        } else if self.handle.is_none()
            && let Err(error) = self.ensure_receive()
        {
            window.invoke_show_error_requested(error.to_string().into());
        }
        let active_session = self
            .handle
            .as_ref()
            .and_then(PairingHandle::session)
            .filter(|session| !idle_receiver(session));
        window.set_pairing_active(active_session.is_some());
        let session = active_session.or_else(|| self.last_session.clone());
        if let Some(session) = session {
            let state = match session.state() {
                PairingState::Connecting => text(rtl, "Connecting", "در حال اتصال"),
                PairingState::ExchangingIdentity => {
                    text(rtl, "Exchanging identity", "در حال تبادل هویت")
                }
                PairingState::AwaitingVerification => text(
                    rtl,
                    "Compare this code on both devices before approving",
                    "پیش از تأیید، این کد را در هر دو دستگاه مقایسه کنید",
                ),
                PairingState::PersistingTrust => text(
                    rtl,
                    "Saving trusted device",
                    "در حال ذخیره دستگاه مورد اعتماد",
                ),
                PairingState::Trusted => text(rtl, "Paired", "جفت شد"),
                PairingState::Rejected => text(rtl, "Rejected", "رد شد"),
                PairingState::Cancelled => text(rtl, "Cancelled", "لغو شد"),
                PairingState::Expired => text(rtl, "Expired", "منقضی شد"),
                _ => text(rtl, "Pairing failed", "جفت‌سازی ناموفق بود"),
            };
            window.set_pairing_status(
                format!("{}\n{}", session.peer_display_name().unwrap_or(""), state).into(),
            );
            if let Some(verification) = session.verification() {
                window.set_pairing_code(verification.manual_code().into());
                window.set_pairing_verifiable(true);
            }
        } else if self.handle.is_some() {
            window.set_pairing_status(text(rtl, "Ready for pairing", "آماده جفت‌سازی").into());
        } else {
            window.set_pairing_status("".into());
        }
        match self.access.pairing().listening_endpoint() {
            Ok(endpoint) => window.set_pairing_endpoint(endpoint.port().to_string().into()),
            Err(_) => window.set_pairing_endpoint("".into()),
        }
        self.devices.refresh_network();
    }
}

fn idle_receiver(session: &PairingSession) -> bool {
    session.role() == PairingRole::Receiver
        && session.state() == PairingState::Connecting
        && session.peer_device_id().is_none()
}

fn terminal(state: PairingState) -> bool {
    matches!(
        state,
        PairingState::Trusted
            | PairingState::Rejected
            | PairingState::Cancelled
            | PairingState::Expired
            | PairingState::Failed
    )
}

#[cfg(test)]
mod tests {
    use super::terminal;
    use continuehere::PairingState;

    #[test]
    fn terminal_pairing_states_are_rearmed() {
        assert!(terminal(PairingState::Trusted));
        assert!(terminal(PairingState::Rejected));
        assert!(terminal(PairingState::Cancelled));
        assert!(terminal(PairingState::Expired));
        assert!(terminal(PairingState::Failed));
        assert!(!terminal(PairingState::Connecting));
        assert!(!terminal(PairingState::AwaitingVerification));
    }
}

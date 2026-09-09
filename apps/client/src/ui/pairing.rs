use std::{cell::RefCell, rc::Rc};

use continuehere::{
    ContinueHere, PairingHandle, PairingMode, PairingSessionChangedDelegate,
    PairingSessionChangedSubscription, PairingState,
};
use slint::ComponentHandle;

use super::{
    MainWindow,
    support::{EventTarget, UiResult, notify, show_result, text},
};

pub(super) struct PairingUiController {
    core: Rc<ContinueHere>,
    handle: Option<PairingHandle>,
    next_handle: u64,
    last_session: Option<continuehere::PairingSession>,
    _changed: PairingSessionChangedSubscription,
}

impl PairingUiController {
    pub(super) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Rc<RefCell<Self>> {
        let target = EventTarget::new(window);
        let changed = core
            .pairing()
            .on_session_changed(PairingSessionChangedDelegate::new(move |_| {
                target.dispatch(|window| {
                    window.invoke_refresh_pairing_requested();
                    notify(&window, 0);
                });
            }));
        let controller = Rc::new(RefCell::new(Self {
            core,
            handle: None,
            next_handle: 0,
            last_session: None,
            _changed: changed,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_pairing_action(move |action, endpoint| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = controller
                    .borrow_mut()
                    .act(action.as_str(), endpoint.as_str());
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
        controller.borrow_mut().refresh(window);
        controller
    }

    fn act(&mut self, action: &str, endpoint: &str) -> UiResult {
        match action {
            "receive" | "pair" => {
                self.handle.take();
                self.last_session = None;
                self.next_handle += 1;
                let handle = self
                    .core
                    .pairing()
                    .get_handle(&format!("ui.pairing.{}", self.next_handle))?;
                let mode = if action == "receive" {
                    PairingMode::Receive
                } else {
                    PairingMode::Initiate(endpoint.parse()?)
                };
                handle.configure(mode)?;
                handle.use_handle()?;
                self.handle = Some(handle);
            }
            "approve" => self
                .handle
                .as_ref()
                .ok_or("No active pairing session.")?
                .approve()?,
            "reject" => self
                .handle
                .as_ref()
                .ok_or("No active pairing session.")?
                .reject()?,
            "cancel" => {
                self.handle.take();
                self.last_session = None;
            }
            _ => return Err("Unknown pairing action.".into()),
        }
        Ok(())
    }

    fn refresh(&mut self, window: &MainWindow) {
        let rtl = window.get_rtl();
        window.set_pairing_active(self.handle.is_some());
        window.set_pairing_verifiable(false);
        window.set_pairing_code("".into());
        let session = self
            .handle
            .as_ref()
            .and_then(PairingHandle::session)
            .or_else(|| self.last_session.clone());
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
            if matches!(
                session.state(),
                PairingState::Trusted
                    | PairingState::Rejected
                    | PairingState::Cancelled
                    | PairingState::Expired
                    | PairingState::Failed
            ) {
                self.last_session = Some(session);
                self.handle.take();
                window.set_pairing_active(false);
            }
        } else if self.handle.is_some() {
            window.set_pairing_status(
                text(rtl, "Waiting for another device", "در انتظار دستگاه دیگر").into(),
            );
        } else {
            window.set_pairing_status("".into());
        }
        if let Ok(endpoint) = self.core.pairing().listening_endpoint() {
            window.set_pairing_endpoint(endpoint.to_string().into());
        }
    }
}

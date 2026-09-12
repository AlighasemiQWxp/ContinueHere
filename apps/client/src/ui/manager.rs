use std::{cell::RefCell, error::Error, rc::Rc, time::Duration};

use continuehere::ContinueHere;
use slint::ComponentHandle;
use tokio::runtime::Runtime;

use super::{
    MainWindow, devices::DevicesUiController, handoff::HandoffUiController,
    history::HistoryUiController, pairing::PairingUiController, preview::PreviewUiController,
    settings::SettingsUiController, transfers::TransferUiController,
    transition::UiTransitionController,
};

pub(crate) struct UiManager {
    core: Rc<ContinueHere>,
    devices: Rc<RefCell<DevicesUiController>>,
    settings: Rc<SettingsUiController>,
    pairing: Rc<RefCell<PairingUiController>>,
    handoff: Rc<RefCell<HandoffUiController>>,
    transfers: Rc<RefCell<TransferUiController>>,
    history: Rc<RefCell<HistoryUiController>>,
    preview: Rc<RefCell<PreviewUiController>>,
    transitions: Rc<UiTransitionController>,
}

impl UiManager {
    pub(crate) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Self {
        let transitions = UiTransitionController::start(window);
        let preview = PreviewUiController::start(window, Rc::clone(&transitions));
        let history = HistoryUiController::start(Rc::clone(&core), window);
        let pairing = PairingUiController::start(Rc::clone(&core), window);
        let handoff = HandoffUiController::start(Rc::clone(&core), window);
        let transfers = TransferUiController::start(Rc::clone(&core), window);
        let devices = DevicesUiController::start(Rc::clone(&core), window, Rc::clone(&transitions));
        let settings = SettingsUiController::start(Rc::clone(&core), window);
        let view = window.as_weak();
        window.on_page_selected(move |page| {
            let Some(window) = view.upgrade() else {
                return;
            };
            if ![0, 1, 3, 4].contains(&page) || window.get_active_transition() != 0 {
                return;
            }
            window.set_reduce_motion(crate::platform::reduce_motion());
            window.set_page(page);
            match page {
                0 => window.set_devices_unread(false),
                1 => window.set_send_unread(false),
                3 => {
                    window.set_history_unread(false);
                    window.set_history_device("".into());
                }
                _ => {}
            }
            window.invoke_refresh_history_requested();
            window.set_content_opacity(0.0);
            let view = window.as_weak();
            slint::Timer::single_shot(Duration::from_millis(16), move || {
                if let Some(window) = view.upgrade() {
                    window.set_content_opacity(1.0);
                }
            });
        });
        window.set_ready(true);
        window.set_reduce_motion(crate::platform::reduce_motion());
        Self {
            core,
            devices,
            settings,
            pairing,
            handoff,
            transfers,
            history,
            preview,
            transitions,
        }
    }

    pub(crate) fn shutdown(self, runtime: &Runtime) -> Result<(), Box<dyn Error>> {
        let Self {
            core,
            devices,
            settings,
            pairing,
            handoff,
            transfers,
            history,
            preview,
            transitions,
        } = self;
        drop(preview);
        drop(history);
        drop(transfers);
        drop(handoff);
        drop(pairing);
        drop(devices);
        drop(settings);
        drop(transitions);
        runtime.block_on(async {
            let core = Rc::try_unwrap(core).map_err(|_| "UI core ownership was not released")?;
            core.shutdown().await?;
            Ok(())
        })
    }
}

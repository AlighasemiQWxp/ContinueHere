use std::{cell::RefCell, error::Error, rc::Rc};

use continuehere::ContinueHere;
use tokio::runtime::Runtime;

use super::{
    MainWindow,
    devices::{DevicesAccess, DevicesUiController},
    handoff::{HandoffAccess, HandoffUiController},
    history::{HistoryAccess, HistoryUiController},
    pairing::{PairingAccess, PairingUiController},
    preview::PreviewUiController,
    selection::ContentSelectionUiController,
    settings::{SettingsAccess, SettingsUiController},
    shell::{ShellAccess, ShellController},
    transfers::{TransferUiController, TransfersAccess},
    transition::UiTransitionController,
};

pub(crate) struct UiManager {
    core: Rc<ContinueHere>,
    shell: Rc<ShellController>,
    devices: Rc<RefCell<DevicesUiController>>,
    settings: Rc<SettingsUiController>,
    pairing: Rc<RefCell<PairingUiController>>,
    handoff: Rc<RefCell<HandoffUiController>>,
    transfers: Rc<RefCell<TransferUiController>>,
    history: Rc<RefCell<HistoryUiController>>,
    preview: Rc<RefCell<PreviewUiController>>,
    transitions: Rc<UiTransitionController>,
    selection: Rc<RefCell<ContentSelectionUiController>>,
}

impl UiManager {
    pub(crate) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Self {
        let shell = ShellController::start(ShellAccess::new(Rc::clone(&core)), window);
        let transitions = UiTransitionController::start(window);
        let selection = ContentSelectionUiController::start(window);
        let preview = PreviewUiController::start(window, Rc::clone(&transitions));
        let preview_opener = PreviewUiController::opener(&preview, window);
        let history = HistoryUiController::start(
            HistoryAccess::new(Rc::clone(&core)),
            preview_opener.clone(),
            window,
        );
        let devices = DevicesUiController::start(
            DevicesAccess::new(Rc::clone(&core)),
            window,
            Rc::clone(&transitions),
        );
        let pairing = PairingUiController::start(
            PairingAccess::new(Rc::clone(&core)),
            DevicesUiController::pairing_devices(&devices, window),
            window,
        );
        let handoff = HandoffUiController::start(
            HandoffAccess::new(Rc::clone(&core)),
            window,
            Rc::clone(&transitions),
            Rc::clone(&selection),
            preview_opener.clone(),
        );
        let handoff_transfers = HandoffUiController::transfers(&handoff);
        let transfers = TransferUiController::start(
            TransfersAccess::new(Rc::clone(&core)),
            handoff_transfers,
            window,
            Rc::clone(&selection),
            preview_opener,
        );
        let settings = SettingsUiController::start(
            SettingsAccess::new(Rc::clone(&core)),
            window,
            Rc::clone(&selection),
        );
        shell.set_history_navigation(HistoryUiController::navigation(&history, window));
        window.set_ready(true);
        window.set_reduce_motion(crate::platform::reduce_motion());
        Self {
            core,
            shell,
            devices,
            settings,
            pairing,
            handoff,
            transfers,
            history,
            preview,
            transitions,
            selection,
        }
    }

    pub(crate) fn shutdown(self, runtime: &Runtime) -> Result<(), Box<dyn Error>> {
        let Self {
            core,
            shell,
            devices,
            settings,
            pairing,
            handoff,
            transfers,
            history,
            preview,
            transitions,
            selection,
        } = self;
        drop(shell);
        drop(preview);
        drop(history);
        drop(transfers);
        drop(handoff);
        drop(pairing);
        drop(devices);
        drop(settings);
        drop(selection);
        drop(transitions);
        runtime.block_on(async {
            let core = Rc::try_unwrap(core).map_err(|_| "UI core ownership was not released")?;
            core.shutdown().await?;
            Ok(())
        })
    }
}

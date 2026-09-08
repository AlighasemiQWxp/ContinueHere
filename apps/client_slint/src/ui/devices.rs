use std::{
    cell::RefCell,
    rc::{Rc, Weak as RcWeak},
    sync::{Arc, Mutex},
};

use continuehere::{
    ConnectionChangedDelegate, ConnectionChangedSubscription, ContinueHere,
    DeviceIdentityChangedDelegate, DeviceIdentityChangedSubscription, DiscoveryChangedDelegate,
    DiscoveryChangedSubscription, DiscoveryEndpoint, DiscoveryHandle, DiscoveryMode,
    DiscoveryStatusChangedDelegate, DiscoveryStatusChangedSubscription, Platform,
    TrustedDeviceChangedDelegate, TrustedDeviceChangedSubscription,
};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use super::{DeviceRow, MainWindow};

#[derive(Clone)]
struct UiEventTarget(Arc<Mutex<slint::Weak<MainWindow>>>);

impl UiEventTarget {
    fn new(window: slint::Weak<MainWindow>) -> Self {
        Self(Arc::new(Mutex::new(window)))
    }

    fn request_refresh(&self) {
        let window = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let _event_result = window.upgrade_in_event_loop(|window| {
            window.invoke_refresh_devices_requested();
        });
    }
}

pub(super) struct DevicesUiController {
    core: Rc<ContinueHere>,
    local_discovery: Option<DiscoveryHandle>,
    manual_discoveries: Vec<DiscoveryHandle>,
    next_manual_discovery: u64,
    _identity_changed: DeviceIdentityChangedSubscription,
    _discovery_changed: DiscoveryChangedSubscription,
    _discovery_status_changed: DiscoveryStatusChangedSubscription,
    _trusted_changed: TrustedDeviceChangedSubscription,
    _connection_changed: ConnectionChangedSubscription,
}

struct DevicesSnapshot {
    local_name: String,
    local_detail: String,
    nearby: Vec<DeviceItem>,
    trusted: Vec<DeviceItem>,
    discovery_active: bool,
}

struct DeviceItem {
    name: String,
    detail: String,
    connected: bool,
}

impl DevicesUiController {
    pub(super) fn start(core: Rc<ContinueHere>, window: &MainWindow) -> Rc<RefCell<Self>> {
        let local_discovery = start_local_discovery(&core, window);

        let event_target = UiEventTarget::new(window.as_weak());
        let identity_changed =
            core.devices()
                .on_identity_changed(DeviceIdentityChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let discovery_changed =
            core.discovery()
                .on_changed(DiscoveryChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let discovery_status_changed =
            core.discovery()
                .on_status_changed(DiscoveryStatusChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let trusted_changed =
            core.pairing()
                .on_trusted_device_changed(TrustedDeviceChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let connection_changed =
            core.transport()
                .on_connection_changed(ConnectionChangedDelegate::new(refresh_delegate(
                    event_target,
                )));

        let controller = Rc::new(RefCell::new(Self {
            core,
            local_discovery,
            manual_discoveries: Vec::new(),
            next_manual_discovery: 0,
            _identity_changed: identity_changed,
            _discovery_changed: discovery_changed,
            _discovery_status_changed: discovery_status_changed,
            _trusted_changed: trusted_changed,
            _connection_changed: connection_changed,
        }));
        Self::bind_callbacks(Rc::downgrade(&controller), window);
        controller.borrow().refresh(window);
        controller
    }

    fn bind_callbacks(controller: RcWeak<RefCell<Self>>, window: &MainWindow) {
        let window_weak = window.as_weak();
        let add_controller = controller.clone();
        window.on_add_manual_endpoint(move |value| {
            let Some(controller) = add_controller.upgrade() else {
                return;
            };
            let Some(window) = window_weak.upgrade() else {
                return;
            };
            match controller.borrow_mut().add_manual_endpoint(value.as_str()) {
                Ok(()) => window.set_error_message("".into()),
                Err(error) => window.set_error_message(error.to_string().into()),
            }
        });

        let refresh_controller = controller;
        let refresh_window = window.as_weak();
        window.on_refresh_devices_requested(move || {
            let Some(controller) = refresh_controller.upgrade() else {
                return;
            };
            let Some(window) = refresh_window.upgrade() else {
                return;
            };
            controller.borrow().refresh(&window);
        });
    }

    fn add_manual_endpoint(&mut self, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint = value.parse::<DiscoveryEndpoint>()?;
        self.next_manual_discovery += 1;
        let identifier = format!("ui.discovery.manual.{}", self.next_manual_discovery);
        let handle = self.core.discovery().get_handle(&identifier)?;
        handle.configure(DiscoveryMode::ManualEndpoint(endpoint))?;
        handle.use_handle()?;
        self.manual_discoveries.push(handle);
        Ok(())
    }

    fn refresh(&self, window: &MainWindow) {
        apply_snapshot(window, snapshot(&self.core));
    }
}

impl Drop for DevicesUiController {
    fn drop(&mut self) {
        for handle in &self.manual_discoveries {
            let _release_result = handle.release();
        }
        if let Some(handle) = &self.local_discovery {
            let _release_result = handle.release();
        }
    }
}

fn start_local_discovery(core: &ContinueHere, window: &MainWindow) -> Option<DiscoveryHandle> {
    let result = (|| {
        let handle = core.discovery().get_handle("ui.discovery.local")?;
        handle.configure(DiscoveryMode::LocalBrowse)?;
        handle.use_handle()?;
        Ok::<DiscoveryHandle, continuehere::DiscoveryError>(handle)
    })();
    match result {
        Ok(handle) => Some(handle),
        Err(error) => {
            window.set_error_message(error.to_string().into());
            None
        }
    }
}

fn refresh_delegate<T>(event_target: UiEventTarget) -> impl Fn(T) + Send + Sync + 'static
where
    T: 'static,
{
    move |_| {
        event_target.request_refresh();
    }
}

fn snapshot(core: &ContinueHere) -> DevicesSnapshot {
    let identity = core.devices().identity();
    let connections = core.transport().connections();
    let nearby = core
        .discovery()
        .candidates()
        .into_iter()
        .map(|candidate| DeviceItem {
            name: candidate.id().to_string(),
            detail: candidate
                .endpoints()
                .first()
                .map(ToString::to_string)
                .unwrap_or_default(),
            connected: false,
        })
        .collect();
    let trusted = core
        .pairing()
        .trusted_devices()
        .into_iter()
        .map(|device| DeviceItem {
            name: device.display_name().to_owned(),
            detail: platform_name(device.platform()).to_owned(),
            connected: connections
                .iter()
                .any(|connection| connection.device_id() == device.device_id()),
        })
        .collect();

    DevicesSnapshot {
        local_name: identity.display_name().to_owned(),
        local_detail: format!("{} · {}", platform_name(identity.platform()), identity.id()),
        nearby,
        trusted,
        discovery_active: matches!(
            core.discovery().status(),
            continuehere::DiscoveryStatus::Active
        ),
    }
}

fn apply_snapshot(window: &MainWindow, snapshot: DevicesSnapshot) {
    window.set_local_device_name(snapshot.local_name.into());
    window.set_local_device_detail(snapshot.local_detail.into());
    window.set_nearby_devices(device_model(snapshot.nearby));
    window.set_trusted_devices(device_model(snapshot.trusted));
    window.set_discovery_active(snapshot.discovery_active);
}

fn device_model(items: Vec<DeviceItem>) -> ModelRc<DeviceRow> {
    let rows: Vec<DeviceRow> = items
        .into_iter()
        .map(|item| DeviceRow {
            name: SharedString::from(item.name),
            detail: SharedString::from(item.detail),
            connected: item.connected,
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "Windows",
        Platform::Linux => "Linux",
        Platform::MacOs => "macOS",
        Platform::Android => "Android",
        Platform::Ios => "iOS",
        Platform::Unknown => "Unknown",
        _ => "Unknown",
    }
}

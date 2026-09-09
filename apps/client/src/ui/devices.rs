use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak as RcWeak},
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
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

type ConnectionFuture = Pin<Box<dyn Future<Output = Result<(), String>>>>;

struct ConnectionWake(super::support::EventTarget);

impl Wake for ConnectionWake {
    fn wake(self: Arc<Self>) {
        self.0
            .dispatch(|window| window.invoke_poll_connection_requested());
    }
}

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
            super::support::notify(&window, 0);
        });
    }
}

pub(super) struct DevicesUiController {
    advertisements: Vec<DiscoveryHandle>,
    addresses: Vec<std::net::Ipv4Addr>,
    network_timer: slint::Timer,
    core: Rc<ContinueHere>,
    connection: Option<ConnectionFuture>,
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
    id: String,
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
            advertisements: Vec::new(),
            addresses: Vec::new(),
            network_timer: slint::Timer::default(),
            core,
            connection: None,
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
        controller.borrow_mut().refresh_network(window);
        let view = window.as_weak();
        controller.borrow().network_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(5),
            move || {
                if let Some(window) = view.upgrade() {
                    window.invoke_refresh_network();
                }
            },
        );
        controller.borrow().refresh(window);
        controller
    }

    fn bind_callbacks(controller: RcWeak<RefCell<Self>>, window: &MainWindow) {
        let network_controller = controller.clone();
        let network_view = window.as_weak();
        window.on_refresh_network(move || {
            if let (Some(controller), Some(window)) =
                (network_controller.upgrade(), network_view.upgrade())
            {
                controller.borrow_mut().refresh_network(&window);
            }
        });
        let reconnect_controller = controller.clone();
        let reconnect_view = window.as_weak();
        window.on_reconnect_requested(move |id| {
            if let (Some(controller), Some(window)) =
                (reconnect_controller.upgrade(), reconnect_view.upgrade())
            {
                let controller = controller.borrow();
                let result = (|| -> super::support::UiResult {
                    let device = continuehere::DeviceId::new(id.to_string())?;
                    if !controller
                        .core
                        .pairing()
                        .trusted_devices()
                        .iter()
                        .any(|peer| peer.device_id() == &device)
                    {
                        return Err("Pair this device again before reconnecting.".into());
                    }
                    let endpoint = controller
                        .core
                        .transport()
                        .known_endpoint(&device)
                        .map(|value| value.to_string())
                        .unwrap_or_default();
                    window.set_reconnect_endpoint(endpoint.into());
                    window.set_reconnect_device(id.clone());
                    Ok(())
                })();
                super::support::show_result(&window, result);
            }
        });
        let poll_controller = controller.clone();
        let poll_window = window.as_weak();
        window.on_poll_connection_requested(move || {
            if let (Some(controller), Some(window)) =
                (poll_controller.upgrade(), poll_window.upgrade())
            {
                controller.borrow_mut().poll_connection(&window);
            }
        });
        let action_controller = controller.clone();
        let action_window = window.as_weak();
        window.on_device_action(move |id, action, endpoint| {
            if let (Some(controller), Some(window)) =
                (action_controller.upgrade(), action_window.upgrade())
            {
                let result = controller.borrow_mut().act(
                    id.as_str(),
                    action.as_str(),
                    endpoint.as_str(),
                    &window,
                );
                super::support::show_result(&window, result);
            }
        });
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
        if let Ok(endpoint) = self.core.transport().listening_endpoint() {
            window.set_transport_endpoint(endpoint.port().to_string().into());
        }
        window.invoke_refresh_history_requested();
    }

    fn act(
        &mut self,
        id: &str,
        action: &str,
        endpoint: &str,
        window: &MainWindow,
    ) -> super::support::UiResult {
        let id = continuehere::DeviceId::new(id.to_owned())?;
        if action == "forget" {
            self.core.pairing().remove_trusted_device(&id)?;
            return Ok(());
        }
        if window.get_connection_busy() {
            return Ok(());
        }
        let endpoint = if action == "connect" {
            Some(endpoint.parse::<DiscoveryEndpoint>()?)
        } else {
            None
        };
        if action != "connect" && action != "disconnect" {
            return Err("Unknown device action.".into());
        }
        let core = Rc::clone(&self.core);
        window.set_connection_busy(true);
        self.connection = Some(Box::pin(async move {
            if let Some(endpoint) = endpoint {
                core.transport().connect(&id, &endpoint).await.map(|_| ())
            } else {
                core.transport().disconnect(&id).await
            }
            .map_err(|error| error.to_string())
        }));
        self.poll_connection(window);
        Ok(())
    }

    fn refresh_network(&mut self, window: &MainWindow) {
        let addresses = match self.core.discovery().local_addresses() {
            Ok(value) => value,
            Err(error) => {
                window.set_local_addresses(
                    format!("Unable to read network addresses: {error}").into(),
                );
                return;
            }
        };
        let label = if addresses.is_empty() {
            super::support::text(
                window.get_rtl(),
                "No LAN address available. Connect to Wi-Fi or Ethernet.",
                "نشانی شبکه موجود نیست. به وای‌فای یا کابل شبکه متصل شوید.",
            )
            .to_owned()
        } else {
            addresses
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        };
        window.set_local_addresses(label.into());
        if addresses == self.addresses && !self.advertisements.is_empty() {
            return;
        }
        for handle in self.advertisements.drain(..) {
            let _ = handle.release();
        }
        self.addresses = addresses;
        let Ok(listener) = self.core.pairing().listening_endpoint() else {
            return;
        };
        for (index, address) in self.addresses.iter().enumerate() {
            let result = (|| -> Result<DiscoveryHandle, continuehere::DiscoveryError> {
                let endpoint = DiscoveryEndpoint::new(address.to_string(), listener.port())?;
                let handle = self
                    .core
                    .discovery()
                    .get_handle(&format!("ui.discovery.receive.{index}"))?;
                handle.configure(DiscoveryMode::AdvertiseEndpoint(endpoint))?;
                handle.use_handle()?;
                Ok(handle)
            })();
            match result {
                Ok(handle) => self.advertisements.push(handle),
                Err(error) => window.set_error_message(error.to_string().into()),
            }
        }
    }

    fn poll_connection(&mut self, window: &MainWindow) {
        let Some(connection) = &mut self.connection else {
            return;
        };
        let waker = Waker::from(Arc::new(ConnectionWake(super::support::EventTarget::new(
            window,
        ))));
        if let Poll::Ready(result) = connection.as_mut().poll(&mut Context::from_waker(&waker)) {
            self.connection.take();
            window.set_connection_busy(false);
            super::support::show_result(window, result);
            self.refresh(window);
        }
    }
}

impl Drop for DevicesUiController {
    fn drop(&mut self) {
        self.network_timer.stop();
        for handle in &self.advertisements {
            let _ = handle.release();
        }
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
            id: candidate.id().to_string(),
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
            id: device.device_id().to_string(),
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
    let selected = window.get_destination_device();
    if !snapshot
        .trusted
        .iter()
        .any(|item| item.id == selected.as_str() && item.connected)
    {
        window.set_destination_device(
            snapshot
                .trusted
                .iter()
                .find(|item| item.connected)
                .map(|item| item.id.as_str())
                .unwrap_or("")
                .into(),
        );
    }
    window.set_local_device_name(snapshot.local_name.into());
    let connected: Vec<_> = snapshot
        .trusted
        .iter()
        .filter(|item| item.connected)
        .collect();
    window.set_connected_device_names(super::support::model(
        connected
            .iter()
            .map(|item| item.name.as_str().into())
            .collect(),
    ));
    window.set_connected_device_ids(super::support::model(
        connected
            .iter()
            .map(|item| item.id.as_str().into())
            .collect(),
    ));
    window.set_connected_device_menu(super::support::model(
        connected
            .iter()
            .map(|item| super::MenuItem {
                text: item.name.as_str().into(),
                enabled: true,
                ..Default::default()
            })
            .collect(),
    ));
    window.set_destination_index(
        connected
            .iter()
            .position(|item| item.id == window.get_destination_device().as_str())
            .map(|index| index as i32)
            .unwrap_or(-1),
    );
    window.set_local_device_detail(snapshot.local_detail.into());
    window.set_nearby_devices(device_model(snapshot.nearby));
    window.set_trusted_devices(device_model(snapshot.trusted));
    window.set_discovery_active(snapshot.discovery_active);
}

fn device_model(items: Vec<DeviceItem>) -> ModelRc<DeviceRow> {
    let rows: Vec<DeviceRow> = items
        .into_iter()
        .map(|item| DeviceRow {
            id: item.id.into(),
            name: SharedString::from(item.name),
            detail: SharedString::from(item.detail),
            connected: item.connected,
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn platform_name(platform: Platform) -> &'static str {
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

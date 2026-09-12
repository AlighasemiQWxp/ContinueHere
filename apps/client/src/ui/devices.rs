use std::{
    cell::RefCell,
    collections::BTreeMap,
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

use super::{
    DeviceRow, MainWindow,
    transition::{UiTransition, UiTransitionController, UiTransitionHandle},
};

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
    advertised_listener: Option<DiscoveryEndpoint>,
    network_timer: slint::Timer,
    core: Rc<ContinueHere>,
    connection: Option<ConnectionFuture>,
    local_discovery: Option<DiscoveryHandle>,
    manual_discoveries: BTreeMap<DiscoveryEndpoint, DiscoveryHandle>,
    next_manual_discovery: u64,
    transitions: Rc<UiTransitionController>,
    reconnect_transition: Option<UiTransitionHandle>,
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
    endpoint: String,
    connected: bool,
}

impl DevicesUiController {
    pub(super) fn start(
        core: Rc<ContinueHere>,
        window: &MainWindow,
        transitions: Rc<UiTransitionController>,
    ) -> Rc<RefCell<Self>> {
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
            advertised_listener: None,
            network_timer: slint::Timer::default(),
            core,
            connection: None,
            local_discovery,
            manual_discoveries: BTreeMap::new(),
            next_manual_discovery: 0,
            transitions,
            reconnect_transition: None,
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
                let result = controller.borrow_mut().open_reconnect(id.as_str(), &window);
                super::support::show_result(&window, result);
            }
        });
        let close_controller = controller.clone();
        let close_window = window.as_weak();
        window.on_close_reconnect_requested(move || {
            if let (Some(controller), Some(window)) =
                (close_controller.upgrade(), close_window.upgrade())
            {
                controller.borrow_mut().close_reconnect(&window);
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
        let pairing_connect_controller = controller.clone();
        let pairing_connect_window = window.as_weak();
        window.on_pairing_connect_requested(move |id, endpoint| {
            if let (Some(controller), Some(window)) = (
                pairing_connect_controller.upgrade(),
                pairing_connect_window.upgrade(),
            ) {
                let result =
                    controller
                        .borrow_mut()
                        .act(id.as_str(), "connect", endpoint.as_str(), &window);
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
                Ok(true) => {}
                Ok(false) => {
                    window.invoke_show_notice(
                        super::support::text(
                            window.get_rtl(),
                            "This endpoint is already in the list.",
                            "این نشانی از قبل در فهرست وجود دارد.",
                        )
                        .into(),
                    );
                }
                Err(error) => window.invoke_show_error_requested(error.to_string().into()),
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

    fn add_manual_endpoint(&mut self, value: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let endpoint = value.parse::<DiscoveryEndpoint>()?;
        if self.manual_discoveries.contains_key(&endpoint) {
            return Ok(false);
        }
        self.next_manual_discovery += 1;
        let identifier = format!("ui.discovery.manual.{}", self.next_manual_discovery);
        let handle = self.core.discovery().get_handle(&identifier)?;
        handle.configure(DiscoveryMode::ManualEndpoint(endpoint.clone()))?;
        handle.use_handle()?;
        self.manual_discoveries.insert(endpoint, handle);
        Ok(true)
    }

    fn open_reconnect(&mut self, id: &str, window: &MainWindow) -> super::support::UiResult {
        self.close_reconnect(window);
        let device = continuehere::DeviceId::new(id.to_owned())?;
        if !self
            .core
            .pairing()
            .trusted_devices()
            .iter()
            .any(|peer| peer.device_id() == &device)
        {
            return Err("Pair this device again before reconnecting.".into());
        }
        let endpoint = self
            .core
            .transport()
            .known_endpoint(&device)
            .map(|value| value.to_string())
            .unwrap_or_default();
        let transition = self.transitions.get_handle("reconnect");
        transition.configure(UiTransition::Reconnect)?;
        transition.use_handle()?;
        window.set_reconnect_endpoint(endpoint.into());
        window.set_reconnect_device(id.into());
        self.reconnect_transition = Some(transition);
        Ok(())
    }

    fn close_reconnect(&mut self, window: &MainWindow) {
        window.set_reconnect_device("".into());
        window.set_reconnect_endpoint("".into());
        self.reconnect_transition.take();
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
        if action == "connect"
            && self
                .core
                .transport()
                .connections()
                .iter()
                .any(|connection| connection.device_id() == &id)
        {
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
        let listener = match self.core.pairing().listening_endpoint() {
            Ok(listener) => listener,
            Err(_) => {
                self.release_advertisements();
                self.addresses = addresses;
                self.advertised_listener = None;
                return;
            }
        };
        if addresses == self.addresses
            && self.advertised_listener.as_ref() == Some(&listener)
            && !self.advertisements.is_empty()
        {
            return;
        }
        self.release_advertisements();
        self.addresses = addresses;
        self.advertised_listener = Some(listener.clone());
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
                Err(error) => window.invoke_show_error_requested(error.to_string().into()),
            }
        }
    }

    fn release_advertisements(&mut self) {
        for handle in self.advertisements.drain(..) {
            let _ = handle.release();
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
        self.reconnect_transition.take();
        for handle in &self.advertisements {
            let _ = handle.release();
        }
        for handle in self.manual_discoveries.values() {
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
            window.invoke_show_error_requested(error.to_string().into());
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
    let mut nearby_by_endpoint = BTreeMap::new();
    for candidate in core.discovery().candidates() {
        if let Some(endpoint) = candidate.endpoints().first() {
            insert_nearby(
                &mut nearby_by_endpoint,
                candidate.id().to_string(),
                endpoint.clone(),
            );
        }
    }
    let nearby = nearby_by_endpoint.into_values().collect();
    let trusted = core
        .pairing()
        .trusted_devices()
        .into_iter()
        .map(|device| DeviceItem {
            id: device.device_id().to_string(),
            name: device.display_name().to_owned(),
            detail: platform_name(device.platform()).to_owned(),
            endpoint: core
                .transport()
                .known_endpoint(device.device_id())
                .map(|endpoint| endpoint.to_string())
                .unwrap_or_default(),
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

fn insert_nearby(
    items: &mut BTreeMap<DiscoveryEndpoint, DeviceItem>,
    id: String,
    endpoint: DiscoveryEndpoint,
) {
    items.entry(endpoint.clone()).or_insert_with(|| DeviceItem {
        name: id.clone(),
        id,
        detail: endpoint.to_string(),
        endpoint: String::new(),
        connected: false,
    });
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
            endpoint: SharedString::from(item.endpoint),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use continuehere::DiscoveryEndpoint;

    use super::insert_nearby;

    #[test]
    fn nearby_candidates_are_coalesced_by_normalized_endpoint() {
        let mut items = BTreeMap::new();
        let first: DiscoveryEndpoint = "EXAMPLE.local:4242".parse().expect("valid endpoint");
        let duplicate: DiscoveryEndpoint = "example.local:4242".parse().expect("valid endpoint");

        insert_nearby(&mut items, "first".to_owned(), first);
        insert_nearby(&mut items, "duplicate".to_owned(), duplicate);

        assert_eq!(items.len(), 1);
        assert_eq!(items.into_values().next().expect("one item").id, "first");
    }
}

mod access;
mod snapshot;

pub(in crate::ui) use access::DevicesAccess;

pub(super) use snapshot::platform_name;
use snapshot::{apply_snapshot, snapshot};

mod bindings;

use crate::ui::shared::EventTarget;

use std::{
    cell::RefCell,
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

use continuehere::{
    ConnectionChangedDelegate, ConnectionChangedSubscription, DeviceId,
    DeviceIdentityChangedDelegate, DeviceIdentityChangedSubscription, DiscoveryChangedDelegate,
    DiscoveryChangedSubscription, DiscoveryEndpoint, DiscoveryHandle, DiscoveryMode,
    DiscoveryStatusChangedDelegate, DiscoveryStatusChangedSubscription,
    TrustedDeviceChangedDelegate, TrustedDeviceChangedSubscription,
};
use slint::ComponentHandle;

use super::{
    MainWindow,
    transition::{UiTransition, UiTransitionController, UiTransitionHandle},
};

type ConnectionFuture = Pin<Box<dyn Future<Output = Result<(), String>>>>;

struct ConnectionWake(super::shared::EventTarget);

impl Wake for ConnectionWake {
    fn wake(self: Arc<Self>) {
        self.0
            .dispatch(|window| window.invoke_poll_connection_requested());
    }
}

pub(super) struct DevicesUiController {
    advertisements: Vec<DiscoveryHandle>,
    addresses: Vec<std::net::Ipv4Addr>,
    advertised_listener: Option<DiscoveryEndpoint>,
    network_timer: slint::Timer,
    access: DevicesAccess,
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

#[derive(Clone, Copy)]
pub(super) enum DeviceAction {
    Forget,
    Connect,
    Disconnect,
}

impl DeviceAction {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "forget" => Some(Self::Forget),
            "connect" => Some(Self::Connect),
            "disconnect" => Some(Self::Disconnect),
            _ => None,
        }
    }
}

pub(super) struct PairingDevices {
    controller: std::rc::Weak<RefCell<DevicesUiController>>,
    window: slint::Weak<MainWindow>,
}

impl PairingDevices {
    pub(super) fn connect(&self, device: DeviceId, endpoint: DiscoveryEndpoint) {
        let (Some(controller), Some(window)) = (self.controller.upgrade(), self.window.upgrade())
        else {
            return;
        };
        let result = controller
            .borrow_mut()
            .connect_after_pairing(device, endpoint, &window);
        super::shared::show_result(&window, result);
    }

    pub(super) fn refresh_network(&self) {
        let (Some(controller), Some(window)) = (self.controller.upgrade(), self.window.upgrade())
        else {
            return;
        };
        controller.borrow_mut().refresh_network(&window);
    }
}

impl DevicesUiController {
    pub(super) fn start(
        access: DevicesAccess,
        window: &MainWindow,
        transitions: Rc<UiTransitionController>,
    ) -> Rc<RefCell<Self>> {
        let local_discovery = start_local_discovery(&access, window);

        let event_target = EventTarget::new(window);
        let identity_changed =
            access
                .devices()
                .on_identity_changed(DeviceIdentityChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let discovery_changed =
            access
                .discovery()
                .on_changed(DiscoveryChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let discovery_status_changed =
            access
                .discovery()
                .on_status_changed(DiscoveryStatusChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let trusted_changed =
            access
                .pairing()
                .on_trusted_device_changed(TrustedDeviceChangedDelegate::new(refresh_delegate(
                    event_target.clone(),
                )));
        let connection_changed =
            access
                .transport()
                .on_connection_changed(ConnectionChangedDelegate::new(refresh_delegate(
                    event_target,
                )));

        let controller = Rc::new(RefCell::new(Self {
            advertisements: Vec::new(),
            addresses: Vec::new(),
            advertised_listener: None,
            network_timer: slint::Timer::default(),
            access,
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

    pub(super) fn pairing_devices(
        controller: &Rc<RefCell<Self>>,
        window: &MainWindow,
    ) -> PairingDevices {
        PairingDevices {
            controller: Rc::downgrade(controller),
            window: window.as_weak(),
        }
    }

    fn add_manual_endpoint(&mut self, value: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let endpoint = value.parse::<DiscoveryEndpoint>()?;
        if self.manual_discoveries.contains_key(&endpoint) {
            return Ok(false);
        }
        self.next_manual_discovery += 1;
        let identifier = format!("ui.discovery.manual.{}", self.next_manual_discovery);
        let handle = self.access.discovery().get_handle(&identifier)?;
        handle.configure(DiscoveryMode::ManualEndpoint(endpoint.clone()))?;
        handle.use_handle()?;
        self.manual_discoveries.insert(endpoint, handle);
        Ok(true)
    }

    fn open_reconnect(&mut self, id: &str, window: &MainWindow) -> super::shared::UiResult {
        self.close_reconnect(window);
        let device = continuehere::DeviceId::new(id.to_owned())?;
        if !self
            .access
            .pairing()
            .trusted_devices()
            .iter()
            .any(|peer| peer.device_id() == &device)
        {
            return Err("Pair this device again before reconnecting.".into());
        }
        let endpoint = self
            .access
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
        apply_snapshot(window, snapshot(&self.access));
        if let Ok(endpoint) = self.access.transport().listening_endpoint() {
            window.set_transport_endpoint(endpoint.port().to_string().into());
        }
    }

    fn act(
        &mut self,
        id: &str,
        action: DeviceAction,
        endpoint: &str,
        window: &MainWindow,
    ) -> super::shared::UiResult {
        let id = continuehere::DeviceId::new(id.to_owned())?;
        match action {
            DeviceAction::Forget => {
                self.access.pairing().remove_trusted_device(&id)?;
                Ok(())
            }
            DeviceAction::Connect => {
                if self
                    .access
                    .transport()
                    .connections()
                    .iter()
                    .any(|connection| connection.device_id() == &id)
                {
                    return Ok(());
                }
                self.start_connection(id, Some(endpoint.parse::<DiscoveryEndpoint>()?), window)
            }
            DeviceAction::Disconnect => self.start_connection(id, None, window),
        }
    }

    fn connect_after_pairing(
        &mut self,
        device: DeviceId,
        endpoint: DiscoveryEndpoint,
        window: &MainWindow,
    ) -> super::shared::UiResult {
        self.start_connection(device, Some(endpoint), window)
    }

    fn start_connection(
        &mut self,
        id: DeviceId,
        endpoint: Option<DiscoveryEndpoint>,
        window: &MainWindow,
    ) -> super::shared::UiResult {
        if endpoint.is_some()
            && self
                .access
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
        let access = self.access.clone();
        window.set_connection_busy(true);
        self.connection = Some(Box::pin(async move {
            if let Some(endpoint) = endpoint {
                access.transport().connect(&id, &endpoint).await.map(|_| ())
            } else {
                access.transport().disconnect(&id).await
            }
            .map_err(|error| error.to_string())
        }));
        self.poll_connection(window);
        Ok(())
    }

    fn refresh_network(&mut self, window: &MainWindow) {
        let addresses = match self.access.discovery().local_addresses() {
            Ok(value) => value,
            Err(error) => {
                window.set_local_addresses(
                    format!("Unable to read network addresses: {error}").into(),
                );
                return;
            }
        };
        let label = if addresses.is_empty() {
            super::shared::text(
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
        let listener = match self.access.pairing().listening_endpoint() {
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
                    .access
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
        let waker = Waker::from(Arc::new(ConnectionWake(super::shared::EventTarget::new(
            window,
        ))));
        if let Poll::Ready(result) = connection.as_mut().poll(&mut Context::from_waker(&waker)) {
            self.connection.take();
            window.set_connection_busy(false);
            super::shared::show_result(window, result);
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

fn start_local_discovery(access: &DevicesAccess, window: &MainWindow) -> Option<DiscoveryHandle> {
    let result = (|| {
        let handle = access.discovery().get_handle("ui.discovery.local")?;
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

fn refresh_delegate<T>(event_target: EventTarget) -> impl Fn(T) + Send + Sync + 'static
where
    T: 'static,
{
    move |_| {
        event_target.dispatch(|window| {
            window.invoke_refresh_devices_requested();
            crate::ui::shared::notify(&window, 0);
        });
    }
}

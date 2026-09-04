use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{discovery::DiscoveryEndpoint, security::CryptographicIdentity};

use super::{PairingChannel, TransportError};

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const INCOMING_QUEUE_LIMIT: usize = 8;
const INCOMING_RATE_WINDOW: Duration = Duration::from_secs(60);
const MAX_INCOMING_PER_ADDRESS: usize = 5;
const MAX_RATE_LIMIT_ADDRESSES: usize = 256;

#[derive(Clone)]
pub(crate) struct PairingTransportCapability {
    state: Arc<Mutex<PairingTransportState>>,
}

struct PairingTransportState {
    running: bool,
    listener_owners: usize,
    endpoint: Option<DiscoveryEndpoint>,
    incoming: Option<Arc<Mutex<mpsc::Receiver<TcpStream>>>>,
    shutdown: Option<Arc<AtomicBool>>,
    worker: Option<JoinHandle<()>>,
}

impl PairingTransportCapability {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PairingTransportState {
                running: false,
                listener_owners: 0,
                endpoint: None,
                incoming: None,
                shutdown: None,
                worker: None,
            })),
        }
    }

    pub(crate) fn start(&self) -> Result<(), TransportError> {
        lock(&self.state)?.running = true;
        Ok(())
    }

    pub(crate) fn stop(&self) -> Result<(), TransportError> {
        let worker = {
            let mut state = lock(&self.state)?;
            state.running = false;
            state.listener_owners = 0;
            take_listener(&mut state)
        };
        stop_listener(worker)
    }

    pub(crate) fn acquire_listener(&self) -> Result<DiscoveryEndpoint, TransportError> {
        let mut state = lock(&self.state)?;
        if !state.running {
            return Err(TransportError::ManagerUnavailable);
        }
        if let Some(endpoint) = state.endpoint.clone() {
            state.listener_owners += 1;
            return Ok(endpoint);
        }
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            .map_err(|_| TransportError::ListenerUnavailable)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| TransportError::ListenerUnavailable)?;
        let port = listener
            .local_addr()
            .map_err(|_| TransportError::ListenerUnavailable)?
            .port();
        let endpoint = DiscoveryEndpoint::new("0.0.0.0", port)
            .map_err(|_| TransportError::ListenerUnavailable)?;
        let (sender, receiver) = mpsc::sync_channel(INCOMING_QUEUE_LIMIT);
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = Arc::clone(&shutdown);
        let worker = thread::Builder::new()
            .name("continuehere-pairing-listener".to_owned())
            .spawn(move || run_listener(listener, sender, worker_shutdown))
            .map_err(|_| TransportError::ListenerUnavailable)?;
        state.listener_owners = 1;
        state.endpoint = Some(endpoint.clone());
        state.incoming = Some(Arc::new(Mutex::new(receiver)));
        state.shutdown = Some(shutdown);
        state.worker = Some(worker);
        Ok(endpoint)
    }

    pub(crate) fn release_listener(&self) -> Result<(), TransportError> {
        let worker = {
            let mut state = lock(&self.state)?;
            if state.listener_owners == 0 {
                return Ok(());
            }
            state.listener_owners -= 1;
            if state.listener_owners != 0 {
                return Ok(());
            }
            take_listener(&mut state)
        };
        stop_listener(worker)
    }

    pub(crate) fn endpoint(&self) -> Result<DiscoveryEndpoint, TransportError> {
        lock(&self.state)?
            .endpoint
            .clone()
            .ok_or(TransportError::ManagerUnavailable)
    }

    pub(crate) fn connect(
        &self,
        endpoint: &DiscoveryEndpoint,
        identity: &CryptographicIdentity,
    ) -> Result<PairingChannel, TransportError> {
        if !lock(&self.state)?.running {
            return Err(TransportError::ManagerUnavailable);
        }
        PairingChannel::connect(endpoint, identity)
    }

    pub(crate) fn accept(
        &self,
        identity: &CryptographicIdentity,
        timeout: Duration,
    ) -> Result<Option<PairingChannel>, TransportError> {
        let incoming = {
            let state = lock(&self.state)?;
            state
                .incoming
                .as_ref()
                .cloned()
                .ok_or(TransportError::ManagerUnavailable)?
        };
        let stream = {
            let receiver = lock(&incoming)?;
            match receiver.recv_timeout(timeout) {
                Ok(stream) => Some(stream),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(TransportError::ListenerUnavailable);
                }
            }
        };
        match stream {
            Some(stream) => PairingChannel::accept(stream, identity).map(Some),
            None => Ok(None),
        }
    }
}

fn take_listener(
    state: &mut PairingTransportState,
) -> (Option<Arc<AtomicBool>>, Option<JoinHandle<()>>) {
    let shutdown = state.shutdown.take();
    let worker = state.worker.take();
    state.endpoint = None;
    state.incoming = None;
    (shutdown, worker)
}

fn stop_listener(
    listener: (Option<Arc<AtomicBool>>, Option<JoinHandle<()>>),
) -> Result<(), TransportError> {
    let (shutdown, worker) = listener;
    if let Some(shutdown) = shutdown {
        shutdown.store(true, Ordering::Release);
    }
    if let Some(worker) = worker {
        worker
            .join()
            .map_err(|_| TransportError::WorkerStopFailed)?;
    }
    Ok(())
}

fn run_listener(
    listener: TcpListener,
    incoming: mpsc::SyncSender<TcpStream>,
    shutdown: Arc<AtomicBool>,
) {
    let mut attempts = HashMap::new();
    while !shutdown.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, address)) => {
                if allow_incoming(&mut attempts, address.ip()) {
                    match incoming.try_send(stream) {
                        Ok(()) | Err(mpsc::TrySendError::Full(_)) => {}
                        Err(mpsc::TrySendError::Disconnected(_)) => return,
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::park_timeout(ACCEPT_POLL_INTERVAL);
            }
            Err(_) => return,
        }
    }
}

fn allow_incoming(attempts: &mut HashMap<IpAddr, VecDeque<Instant>>, address: IpAddr) -> bool {
    let now = Instant::now();
    let oldest_allowed = now - INCOMING_RATE_WINDOW;
    attempts.retain(|_, values| {
        values.retain(|attempt| *attempt >= oldest_allowed);
        !values.is_empty()
    });
    if !attempts.contains_key(&address) && attempts.len() >= MAX_RATE_LIMIT_ADDRESSES {
        return false;
    }
    let attempts = attempts.entry(address).or_default();
    if attempts.len() >= MAX_INCOMING_PER_ADDRESS {
        return false;
    }
    attempts.push_back(now);
    true
}

fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>, TransportError> {
    value
        .lock()
        .map_err(|_| TransportError::SynchronizationFailed)
}

#[cfg(test)]
mod tests {
    use std::{net::TcpStream, sync::mpsc, thread, time::Duration};

    use super::PairingTransportCapability;

    #[test]
    fn pairing_listener_follows_reference_ownership() {
        let capability = PairingTransportCapability::new();
        capability.start().expect("transport should start");

        assert!(capability.endpoint().is_err());
        let endpoint = capability
            .acquire_listener()
            .expect("listener endpoint should be available");
        assert_eq!(endpoint.host(), "0.0.0.0");
        assert_ne!(endpoint.port(), 0);

        capability
            .release_listener()
            .expect("listener should release");
        assert!(capability.endpoint().is_err());
        capability.stop().expect("transport should stop");
    }

    #[test]
    fn waiting_for_connection_does_not_lock_transport_state() {
        let capability = PairingTransportCapability::new();
        capability.start().expect("transport should start");
        let endpoint = capability
            .acquire_listener()
            .expect("listener endpoint should be available");
        let incoming = capability
            .state
            .lock()
            .expect("transport state should lock")
            .incoming
            .clone()
            .expect("incoming queue should be available");
        let (waiting_sender, waiting_receiver) = mpsc::channel();
        let waiting_worker = thread::spawn(move || {
            let receiver = incoming.lock().expect("incoming queue should lock");
            waiting_sender
                .send(())
                .expect("queue wait should be observable");
            receiver.recv_timeout(Duration::from_secs(5))
        });
        waiting_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("incoming queue should begin waiting");

        let endpoint_capability = capability.clone();
        let (endpoint_sender, endpoint_receiver) = mpsc::channel();
        let endpoint_worker = thread::spawn(move || {
            endpoint_sender
                .send(endpoint_capability.endpoint())
                .expect("endpoint result should be observable");
        });
        let observed_endpoint = endpoint_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("queue wait must not block transport state")
            .expect("endpoint should remain available");
        assert_eq!(observed_endpoint, endpoint);

        TcpStream::connect(("127.0.0.1", endpoint.port()))
            .expect("loopback connection should succeed");
        waiting_worker
            .join()
            .expect("queue wait should stop")
            .expect("connection should reach incoming queue");
        endpoint_worker.join().expect("endpoint read should stop");
        capability
            .release_listener()
            .expect("listener should release");
        capability.stop().expect("transport should stop");
    }
}

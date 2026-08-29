use std::{collections::HashSet, net::IpAddr, time::Duration};

use mdns_sd::{
    DaemonEvent, Receiver, ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent, ServiceInfo,
};

use crate::models::ProtocolVersion;

use super::{
    DiscoveryCandidateId, DiscoveryEndpoint, DiscoveryError, MAX_DISCOVERY_IDENTIFIER_SIZE,
};

const SERVICE_TYPE: &str = "_continuehere._tcp.local.";
const PROTOCOL_VERSION_PROPERTY: &str = "v";

pub(crate) enum DiscoveryBackendEvent {
    Resolved {
        id: DiscoveryCandidateId,
        endpoints: Vec<DiscoveryEndpoint>,
        protocol_version: ProtocolVersion,
    },
    Removed(DiscoveryCandidateId),
}

pub(crate) trait DiscoveryBackend: Send {
    fn start_browse(&mut self) -> Result<(), DiscoveryError>;

    fn stop_browse(&mut self) -> Result<(), DiscoveryError>;

    fn start_advertisement(
        &mut self,
        instance_id: &DiscoveryCandidateId,
        endpoint: &DiscoveryEndpoint,
    ) -> Result<String, DiscoveryError>;

    fn stop_advertisement(&mut self, advertisement: &str) -> Result<(), DiscoveryError>;

    fn poll_event(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<DiscoveryBackendEvent>, DiscoveryError>;

    fn shutdown(&mut self) -> Result<(), DiscoveryError>;
}

pub(crate) struct MdnsDiscoveryBackend {
    daemon: ServiceDaemon,
    monitor: Receiver<DaemonEvent>,
    browse: Option<Receiver<ServiceEvent>>,
    advertisements: HashSet<String>,
}

impl MdnsDiscoveryBackend {
    pub(crate) fn new() -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new().map_err(|_| DiscoveryError::BackendUnavailable)?;
        let monitor = daemon
            .monitor()
            .map_err(|_| DiscoveryError::BackendUnavailable)?;
        Ok(Self {
            daemon,
            monitor,
            browse: None,
            advertisements: HashSet::new(),
        })
    }
}

impl DiscoveryBackend for MdnsDiscoveryBackend {
    fn start_browse(&mut self) -> Result<(), DiscoveryError> {
        if self.browse.is_none() {
            self.browse = Some(
                self.daemon
                    .browse(SERVICE_TYPE)
                    .map_err(|_| DiscoveryError::BackendUnavailable)?,
            );
        }
        Ok(())
    }

    fn stop_browse(&mut self) -> Result<(), DiscoveryError> {
        if self.browse.take().is_some() {
            self.daemon
                .stop_browse(SERVICE_TYPE)
                .map_err(|_| DiscoveryError::BackendUnavailable)?;
        }
        Ok(())
    }

    fn start_advertisement(
        &mut self,
        instance_id: &DiscoveryCandidateId,
        endpoint: &DiscoveryEndpoint,
    ) -> Result<String, DiscoveryError> {
        let host_label = instance_id.as_str().chars().take(24).collect::<String>();
        let hostname = format!("ch-{host_label}.local.");
        let version = ProtocolVersion::CURRENT.to_string();
        let properties = [(PROTOCOL_VERSION_PROPERTY, version.as_str())];
        let addresses = advertisement_addresses(endpoint);
        let service = ServiceInfo::new(
            SERVICE_TYPE,
            instance_id.as_str(),
            &hostname,
            addresses.as_str(),
            endpoint.port(),
            &properties[..],
        )
        .map_err(|_| DiscoveryError::BackendUnavailable)?
        .enable_addr_auto();
        let fullname = service.get_fullname().to_owned();
        self.daemon
            .register(service)
            .map_err(|_| DiscoveryError::BackendUnavailable)?;
        self.advertisements.insert(fullname.clone());
        Ok(fullname)
    }

    fn stop_advertisement(&mut self, advertisement: &str) -> Result<(), DiscoveryError> {
        self.daemon
            .unregister(advertisement)
            .map_err(|_| DiscoveryError::BackendUnavailable)?;
        self.advertisements.remove(advertisement);
        Ok(())
    }

    fn poll_event(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<DiscoveryBackendEvent>, DiscoveryError> {
        while let Ok(event) = self.monitor.try_recv() {
            if matches!(event, DaemonEvent::Error(_)) {
                return Err(DiscoveryError::BackendUnavailable);
            }
        }

        let Some(receiver) = self.browse.as_ref() else {
            std::thread::park_timeout(timeout);
            return Ok(None);
        };
        let event = match receiver.recv_timeout(timeout) {
            Ok(event) => event,
            Err(mdns_sd::RecvTimeoutError::Timeout) => return Ok(None),
            Err(mdns_sd::RecvTimeoutError::Disconnected) => {
                return Err(DiscoveryError::BackendUnavailable);
            }
        };

        match event {
            ServiceEvent::ServiceResolved(service) => Ok(resolve_service(&service)),
            ServiceEvent::ServiceRemoved(_, fullname) => {
                Ok(candidate_id_from_fullname(&fullname).map(DiscoveryBackendEvent::Removed))
            }
            _ => Ok(None),
        }
    }

    fn shutdown(&mut self) -> Result<(), DiscoveryError> {
        let _browse_result = self.stop_browse();
        let advertisements = std::mem::take(&mut self.advertisements);
        for fullname in advertisements {
            let _unregister_result = self.daemon.unregister(&fullname);
        }
        self.daemon
            .shutdown()
            .map_err(|_| DiscoveryError::BackendUnavailable)?;
        Ok(())
    }
}

fn resolve_service(service: &ResolvedService) -> Option<DiscoveryBackendEvent> {
    if !service.is_valid() || service.get_port() == 0 {
        return None;
    }
    let properties = service.get_properties();
    if properties.len() != 1
        || !properties.iter().all(|property| {
            property
                .key()
                .eq_ignore_ascii_case(PROTOCOL_VERSION_PROPERTY)
        })
    {
        return None;
    }

    let protocol_version =
        parse_protocol_version(service.get_property_val_str(PROTOCOL_VERSION_PROPERTY)?)?;
    if protocol_version.major() != ProtocolVersion::CURRENT.major() {
        return None;
    }
    let id = candidate_id_from_fullname(service.get_fullname())?;
    let endpoints = service
        .get_addresses()
        .iter()
        .filter_map(|address| endpoint_from_scoped_ip(address, service.get_port()))
        .collect::<Vec<_>>();
    if endpoints.is_empty() {
        return None;
    }

    Some(DiscoveryBackendEvent::Resolved {
        id,
        endpoints,
        protocol_version,
    })
}

fn endpoint_from_scoped_ip(address: &ScopedIp, port: u16) -> Option<DiscoveryEndpoint> {
    let host = match address {
        ScopedIp::V4(address) => address.addr().to_string(),
        ScopedIp::V6(address) => {
            if address.addr().is_unicast_link_local() {
                format!("{}%{}", address.addr(), address.scope_id().index)
            } else {
                address.addr().to_string()
            }
        }
        _ => return None,
    };
    DiscoveryEndpoint::new(host, port).ok()
}

fn candidate_id_from_fullname(fullname: &str) -> Option<DiscoveryCandidateId> {
    let value = fullname.strip_suffix(SERVICE_TYPE)?.trim_end_matches('.');
    if value.len() > MAX_DISCOVERY_IDENTIFIER_SIZE {
        return None;
    }
    DiscoveryCandidateId::new(value).ok()
}

fn parse_protocol_version(value: &str) -> Option<ProtocolVersion> {
    let (major, minor) = value.split_once('.')?;
    let major = major.parse::<u16>().ok()?;
    let minor = minor.parse::<u16>().ok()?;
    Some(ProtocolVersion::new(major, minor))
}

fn advertisement_addresses(endpoint: &DiscoveryEndpoint) -> String {
    match endpoint.host().parse::<IpAddr>() {
        Ok(address) if !address.is_unspecified() => address.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{DiscoveryBackend, DiscoveryBackendEvent, MdnsDiscoveryBackend};
    use crate::discovery::{DiscoveryCandidateId, DiscoveryEndpoint};

    #[test]
    #[ignore = "requires a multicast-capable local network"]
    fn local_backend_advertises_and_discovers_a_candidate() {
        let mut advertiser = MdnsDiscoveryBackend::new().expect("advertiser should start");
        let mut browser = MdnsDiscoveryBackend::new().expect("browser should start");
        browser.start_browse().expect("browse should start");
        let id = DiscoveryCandidateId::new(format!("test-{}", uuid::Uuid::new_v4()))
            .expect("identifier should be valid");
        let endpoint = DiscoveryEndpoint::new("0.0.0.0", 5200).expect("endpoint should be valid");
        let fullname = advertiser
            .start_advertisement(&id, &endpoint)
            .expect("advertisement should start");
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut found = false;
        while Instant::now() < deadline {
            if matches!(
                browser.poll_event(Duration::from_millis(250)),
                Ok(Some(DiscoveryBackendEvent::Resolved { id: found_id, .. })) if found_id == id
            ) {
                found = true;
                break;
            }
        }

        let _stop_advertisement = advertiser.stop_advertisement(&fullname);
        let _stop_browser = browser.shutdown();
        let _stop_advertiser = advertiser.shutdown();
        assert!(found);
    }
}

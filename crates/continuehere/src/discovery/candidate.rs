use std::{fmt, net::IpAddr, str::FromStr};

use crate::models::ProtocolVersion;

use super::DiscoveryError;

pub(crate) const MAX_DISCOVERY_IDENTIFIER_SIZE: usize = 128;
const MAX_ENDPOINT_INPUT_SIZE: usize = 512;
const MAX_HOST_SIZE: usize = 253;
const MAX_CANDIDATE_ENDPOINTS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiscoveryCandidateId(String);

impl DiscoveryCandidateId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, DiscoveryError> {
        let value = value.into();
        let value = value.trim();
        if value.is_empty() || value.len() > MAX_DISCOVERY_IDENTIFIER_SIZE {
            return Err(DiscoveryError::InvalidCandidateIdentifier);
        }

        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiscoveryCandidateId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiscoveryEndpoint {
    host: String,
    port: u16,
}

impl DiscoveryEndpoint {
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, DiscoveryError> {
        let host = normalize_host(host.into())?;
        if port == 0 {
            return Err(DiscoveryError::InvalidEndpointPort);
        }

        Ok(Self { host, port })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub const fn port(&self) -> u16 {
        self.port
    }
}

impl fmt::Display for DiscoveryEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') {
            write!(formatter, "[{}]:{}", self.host, self.port)
        } else {
            write!(formatter, "{}:{}", self.host, self.port)
        }
    }
}

impl FromStr for DiscoveryEndpoint {
    type Err = DiscoveryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            return Err(DiscoveryError::EmptyEndpoint);
        }
        if value.len() > MAX_ENDPOINT_INPUT_SIZE {
            return Err(DiscoveryError::EndpointTooLong);
        }

        if let Some(value) = value.strip_prefix('[') {
            let Some((host, port)) = value.split_once("]:") else {
                return Err(DiscoveryError::MissingEndpointPort);
            };
            let port = port
                .parse::<u16>()
                .map_err(|_| DiscoveryError::InvalidEndpointPort)?;
            return Self::new(host, port);
        }

        if let Ok(socket) = value.parse::<std::net::SocketAddr>() {
            return Self::new(socket.ip().to_string(), socket.port());
        }

        let Some((host, port)) = value.rsplit_once(':') else {
            return Err(DiscoveryError::MissingEndpointPort);
        };
        if host.starts_with('[') || host.ends_with(']') || host.contains(':') {
            return Err(DiscoveryError::InvalidEndpointHost);
        }
        let port = port
            .parse::<u16>()
            .map_err(|_| DiscoveryError::InvalidEndpointPort)?;
        Self::new(host, port)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiscoverySource {
    Local,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryCandidate {
    id: DiscoveryCandidateId,
    endpoints: Vec<DiscoveryEndpoint>,
    protocol_version: ProtocolVersion,
    source: DiscoverySource,
}

impl DiscoveryCandidate {
    pub(crate) fn new<I>(
        id: DiscoveryCandidateId,
        endpoints: I,
        protocol_version: ProtocolVersion,
        source: DiscoverySource,
    ) -> Result<Self, DiscoveryError>
    where
        I: IntoIterator<Item = DiscoveryEndpoint>,
    {
        let mut endpoints = endpoints.into_iter().collect::<Vec<_>>();
        endpoints.sort();
        endpoints.dedup();
        if endpoints.is_empty() {
            return Err(DiscoveryError::MissingCandidateEndpoint);
        }
        if endpoints.len() > MAX_CANDIDATE_ENDPOINTS {
            return Err(DiscoveryError::CandidateEndpointLimit);
        }

        Ok(Self {
            id,
            endpoints,
            protocol_version,
            source,
        })
    }

    pub fn id(&self) -> &DiscoveryCandidateId {
        &self.id
    }

    pub fn endpoints(&self) -> &[DiscoveryEndpoint] {
        &self.endpoints
    }

    pub const fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub const fn source(&self) -> DiscoverySource {
        self.source
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiscoveryMode {
    LocalBrowse,
    ManualEndpoint(DiscoveryEndpoint),
    AdvertiseEndpoint(DiscoveryEndpoint),
}

fn normalize_host(host: String) -> Result<String, DiscoveryError> {
    let host = host.trim().trim_end_matches('.');
    if host.is_empty() || host.len() > MAX_HOST_SIZE || !host.is_ascii() {
        return Err(DiscoveryError::InvalidEndpointHost);
    }
    if host.parse::<IpAddr>().is_ok() {
        return Ok(host.to_ascii_lowercase());
    }
    if let Some((address, scope)) = host.split_once('%') {
        let valid_address = address.parse::<std::net::Ipv6Addr>().is_ok();
        let valid_scope = scope.parse::<u32>().is_ok_and(|scope| scope > 0);
        if valid_address && valid_scope {
            return Ok(host.to_ascii_lowercase());
        }
        return Err(DiscoveryError::InvalidEndpointHost);
    }
    if host.contains(':')
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|value| value.is_ascii_alphanumeric() || value == b'-')
        })
    {
        return Err(DiscoveryError::InvalidEndpointHost);
    }

    Ok(host.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{DiscoveryEndpoint, DiscoveryError};

    #[test]
    fn parses_supported_manual_endpoints() {
        let ipv4 = "192.168.1.20:5200"
            .parse::<DiscoveryEndpoint>()
            .expect("IPv4 endpoint should be valid");
        let ipv6 = "[fe80::1]:5200"
            .parse::<DiscoveryEndpoint>()
            .expect("IPv6 endpoint should be valid");
        let scoped_ipv6 = "[fe80::1%3]:5200"
            .parse::<DiscoveryEndpoint>()
            .expect("scoped IPv6 endpoint should be valid");
        let hostname = "Desktop.Local.:5200"
            .parse::<DiscoveryEndpoint>()
            .expect("hostname endpoint should be valid");

        assert_eq!(ipv4.host(), "192.168.1.20");
        assert_eq!(ipv6.host(), "fe80::1");
        assert_eq!(ipv6.to_string(), "[fe80::1]:5200");
        assert_eq!(scoped_ipv6.host(), "fe80::1%3");
        assert_eq!(scoped_ipv6.to_string(), "[fe80::1%3]:5200");
        assert_eq!(hostname.host(), "desktop.local");
    }

    #[test]
    fn rejects_invalid_manual_endpoints() {
        assert_eq!(
            "desktop.local".parse::<DiscoveryEndpoint>(),
            Err(DiscoveryError::MissingEndpointPort)
        );
        assert_eq!(
            "desktop.local:0".parse::<DiscoveryEndpoint>(),
            Err(DiscoveryError::InvalidEndpointPort)
        );
        assert_eq!(
            "user@desktop.local:5200".parse::<DiscoveryEndpoint>(),
            Err(DiscoveryError::InvalidEndpointHost)
        );
    }
}

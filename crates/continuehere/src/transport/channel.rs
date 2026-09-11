use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};

use rustls::{
    ClientConfig, ClientConnection, ServerConfig, ServerConnection, StreamOwned,
    pki_types::ServerName, version::TLS13,
};
use sha2::{Digest, Sha256};
use x509_parser::parse_x509_certificate;

use crate::{discovery::DiscoveryEndpoint, security::CryptographicIdentity};

use super::{
    PairingMessage, TransportError,
    tls::{certificate_chain, private_key, provider},
    verifier::{UntrustedClientVerifier, UntrustedServerVerifier},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IO_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_PAIRING_FRAME_SIZE: usize = 1024;
const TLS_BUFFER_LIMIT: usize = 16 * 1024;
const PAIRING_ALPN: &[u8] = b"continuehere-pairing/2";

pub(crate) enum PairingChannel {
    Client(StreamOwned<ClientConnection, TcpStream>),
    Server(StreamOwned<ServerConnection, TcpStream>),
}

impl PairingChannel {
    pub(crate) fn connect(
        endpoint: &DiscoveryEndpoint,
        identity: &CryptographicIdentity,
    ) -> Result<Self, TransportError> {
        let address = endpoint
            .to_string()
            .to_socket_addrs()
            .map_err(|_| TransportError::ConnectionFailed)?
            .next()
            .ok_or(TransportError::ConnectionFailed)?;
        let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
            .map_err(|_| TransportError::ConnectionFailed)?;
        configure_stream(&stream)?;
        let provider = provider();
        let verifier = Arc::new(UntrustedServerVerifier::new(Arc::clone(&provider)));
        let mut config = ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&TLS13])
            .map_err(|_| TransportError::TlsConfigurationFailed)?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(certificate_chain(identity), private_key(identity))
            .map_err(|_| TransportError::TlsConfigurationFailed)?;
        config.alpn_protocols = vec![PAIRING_ALPN.to_vec()];
        config.enable_early_data = false;
        config.resumption = rustls::client::Resumption::disabled();
        let server_name = ServerName::try_from("continuehere.invalid".to_owned())
            .map_err(|_| TransportError::TlsConfigurationFailed)?;
        let mut connection = ClientConnection::new(Arc::new(config), server_name)
            .map_err(|_| TransportError::TlsConfigurationFailed)?;
        connection.set_buffer_limit(Some(TLS_BUFFER_LIMIT));
        let mut channel = Self::Client(StreamOwned::new(connection, stream));
        channel.complete_handshake()?;
        Ok(channel)
    }

    pub(crate) fn accept(
        stream: TcpStream,
        identity: &CryptographicIdentity,
    ) -> Result<Self, TransportError> {
        configure_stream(&stream)?;
        let provider = provider();
        let verifier = Arc::new(UntrustedClientVerifier::new(Arc::clone(&provider)));
        let mut config = ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&TLS13])
            .map_err(|_| TransportError::TlsConfigurationFailed)?
            .with_client_cert_verifier(verifier)
            .with_single_cert(certificate_chain(identity), private_key(identity))
            .map_err(|_| TransportError::TlsConfigurationFailed)?;
        config.alpn_protocols = vec![PAIRING_ALPN.to_vec()];
        let mut connection = ServerConnection::new(Arc::new(config))
            .map_err(|_| TransportError::TlsConfigurationFailed)?;
        connection.set_buffer_limit(Some(TLS_BUFFER_LIMIT));
        let mut channel = Self::Server(StreamOwned::new(connection, stream));
        channel.complete_handshake()?;
        Ok(channel)
    }

    pub(crate) fn send(&mut self, message: &PairingMessage) -> Result<(), TransportError> {
        let payload = message.encode()?;
        if payload.len() > MAX_PAIRING_FRAME_SIZE {
            return Err(TransportError::MessageTooLarge);
        }
        let length = u32::try_from(payload.len()).map_err(|_| TransportError::MessageTooLarge)?;
        self.write_all(&length.to_be_bytes())?;
        self.write_all(&payload)?;
        self.flush()
    }

    pub(crate) fn receive(&mut self) -> Result<PairingMessage, TransportError> {
        let mut length = [0_u8; 4];
        self.read_exact(&mut length)?;
        let length = usize::try_from(u32::from_be_bytes(length))
            .map_err(|_| TransportError::MessageTooLarge)?;
        if length == 0 || length > MAX_PAIRING_FRAME_SIZE {
            return Err(TransportError::MessageTooLarge);
        }
        let mut payload = vec![0_u8; length];
        self.read_exact(&mut payload)?;
        PairingMessage::decode(&payload)
    }

    pub(crate) fn peer_fingerprint(&self) -> Result<[u8; 32], TransportError> {
        let certificates = match self {
            Self::Client(stream) => stream.conn.peer_certificates(),
            Self::Server(stream) => stream.conn.peer_certificates(),
        }
        .ok_or(TransportError::MissingPeerIdentity)?;
        if certificates.len() != 1 {
            return Err(TransportError::MissingPeerIdentity);
        }
        let (_, certificate) = parse_x509_certificate(certificates[0].as_ref())
            .map_err(|_| TransportError::MissingPeerIdentity)?;
        let digest = Sha256::digest(certificate.tbs_certificate.subject_pki.raw);
        let mut fingerprint = [0_u8; 32];
        fingerprint.copy_from_slice(&digest);
        Ok(fingerprint)
    }

    pub(crate) fn peer_endpoint(
        &self,
        connection_port: u16,
    ) -> Result<DiscoveryEndpoint, TransportError> {
        let address = match self {
            Self::Client(stream) => stream.sock.peer_addr(),
            Self::Server(stream) => stream.sock.peer_addr(),
        }
        .map_err(|_| TransportError::ConnectionFailed)?;
        DiscoveryEndpoint::new(address.ip().to_string(), connection_port)
            .map_err(|_| TransportError::InvalidMessage)
    }

    pub(crate) fn export_authentication(&self, context: &[u8]) -> Result<[u8; 32], TransportError> {
        let output = [0_u8; 32];
        let output = match self {
            Self::Client(stream) => stream.conn.export_keying_material(
                output,
                b"EXPORTER-ContinueHere-Pairing-v1",
                Some(context),
            ),
            Self::Server(stream) => stream.conn.export_keying_material(
                output,
                b"EXPORTER-ContinueHere-Pairing-v1",
                Some(context),
            ),
        }
        .map_err(|_| TransportError::TlsHandshakeFailed)?;
        Ok(output)
    }

    pub(crate) fn set_timeout(&self, timeout: Duration) -> Result<(), TransportError> {
        let stream = match self {
            Self::Client(stream) => &stream.sock,
            Self::Server(stream) => &stream.sock,
        };
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|_| TransportError::ConnectionFailed)?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|_| TransportError::ConnectionFailed)
    }

    fn complete_handshake(&mut self) -> Result<(), TransportError> {
        match self {
            Self::Client(stream) => stream
                .conn
                .complete_io(&mut stream.sock)
                .map_err(map_io_error),
            Self::Server(stream) => stream
                .conn
                .complete_io(&mut stream.sock)
                .map_err(map_io_error),
        }?;
        Ok(())
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        match self {
            Self::Client(stream) => stream.write_all(bytes),
            Self::Server(stream) => stream.write_all(bytes),
        }
        .map_err(map_io_error)
    }

    fn flush(&mut self) -> Result<(), TransportError> {
        match self {
            Self::Client(stream) => stream.flush(),
            Self::Server(stream) => stream.flush(),
        }
        .map_err(map_io_error)
    }

    fn read_exact(&mut self, bytes: &mut [u8]) -> Result<(), TransportError> {
        match self {
            Self::Client(stream) => stream.read_exact(bytes),
            Self::Server(stream) => stream.read_exact(bytes),
        }
        .map_err(map_io_error)
    }
}

fn configure_stream(stream: &TcpStream) -> Result<(), TransportError> {
    stream
        .set_nonblocking(false)
        .map_err(|_| TransportError::ConnectionFailed)?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| TransportError::ConnectionFailed)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| TransportError::ConnectionFailed)?;
    stream
        .set_nodelay(true)
        .map_err(|_| TransportError::ConnectionFailed)
}

fn map_io_error(error: std::io::Error) -> TransportError {
    if error.kind() == std::io::ErrorKind::TimedOut
        || error.kind() == std::io::ErrorKind::WouldBlock
    {
        return TransportError::TimedOut;
    }
    TransportError::ConnectionFailed
}

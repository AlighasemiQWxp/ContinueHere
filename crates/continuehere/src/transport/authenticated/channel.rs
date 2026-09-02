use std::{sync::Arc, time::Duration};

use rustls::{
    ClientConfig, ServerConfig,
    pki_types::{CertificateDer, ServerName},
    version::TLS13,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    time::timeout,
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::{
    discovery::DiscoveryEndpoint,
    pairing::{TrustedDevice, TrustedPeerLookup},
    security::CryptographicIdentity,
};

use super::super::{
    TransportError,
    protocol::{self, ProtocolEnvelope},
    tls::{certificate_chain, private_key, provider},
    verifier::{PinnedServerVerifier, TrustedClientVerifier, certificate_fingerprint},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const TLS_BUFFER_LIMIT: usize = 128 * 1024;
const APPLICATION_ALPN: &[u8] = b"continuehere/1";

trait AsyncTlsStream: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T> AsyncTlsStream for T where T: AsyncRead + AsyncWrite + Send + Unpin {}

type BoxedTlsStream = Box<dyn AsyncTlsStream>;

pub(crate) struct AuthenticatedChannel {
    stream: BoxedTlsStream,
    peer_fingerprint: [u8; 32],
}

pub(crate) struct AuthenticatedReader {
    stream: ReadHalf<BoxedTlsStream>,
}

pub(crate) struct AuthenticatedWriter {
    stream: WriteHalf<BoxedTlsStream>,
}

impl AuthenticatedChannel {
    pub(crate) async fn connect(
        endpoint: &DiscoveryEndpoint,
        identity: &CryptographicIdentity,
        trusted_peer: &TrustedDevice,
    ) -> Result<Self, TransportError> {
        let address = tokio::net::lookup_host(endpoint.to_string())
            .await
            .map_err(|_| TransportError::ConnectionFailed)?
            .next()
            .ok_or(TransportError::ConnectionFailed)?;
        let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(address))
            .await
            .map_err(|_| TransportError::TimedOut)?
            .map_err(|_| TransportError::ConnectionFailed)?;
        configure_stream(&stream)?;

        let provider = provider();
        let verifier = Arc::new(PinnedServerVerifier::new(
            Arc::clone(&provider),
            *trusted_peer.public_key_fingerprint(),
        ));
        let mut config = ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&TLS13])
            .map_err(|_| TransportError::TlsConfigurationFailed)?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(certificate_chain(identity), private_key(identity))
            .map_err(|_| TransportError::TlsConfigurationFailed)?;
        config.alpn_protocols = vec![APPLICATION_ALPN.to_vec()];
        config.enable_early_data = false;
        config.resumption = rustls::client::Resumption::disabled();
        let connector = TlsConnector::from(Arc::new(config));
        let server_name = ServerName::try_from("continuehere.invalid".to_owned())
            .map_err(|_| TransportError::TlsConfigurationFailed)?;
        let mut stream = timeout(HANDSHAKE_TIMEOUT, connector.connect(server_name, stream))
            .await
            .map_err(|_| TransportError::TimedOut)?
            .map_err(|_| TransportError::TlsHandshakeFailed)?;
        if stream.get_ref().1.alpn_protocol() != Some(APPLICATION_ALPN) {
            return Err(TransportError::TlsHandshakeFailed);
        }
        let peer_fingerprint = peer_fingerprint(stream.get_ref().1.peer_certificates())?;
        stream.get_mut().1.set_buffer_limit(Some(TLS_BUFFER_LIMIT));
        Ok(Self {
            stream: Box::new(stream),
            peer_fingerprint,
        })
    }

    pub(crate) async fn accept(
        stream: TcpStream,
        identity: &CryptographicIdentity,
        trusted_peers: TrustedPeerLookup,
    ) -> Result<Self, TransportError> {
        configure_stream(&stream)?;
        let provider = provider();
        let verifier = Arc::new(TrustedClientVerifier::new(
            Arc::clone(&provider),
            trusted_peers,
        ));
        let mut config = ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&TLS13])
            .map_err(|_| TransportError::TlsConfigurationFailed)?
            .with_client_cert_verifier(verifier)
            .with_single_cert(certificate_chain(identity), private_key(identity))
            .map_err(|_| TransportError::TlsConfigurationFailed)?;
        config.alpn_protocols = vec![APPLICATION_ALPN.to_vec()];
        config.max_early_data_size = 0;
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let mut stream = timeout(HANDSHAKE_TIMEOUT, acceptor.accept(stream))
            .await
            .map_err(|_| TransportError::TimedOut)?
            .map_err(|_| TransportError::TlsHandshakeFailed)?;
        if stream.get_ref().1.alpn_protocol() != Some(APPLICATION_ALPN) {
            return Err(TransportError::TlsHandshakeFailed);
        }
        let peer_fingerprint = peer_fingerprint(stream.get_ref().1.peer_certificates())?;
        stream.get_mut().1.set_buffer_limit(Some(TLS_BUFFER_LIMIT));
        Ok(Self {
            stream: Box::new(stream),
            peer_fingerprint,
        })
    }

    pub(crate) async fn send(
        &mut self,
        envelope: &ProtocolEnvelope,
        maximum_frame_size: usize,
    ) -> Result<(), TransportError> {
        let payload = protocol::encode(envelope)?;
        if payload.is_empty() || payload.len() > maximum_frame_size {
            return Err(TransportError::MessageTooLarge);
        }
        let length = u32::try_from(payload.len()).map_err(|_| TransportError::MessageTooLarge)?;
        self.stream
            .write_all(&length.to_be_bytes())
            .await
            .map_err(map_io_error)?;
        self.stream
            .write_all(&payload)
            .await
            .map_err(map_io_error)?;
        self.stream.flush().await.map_err(map_io_error)
    }

    pub(crate) async fn receive(
        &mut self,
        maximum_frame_size: usize,
    ) -> Result<ProtocolEnvelope, TransportError> {
        let mut length = [0_u8; 4];
        self.stream
            .read_exact(&mut length)
            .await
            .map_err(map_io_error)?;
        let length = usize::try_from(u32::from_be_bytes(length))
            .map_err(|_| TransportError::MessageTooLarge)?;
        if length == 0 || length > maximum_frame_size {
            return Err(TransportError::MessageTooLarge);
        }
        let mut payload = vec![0_u8; length];
        self.stream
            .read_exact(&mut payload)
            .await
            .map_err(map_io_error)?;
        protocol::decode(&payload)
    }

    pub(crate) const fn peer_fingerprint(&self) -> [u8; 32] {
        self.peer_fingerprint
    }

    pub(crate) fn split(self) -> (AuthenticatedReader, AuthenticatedWriter) {
        let (reader, writer) = tokio::io::split(self.stream);
        (
            AuthenticatedReader { stream: reader },
            AuthenticatedWriter { stream: writer },
        )
    }
}

impl AuthenticatedReader {
    pub(crate) async fn receive(
        &mut self,
        maximum_frame_size: usize,
    ) -> Result<ProtocolEnvelope, TransportError> {
        let mut length = [0_u8; 4];
        self.stream
            .read_exact(&mut length)
            .await
            .map_err(map_io_error)?;
        let length = usize::try_from(u32::from_be_bytes(length))
            .map_err(|_| TransportError::MessageTooLarge)?;
        if length == 0 || length > maximum_frame_size {
            return Err(TransportError::MessageTooLarge);
        }
        let mut payload = vec![0_u8; length];
        self.stream
            .read_exact(&mut payload)
            .await
            .map_err(map_io_error)?;
        protocol::decode(&payload)
    }
}

impl AuthenticatedWriter {
    pub(crate) async fn send(
        &mut self,
        envelope: &ProtocolEnvelope,
        maximum_frame_size: usize,
    ) -> Result<(), TransportError> {
        let payload = protocol::encode(envelope)?;
        if payload.is_empty() || payload.len() > maximum_frame_size {
            return Err(TransportError::MessageTooLarge);
        }
        let length = u32::try_from(payload.len()).map_err(|_| TransportError::MessageTooLarge)?;
        self.stream
            .write_all(&length.to_be_bytes())
            .await
            .map_err(map_io_error)?;
        self.stream
            .write_all(&payload)
            .await
            .map_err(map_io_error)?;
        self.stream.flush().await.map_err(map_io_error)
    }
}

fn peer_fingerprint(
    certificates: Option<&[CertificateDer<'_>]>,
) -> Result<[u8; 32], TransportError> {
    let certificates = certificates.ok_or(TransportError::MissingPeerIdentity)?;
    if certificates.len() != 1 {
        return Err(TransportError::MissingPeerIdentity);
    }
    certificate_fingerprint(&certificates[0]).map_err(|_| TransportError::MissingPeerIdentity)
}

fn configure_stream(stream: &TcpStream) -> Result<(), TransportError> {
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

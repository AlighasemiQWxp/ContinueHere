use std::{fmt, sync::Arc};

use rustls::{
    DigitallySignedStruct, DistinguishedName, Error, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
    server::danger::{ClientCertVerified, ClientCertVerifier},
};
use x509_parser::parse_x509_certificate;

use crate::pairing::TrustedPeerLookup;

pub(crate) struct UntrustedServerVerifier {
    provider: Arc<CryptoProvider>,
}

pub(crate) struct UntrustedClientVerifier {
    provider: Arc<CryptoProvider>,
}

pub(crate) struct PinnedServerVerifier {
    provider: Arc<CryptoProvider>,
    expected_fingerprint: [u8; 32],
}

pub(crate) struct TrustedClientVerifier {
    provider: Arc<CryptoProvider>,
    trusted_peers: TrustedPeerLookup,
}

impl UntrustedServerVerifier {
    pub(crate) fn new(provider: Arc<CryptoProvider>) -> Self {
        Self { provider }
    }
}

impl UntrustedClientVerifier {
    pub(crate) fn new(provider: Arc<CryptoProvider>) -> Self {
        Self { provider }
    }
}

impl PinnedServerVerifier {
    pub(crate) fn new(provider: Arc<CryptoProvider>, expected_fingerprint: [u8; 32]) -> Self {
        Self {
            provider,
            expected_fingerprint,
        }
    }
}

impl TrustedClientVerifier {
    pub(crate) fn new(provider: Arc<CryptoProvider>, trusted_peers: TrustedPeerLookup) -> Self {
        Self {
            provider,
            trusted_peers,
        }
    }
}

impl fmt::Debug for UntrustedServerVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UntrustedServerVerifier")
    }
}

impl fmt::Debug for UntrustedClientVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UntrustedClientVerifier")
    }
}

impl fmt::Debug for PinnedServerVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PinnedServerVerifier")
    }
}

impl fmt::Debug for TrustedClientVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TrustedClientVerifier")
    }
}

impl ServerCertVerifier for UntrustedServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        validate_untrusted_chain(end_entity, intermediates)?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

impl ClientCertVerifier for UntrustedClientVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        validate_untrusted_chain(end_entity, intermediates)?;
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

impl ServerCertVerifier for PinnedServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        validate_untrusted_chain(end_entity, intermediates)?;
        let fingerprint = certificate_fingerprint(end_entity)?;
        if fingerprint != self.expected_fingerprint {
            return Err(Error::General("server identity is not trusted".to_owned()));
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

impl ClientCertVerifier for TrustedClientVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        validate_untrusted_chain(end_entity, intermediates)?;
        let fingerprint = certificate_fingerprint(end_entity)?;
        let trusted = self
            .trusted_peers
            .find_by_fingerprint(&fingerprint)
            .map_err(|_| Error::General("trusted-device state is unavailable".to_owned()))?
            .is_some();
        if !trusted {
            return Err(Error::General("client identity is not trusted".to_owned()));
        }
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn validate_untrusted_chain(
    end_entity: &CertificateDer<'_>,
    intermediates: &[CertificateDer<'_>],
) -> Result<(), Error> {
    let parsed = parse_x509_certificate(end_entity.as_ref());
    let is_ed25519 = parsed.as_ref().is_ok_and(|(_, certificate)| {
        certificate
            .tbs_certificate
            .subject_pki
            .algorithm
            .algorithm
            .to_id_string()
            == "1.3.101.112"
            && certificate
                .tbs_certificate
                .subject_pki
                .subject_public_key
                .data
                .len()
                == 32
    });
    if !intermediates.is_empty() || !is_ed25519 {
        return Err(Error::InvalidCertificate(
            rustls::CertificateError::BadEncoding,
        ));
    }
    Ok(())
}

pub(crate) fn certificate_fingerprint(certificate: &CertificateDer<'_>) -> Result<[u8; 32], Error> {
    use sha2::{Digest, Sha256};

    let (_, certificate) = parse_x509_certificate(certificate.as_ref())
        .map_err(|_| Error::General("peer certificate is malformed".to_owned()))?;
    let digest = Sha256::digest(certificate.tbs_certificate.subject_pki.raw);
    let mut fingerprint = [0_u8; 32];
    fingerprint.copy_from_slice(&digest);
    Ok(fingerprint)
}

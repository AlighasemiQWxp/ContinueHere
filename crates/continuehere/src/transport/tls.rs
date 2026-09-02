use std::sync::Arc;

use rustls::{
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};

use crate::security::CryptographicIdentity;

pub(crate) fn certificate_chain(identity: &CryptographicIdentity) -> Vec<CertificateDer<'static>> {
    vec![CertificateDer::from(identity.certificate().to_vec())]
}

pub(crate) fn private_key(identity: &CryptographicIdentity) -> PrivateKeyDer<'static> {
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.private_key().to_vec()))
}

pub(crate) fn provider() -> Arc<CryptoProvider> {
    let mut provider = rustls::crypto::ring::default_provider();
    provider
        .kx_groups
        .retain(|group| group.name() == rustls::NamedGroup::X25519);
    Arc::new(provider)
}

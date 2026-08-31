use rcgen::{CertificateParams, KeyPair, PKCS_ED25519};
use sha2::{Digest, Sha256};
use x509_parser::parse_x509_certificate;
use zeroize::Zeroizing;

use super::SecurityError;

pub(crate) struct CryptographicIdentity {
    certificate: Vec<u8>,
    private_key: Zeroizing<Vec<u8>>,
    fingerprint: [u8; 32],
}

impl CryptographicIdentity {
    pub(crate) fn generate() -> Result<Self, SecurityError> {
        let key_pair = KeyPair::generate_for(&PKCS_ED25519)
            .map_err(|_| SecurityError::IdentityGenerationFailed)?;
        let private_key = Zeroizing::new(key_pair.serialize_der());
        Self::from_private_key(private_key)
    }

    pub(crate) fn from_stored(private_key: Vec<u8>) -> Result<Self, SecurityError> {
        Self::from_private_key(Zeroizing::new(private_key))
            .map_err(|_| SecurityError::InvalidStoredIdentity)
    }

    pub(crate) fn certificate(&self) -> &[u8] {
        &self.certificate
    }

    pub(crate) fn private_key(&self) -> &[u8] {
        &self.private_key
    }

    pub(crate) const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    fn from_private_key(private_key: Zeroizing<Vec<u8>>) -> Result<Self, SecurityError> {
        let key_pair = KeyPair::try_from(private_key.as_slice())
            .map_err(|_| SecurityError::TlsIdentityFailed)?;
        if key_pair.algorithm() != &PKCS_ED25519 {
            return Err(SecurityError::TlsIdentityFailed);
        }
        let parameters = CertificateParams::new(vec!["continuehere.invalid".to_owned()])
            .map_err(|_| SecurityError::TlsIdentityFailed)?;
        let certificate = parameters
            .self_signed(&key_pair)
            .map_err(|_| SecurityError::TlsIdentityFailed)?
            .der()
            .to_vec();
        let (_, parsed) =
            parse_x509_certificate(&certificate).map_err(|_| SecurityError::TlsIdentityFailed)?;
        let digest = Sha256::digest(parsed.tbs_certificate.subject_pki.raw);
        let mut fingerprint = [0_u8; 32];
        fingerprint.copy_from_slice(&digest);
        Ok(Self {
            certificate,
            private_key,
            fingerprint,
        })
    }
}

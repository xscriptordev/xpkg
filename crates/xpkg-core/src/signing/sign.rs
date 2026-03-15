//! Create detached OpenPGP signatures for files.
//!
//! Produces a binary detached signature (`.sig`) alongside the target file.
//! The signature uses the signing-capable subkey from the provided secret key.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sequoia_openpgp::crypto::KeyPair;
use sequoia_openpgp::policy::StandardPolicy;
use sequoia_openpgp::serialize::stream::{Armorer, Message, Signer};
use sequoia_openpgp::types::HashAlgorithm;
use sequoia_openpgp::Cert;

use crate::error::{XpkgError, XpkgResult};

/// Result of a successful signing operation.
#[derive(Debug)]
pub struct SignResult {
    /// Path to the generated `.sig` file.
    pub sig_path: PathBuf,
    /// Size of the signature file in bytes.
    pub sig_size: u64,
    /// Key ID that was used for signing.
    pub key_id: String,
}

/// Sign a file, producing a detached signature at `<path>.sig`.
///
/// The `signer_cert` must contain an unencrypted signing-capable secret key.
/// If `armored` is true the signature is ASCII-armored; otherwise binary.
pub fn sign_file(file_path: &Path, signer_cert: &Cert, armored: bool) -> XpkgResult<SignResult> {
    let policy = StandardPolicy::new();

    // Find a signing-capable secret subkey.
    let keypair = signer_cert
        .keys()
        .with_policy(&policy, None)
        .supported()
        .alive()
        .revoked(false)
        .for_signing()
        .secret()
        .next()
        .ok_or_else(|| XpkgError::SigningError("no valid signing subkey found".into()))?
        .key()
        .clone()
        .into_keypair()
        .map_err(|e| XpkgError::SigningError(format!("extract signing keypair: {e}")))?;

    let key_id = keypair.public().keyid().to_hex();

    let data = std::fs::read(file_path)
        .map_err(|e| XpkgError::SigningError(format!("read {}: {e}", file_path.display())))?;

    let sig_bytes = create_detached_sig(&data, keypair, armored)?;

    let sig_path = file_path.with_extension(format!(
        "{}.sig",
        file_path.extension().unwrap_or_default().to_string_lossy()
    ));

    std::fs::write(&sig_path, &sig_bytes)
        .map_err(|e| XpkgError::SigningError(format!("write sig: {e}")))?;

    let sig_size = sig_bytes.len() as u64;

    Ok(SignResult {
        sig_path,
        sig_size,
        key_id,
    })
}

/// Create a detached signature in memory.
pub fn create_detached_sig(data: &[u8], keypair: KeyPair, armored: bool) -> XpkgResult<Vec<u8>> {
    let mut sig_buf = Vec::new();

    let message = Message::new(&mut sig_buf);

    let message = if armored {
        Armorer::new(message)
            .kind(sequoia_openpgp::armor::Kind::Signature)
            .build()
            .map_err(|e| XpkgError::SigningError(format!("armorer: {e}")))?
    } else {
        message
    };

    let mut signer = Signer::new(message, keypair)
        .detached()
        .hash_algo(HashAlgorithm::SHA512)
        .map_err(|e| XpkgError::SigningError(format!("init signer: {e}")))?
        .creation_time(SystemTime::now())
        .build()
        .map_err(|e| XpkgError::SigningError(format!("build signer: {e}")))?;

    signer
        .write_all(data)
        .map_err(|e| XpkgError::SigningError(format!("write data to signer: {e}")))?;

    signer
        .finalize()
        .map_err(|e| XpkgError::SigningError(format!("finalize signature: {e}")))?;

    Ok(sig_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_openpgp::cert::CertBuilder;

    fn generate_signing_key() -> Cert {
        let (cert, _) = CertBuilder::general_purpose(None, Some("Packager <packager@x.org>"))
            .generate()
            .unwrap();
        cert
    }

    #[test]
    fn test_sign_file_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let cert = generate_signing_key();

        let file = tmp.path().join("test.xp");
        std::fs::write(&file, b"fake package content").unwrap();

        let result = sign_file(&file, &cert, false).unwrap();
        assert!(result.sig_path.exists());
        assert!(result.sig_size > 0);
        assert!(!result.key_id.is_empty());
    }

    #[test]
    fn test_sign_file_armored() {
        let tmp = tempfile::tempdir().unwrap();
        let cert = generate_signing_key();

        let file = tmp.path().join("test.xp");
        std::fs::write(&file, b"fake package content").unwrap();

        let result = sign_file(&file, &cert, true).unwrap();
        let sig_content = std::fs::read_to_string(&result.sig_path).unwrap();
        assert!(sig_content.contains("-----BEGIN PGP SIGNATURE-----"));
    }

    #[test]
    fn test_create_detached_sig_in_memory() {
        let cert = generate_signing_key();
        let policy = StandardPolicy::new();

        let keypair = cert
            .keys()
            .with_policy(&policy, None)
            .supported()
            .alive()
            .revoked(false)
            .for_signing()
            .secret()
            .next()
            .unwrap()
            .key()
            .clone()
            .into_keypair()
            .unwrap();

        let data = b"test data to sign";
        let sig = create_detached_sig(data, keypair, false).unwrap();
        assert!(!sig.is_empty());
    }
}

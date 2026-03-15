//! Verify detached OpenPGP signatures.
//!
//! Given a data file and its `.sig` companion, checks whether the signature
//! is valid according to a supplied public certificate.

use std::path::Path;

use sequoia_openpgp::parse::stream::{
    DetachedVerifierBuilder, MessageLayer, MessageStructure, VerificationHelper,
};
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::policy::StandardPolicy;
use sequoia_openpgp::{Cert, KeyHandle};

use crate::error::{XpkgError, XpkgResult};

/// Outcome of a signature verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// Signature is valid and was made by the given key.
    Good { key_id: String },
    /// Signature exists but no matching key was found.
    UnknownKey,
    /// Signature is invalid or tampered.
    Bad { reason: String },
}

/// Verify a detached signature for `file_path` using certificates in `certs`.
///
/// The signature is read from `sig_path`.
pub fn verify_file(file_path: &Path, sig_path: &Path, certs: &[Cert]) -> XpkgResult<VerifyOutcome> {
    let data = std::fs::read(file_path)
        .map_err(|e| XpkgError::SigningError(format!("read {}: {e}", file_path.display())))?;

    let sig_bytes = std::fs::read(sig_path)
        .map_err(|e| XpkgError::SigningError(format!("read sig {}: {e}", sig_path.display())))?;

    verify_detached(&data, &sig_bytes, certs)
}

/// Verify a detached signature in memory.
pub fn verify_detached(data: &[u8], sig_bytes: &[u8], certs: &[Cert]) -> XpkgResult<VerifyOutcome> {
    let policy = StandardPolicy::new();
    let helper = VHelper::new(certs);

    let mut verifier = DetachedVerifierBuilder::from_bytes(sig_bytes)
        .map_err(|e| XpkgError::SigningError(format!("parse signature: {e}")))?
        .with_policy(&policy, None, helper)
        .map_err(|e| XpkgError::SigningError(format!("init verifier: {e}")))?;

    verifier
        .verify_bytes(data)
        .map_err(|e| XpkgError::SigningError(format!("verify: {e}")))?;

    // If we got here without error, check the helper's results.
    let helper = verifier.into_helper();

    match &helper.result {
        Some(outcome) => Ok(outcome.clone()),
        None => Ok(VerifyOutcome::Bad {
            reason: "no signature found".into(),
        }),
    }
}

// ── Verification helper ─────────────────────────────────────────────────────

struct VHelper {
    certs: Vec<Cert>,
    result: Option<VerifyOutcome>,
}

impl VHelper {
    fn new(certs: &[Cert]) -> Self {
        Self {
            certs: certs.to_vec(),
            result: None,
        }
    }
}

impl VerificationHelper for VHelper {
    fn get_certs(&mut self, _ids: &[KeyHandle]) -> sequoia_openpgp::Result<Vec<Cert>> {
        Ok(self.certs.clone())
    }

    fn check(&mut self, structure: MessageStructure) -> sequoia_openpgp::Result<()> {
        for layer in structure {
            if let MessageLayer::SignatureGroup { results } = layer {
                for result in results {
                    match result {
                        Ok(good) => {
                            let key_id = good.ka.key().keyid().to_hex();
                            self.result = Some(VerifyOutcome::Good { key_id });
                            return Ok(());
                        }
                        Err(e) => {
                            let msg = format!("{e}");
                            if msg.contains("no binding signature") {
                                self.result = Some(VerifyOutcome::UnknownKey);
                            } else {
                                self.result = Some(VerifyOutcome::Bad { reason: msg });
                            }
                        }
                    }
                }
            }
        }

        if self.result.is_none() {
            self.result = Some(VerifyOutcome::Bad {
                reason: "no signature results".into(),
            });
        }

        // Return error if verification failed, so the caller propagates.
        match &self.result {
            Some(VerifyOutcome::Good { .. }) => Ok(()),
            Some(VerifyOutcome::Bad { reason }) => Err(anyhow::anyhow!("bad signature: {reason}")),
            Some(VerifyOutcome::UnknownKey) => Err(anyhow::anyhow!("unknown signing key")),
            None => Err(anyhow::anyhow!("no verification result")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::sign::{create_detached_sig, sign_file};
    use sequoia_openpgp::cert::CertBuilder;

    fn generate_key() -> Cert {
        let (cert, _) =
            CertBuilder::general_purpose(None, Some("Verify Test <verify@example.com>"))
                .generate()
                .unwrap();
        cert
    }

    #[test]
    fn test_verify_good_signature() {
        let cert = generate_key();
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

        let data = b"package data here";
        let sig = create_detached_sig(data, keypair, false).unwrap();

        let result = verify_detached(data, &sig, &[cert]).unwrap();
        assert!(matches!(result, VerifyOutcome::Good { .. }));
    }

    #[test]
    fn test_verify_tampered_data() {
        let cert = generate_key();
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

        let data = b"original data";
        let sig = create_detached_sig(data, keypair, false).unwrap();

        let tampered = b"tampered data";
        let result = verify_detached(tampered, &sig, &[cert]);
        // Should fail verification.
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_wrong_key() {
        let signing_key = generate_key();
        let wrong_key = generate_key();
        let policy = StandardPolicy::new();

        let keypair = signing_key
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

        let data = b"some data";
        let sig = create_detached_sig(data, keypair, false).unwrap();

        // Verify with the wrong public key.
        let result = verify_detached(data, &sig, &[wrong_key]);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_file_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let cert = generate_key();

        let file = tmp.path().join("test.xp");
        std::fs::write(&file, b"package content").unwrap();

        let sign_result = sign_file(&file, &cert, false).unwrap();
        let verify_result = verify_file(&file, &sign_result.sig_path, &[cert]).unwrap();
        assert!(matches!(verify_result, VerifyOutcome::Good { .. }));
    }
}

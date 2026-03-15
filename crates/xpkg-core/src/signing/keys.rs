//! Load OpenPGP keys from files.
//!
//! Supports both ASCII-armored and binary key files for certificates
//! (public keys) and transferable secret keys (TSK).

use std::path::Path;

use sequoia_openpgp::cert::CertParser;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::Cert;

use crate::error::{XpkgError, XpkgResult};

/// Load a certificate (public key) from an ASCII-armored or binary file.
pub fn load_cert(path: &Path) -> XpkgResult<Cert> {
    let data = std::fs::read(path)
        .map_err(|e| XpkgError::SigningError(format!("read key {}: {e}", path.display())))?;

    Cert::from_bytes(&data)
        .map_err(|e| XpkgError::SigningError(format!("parse certificate {}: {e}", path.display())))
}

/// Load a secret key (TSK) from an ASCII-armored or binary file.
///
/// The returned [`Cert`] contains the secret key material.
pub fn load_secret_key(path: &Path) -> XpkgResult<Cert> {
    let data = std::fs::read(path)
        .map_err(|e| XpkgError::SigningError(format!("read secret key {}: {e}", path.display())))?;

    let cert = Cert::from_bytes(&data).map_err(|e| {
        XpkgError::SigningError(format!("parse secret key {}: {e}", path.display()))
    })?;

    // Verify that it actually contains secret key material.
    if cert.keys().secret().next().is_none() {
        return Err(XpkgError::SigningError(format!(
            "{} does not contain a secret key",
            path.display()
        )));
    }

    Ok(cert)
}

/// Load all certificates from a keyring file (may contain multiple certs).
pub fn load_keyring(path: &Path) -> XpkgResult<Vec<Cert>> {
    let data = std::fs::read(path)
        .map_err(|e| XpkgError::SigningError(format!("read keyring {}: {e}", path.display())))?;

    let parser = CertParser::from_bytes(&data)
        .map_err(|e| XpkgError::SigningError(format!("parse keyring {}: {e}", path.display())))?;

    let mut certs = Vec::new();
    for result in parser {
        let cert = result.map_err(|e| XpkgError::SigningError(format!("keyring entry: {e}")))?;
        certs.push(cert);
    }

    Ok(certs)
}

/// Find a certificate whose key ID (short hex) matches the given fingerprint
/// or key ID fragment.
pub fn find_cert_by_id<'a>(certs: &'a [Cert], key_id: &str) -> Option<&'a Cert> {
    let needle = key_id.to_uppercase();
    certs.iter().find(|cert| {
        let fpr = cert.fingerprint().to_hex();
        let kid = cert.keyid().to_hex();
        fpr.ends_with(&needle) || kid.ends_with(&needle)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_openpgp::cert::CertBuilder;
    use sequoia_openpgp::serialize::Serialize;

    fn generate_test_key() -> Cert {
        let (cert, _) = CertBuilder::general_purpose(None, Some("Test User <test@example.com>"))
            .generate()
            .unwrap();
        cert
    }

    #[test]
    fn test_load_cert_from_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cert = generate_test_key();

        let cert_path = tmp.path().join("test.pub");
        let mut file = std::fs::File::create(&cert_path).unwrap();
        cert.serialize(&mut file).unwrap();

        let loaded = load_cert(&cert_path).unwrap();
        assert_eq!(loaded.fingerprint(), cert.fingerprint());
    }

    #[test]
    fn test_load_secret_key_from_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cert = generate_test_key();

        let key_path = tmp.path().join("test.key");
        let mut file = std::fs::File::create(&key_path).unwrap();
        cert.as_tsk().serialize(&mut file).unwrap();

        let loaded = load_secret_key(&key_path).unwrap();
        assert!(loaded.keys().secret().next().is_some());
    }

    #[test]
    fn test_load_public_only_as_secret_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let cert = generate_test_key();

        // Strip secret key material.
        let pub_only = cert.strip_secret_key_material();

        let path = tmp.path().join("pub.key");
        let mut file = std::fs::File::create(&path).unwrap();
        pub_only.serialize(&mut file).unwrap();

        assert!(load_secret_key(&path).is_err());
    }

    #[test]
    fn test_find_cert_by_id() {
        let c1 = generate_test_key();
        let c2 = generate_test_key();
        let certs = vec![c1.clone(), c2];

        let kid = c1.keyid().to_hex();
        let last4 = &kid[kid.len() - 4..];

        let found = find_cert_by_id(&certs, last4);
        assert!(found.is_some());
        assert_eq!(found.unwrap().fingerprint(), c1.fingerprint());
    }

    #[test]
    fn test_load_keyring() {
        let tmp = tempfile::tempdir().unwrap();
        let c1 = generate_test_key();
        let c2 = generate_test_key();

        let ring_path = tmp.path().join("keyring.gpg");
        let mut file = std::fs::File::create(&ring_path).unwrap();
        c1.serialize(&mut file).unwrap();
        c2.serialize(&mut file).unwrap();

        let loaded = load_keyring(&ring_path).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_load_cert_nonexistent_file() {
        let result = load_cert(Path::new("/nonexistent/key.pub"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_cert_invalid_data() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("garbage.pub");
        std::fs::write(&path, b"not a valid key").unwrap();
        assert!(load_cert(&path).is_err());
    }

    #[test]
    fn test_find_cert_by_id_no_match() {
        let c1 = generate_test_key();
        let certs = vec![c1];
        assert!(find_cert_by_id(&certs, "0000DEAD").is_none());
    }
}

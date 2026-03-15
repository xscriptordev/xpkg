//! OpenPGP signing and verification for packages and repository databases.
//!
//! This module provides:
//!
//! - **keys** — load certificates and secret keys from files or keyrings
//! - **sign** — create detached signatures (`.sig`) for `.xp` archives and databases
//! - **verify** — verify detached signatures against public certificates

mod keys;
mod sign;
mod verify;

pub use keys::{find_cert_by_id, load_cert, load_keyring, load_secret_key};
pub use sign::{create_detached_sig, sign_file, SignResult};
pub use verify::{verify_detached, verify_file, VerifyOutcome};

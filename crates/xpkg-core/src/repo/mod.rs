//! Repository management — create, read, and modify package databases.
//!
//! A repository database is a compressed tar archive (`.db.tar.zst` by default)
//! that contains ALPM-compatible `desc` and `depends` files for each package.
//! This module provides:
//!
//! - **types** — [`RepoDb`], [`RepoEntry`], [`DbCompression`]
//! - **desc** — generate and parse `desc`/`depends` virtual files
//! - **db** — read/write database archives, add/remove entries
//! - **inspect** — build a [`RepoEntry`] from a `.xp` package on disk
//! - **deploy** — generate a static repository layout for HTTP hosting

mod db;
mod deploy;
mod desc;
mod inspect;
mod types;

// Re-export public API.
pub use db::{add_entry, read_db, remove_entry, write_db};
pub use deploy::{deploy_repo, DeployResult};
pub use inspect::entry_from_package;
pub use types::{DbCompression, RepoDb, RepoEntry};

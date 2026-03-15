//! Types for the repository database.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Compression format for the repository database archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbCompression {
    Zstd,
    Gzip,
    Xz,
}

impl DbCompression {
    /// File extension including the leading dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Zstd => ".tar.zst",
            Self::Gzip => ".tar.gz",
            Self::Xz => ".tar.xz",
        }
    }

    /// Detect compression from a file path suffix.
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        if name.ends_with(".tar.zst") {
            Some(Self::Zstd)
        } else if name.ends_with(".tar.gz") {
            Some(Self::Gzip)
        } else if name.ends_with(".tar.xz") {
            Some(Self::Xz)
        } else {
            None
        }
    }
}

/// A single package entry inside the repository database.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RepoEntry {
    // ── identity ────────────────────────────────────────────────────
    pub name: String,
    pub version: String,
    pub release: String,
    pub description: String,
    pub url: String,
    pub arch: String,
    pub license: String,

    // ── archive information ─────────────────────────────────────────
    pub filename: String,
    pub compressed_size: u64,
    pub installed_size: u64,
    pub sha256sum: String,
    pub build_date: u64,
    pub packager: String,

    // ── relations ───────────────────────────────────────────────────
    pub depends: Vec<String>,
    pub makedepends: Vec<String>,
    pub checkdepends: Vec<String>,
    pub optdepends: Vec<String>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
}

impl RepoEntry {
    /// The combined version string used as directory name: `name-version-release`.
    pub fn dir_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }

    /// Full version string: `version-release`.
    pub fn full_version(&self) -> String {
        format!("{}-{}", self.version, self.release)
    }
}

/// An in-memory representation of a repository database.
#[derive(Debug, Clone)]
pub struct RepoDb {
    /// Repository name (e.g. "xrepo").
    pub name: String,
    /// Compression format for the output archive.
    pub compression: DbCompression,
    /// Package entries keyed by package name.
    pub entries: BTreeMap<String, RepoEntry>,
    /// Filesystem path of the database archive.
    pub db_path: PathBuf,
}

impl RepoDb {
    /// Create a new empty repository database.
    pub fn new(name: impl Into<String>, db_path: PathBuf) -> Self {
        let compression = DbCompression::from_path(&db_path).unwrap_or(DbCompression::Zstd);
        Self {
            name: name.into(),
            compression,
            entries: BTreeMap::new(),
            db_path,
        }
    }

    /// Number of packages in the database.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the database contains any entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_db_compression_from_path() {
        assert_eq!(
            DbCompression::from_path(Path::new("myrepo.db.tar.zst")),
            Some(DbCompression::Zstd)
        );
        assert_eq!(
            DbCompression::from_path(Path::new("myrepo.db.tar.gz")),
            Some(DbCompression::Gzip)
        );
        assert_eq!(
            DbCompression::from_path(Path::new("myrepo.db.tar.xz")),
            Some(DbCompression::Xz)
        );
        assert_eq!(DbCompression::from_path(Path::new("myrepo.db")), None);
    }

    #[test]
    fn test_repo_entry_dir_name() {
        let entry = RepoEntry {
            name: "hello".into(),
            version: "1.0.0".into(),
            release: "1".into(),
            description: String::new(),
            url: String::new(),
            arch: "x86_64".into(),
            license: String::new(),
            filename: String::new(),
            compressed_size: 0,
            installed_size: 0,
            sha256sum: String::new(),
            build_date: 0,
            packager: String::new(),
            depends: vec![],
            makedepends: vec![],
            checkdepends: vec![],
            optdepends: vec![],
            provides: vec![],
            conflicts: vec![],
            replaces: vec![],
        };
        assert_eq!(entry.dir_name(), "hello-1.0.0-1");
        assert_eq!(entry.full_version(), "1.0.0-1");
    }

    #[test]
    fn test_repo_db_new_empty() {
        let db = RepoDb::new("xrepo", PathBuf::from("xrepo.db.tar.zst"));
        assert_eq!(db.name, "xrepo");
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
        assert_eq!(db.compression, DbCompression::Zstd);
    }
}

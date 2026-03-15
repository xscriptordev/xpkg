//! Core types for the build engine.

use std::path::PathBuf;
use std::time::Duration;

/// The four build phases defined by a recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPhase {
    Prepare,
    Build,
    Check,
    Package,
}

impl std::fmt::Display for BuildPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepare => write!(f, "prepare"),
            Self::Build => write!(f, "build"),
            Self::Check => write!(f, "check"),
            Self::Package => write!(f, "package"),
        }
    }
}

/// Options controlling build behavior (from CLI flags and config).
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    /// Skip the check() phase.
    pub skip_check: bool,
    /// Keep the build directory after completion (don't cleanup).
    pub keep_builddir: bool,
}

/// Resolved paths and state for a single build.
#[derive(Debug, Clone)]
pub struct BuildContext {
    /// Root of the build tree: `{builddir}/{pkgname}-{version}`.
    pub build_root: PathBuf,
    /// Source directory: `{build_root}/src`.
    pub srcdir: PathBuf,
    /// Package install directory: `{build_root}/pkg`.
    pub pkgdir: PathBuf,
    /// Directory containing the recipe file.
    pub startdir: PathBuf,
    /// Package name from recipe.
    pub pkgname: String,
    /// Package version from recipe.
    pub pkgver: String,
    /// Package release from recipe.
    pub pkgrel: u32,
}

/// Result of a successful build.
#[derive(Debug)]
pub struct BuildResult {
    /// Path to the populated package directory (PKGDIR).
    pub pkgdir: PathBuf,
    /// Package name.
    pub pkgname: String,
    /// Package version.
    pub pkgver: String,
    /// Package release.
    pub pkgrel: u32,
    /// Total build duration.
    pub duration: Duration,
}

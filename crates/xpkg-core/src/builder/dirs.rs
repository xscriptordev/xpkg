//! Build directory setup and cleanup.
//!
//! Creates the isolated directory tree for a single package build:
//! ```text
//! {builddir}/{pkgname}-{version}/
//! ├── src/    ← SRCDIR (extracted sources, working directory for build)
//! └── pkg/    ← PKGDIR (package() installs files here)
//! ```

use std::path::{Path, PathBuf};

use crate::error::{XpkgError, XpkgResult};

use super::types::BuildContext;

/// Create the build directory tree and return a [`BuildContext`].
///
/// The build root is `{builddir}/{pkgname}-{version}`. If it already exists
/// it is removed first to ensure a clean build environment.
pub fn setup_build_dirs(
    builddir: &Path,
    startdir: &Path,
    pkgname: &str,
    pkgver: &str,
    pkgrel: u32,
) -> XpkgResult<BuildContext> {
    let dir_name = format!("{pkgname}-{pkgver}");
    let build_root = builddir.join(&dir_name);

    // Clean previous build if present.
    if build_root.exists() {
        tracing::info!(path = %build_root.display(), "removing previous build directory");
        std::fs::remove_dir_all(&build_root).map_err(|e| {
            XpkgError::BuildFailed(format!(
                "failed to remove old build directory {}: {e}",
                build_root.display()
            ))
        })?;
    }

    let srcdir = build_root.join("src");
    let pkgdir = build_root.join("pkg");

    std::fs::create_dir_all(&srcdir).map_err(|e| {
        XpkgError::BuildFailed(format!(
            "failed to create source directory {}: {e}",
            srcdir.display()
        ))
    })?;

    std::fs::create_dir_all(&pkgdir).map_err(|e| {
        XpkgError::BuildFailed(format!(
            "failed to create package directory {}: {e}",
            pkgdir.display()
        ))
    })?;

    tracing::info!(
        build_root = %build_root.display(),
        srcdir = %srcdir.display(),
        pkgdir = %pkgdir.display(),
        "build directories created"
    );

    Ok(BuildContext {
        build_root,
        srcdir,
        pkgdir,
        startdir: startdir.to_path_buf(),
        pkgname: pkgname.to_string(),
        pkgver: pkgver.to_string(),
        pkgrel,
    })
}

/// Remove the build directory tree.
#[allow(dead_code)] // Used in future phases and by callers.
pub fn cleanup_build_dirs(build_root: &Path) -> XpkgResult<()> {
    if build_root.exists() {
        tracing::info!(path = %build_root.display(), "cleaning up build directory");
        std::fs::remove_dir_all(build_root).map_err(|e| {
            XpkgError::BuildFailed(format!(
                "failed to clean up build directory {}: {e}",
                build_root.display()
            ))
        })?;
    }
    Ok(())
}

/// Populate SRCDIR by copying extracted source files into it.
///
/// If `source_dir` contains a single top-level directory (common for tarballs),
/// its contents are moved up so that SRCDIR directly contains the source tree.
pub fn populate_srcdir(srcdir: &Path, source_dir: &Path) -> XpkgResult<()> {
    if !source_dir.exists() {
        tracing::debug!(
            path = %source_dir.display(),
            "no extracted sources to populate"
        );
        return Ok(());
    }

    copy_dir_contents(source_dir, srcdir).map_err(|e| {
        XpkgError::BuildFailed(format!(
            "failed to populate SRCDIR from {}: {e}",
            source_dir.display()
        ))
    })
}

/// Recursively copy contents of `src` into `dst`.
fn copy_dir_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// ── Helpers for canonical paths ─────────────────────────────────────────────

/// Resolve a path to its canonical absolute form, creating it if needed.
#[allow(dead_code)] // Used in future phases.
pub fn ensure_absolute(path: &Path) -> XpkgResult<PathBuf> {
    std::fs::create_dir_all(path).map_err(|e| {
        XpkgError::BuildFailed(format!(
            "failed to create directory {}: {e}",
            path.display()
        ))
    })?;
    path.canonicalize().map_err(|e| {
        XpkgError::BuildFailed(format!(
            "failed to resolve absolute path for {}: {e}",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_setup_creates_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let startdir = tmp.path().join("recipe");
        fs::create_dir_all(&startdir).unwrap();

        let ctx = setup_build_dirs(tmp.path(), &startdir, "hello", "1.0", 1).unwrap();

        assert!(ctx.srcdir.exists());
        assert!(ctx.pkgdir.exists());
        assert!(ctx.build_root.ends_with("hello-1.0"));
        assert_eq!(ctx.pkgname, "hello");
        assert_eq!(ctx.pkgver, "1.0");
        assert_eq!(ctx.pkgrel, 1);
    }

    #[test]
    fn test_setup_cleans_previous_build() {
        let tmp = tempfile::tempdir().unwrap();
        let startdir = tmp.path().join("recipe");
        fs::create_dir_all(&startdir).unwrap();

        // Create a stale build.
        let stale = tmp.path().join("hello-1.0").join("leftover");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join("file.txt"), "stale").unwrap();

        let ctx = setup_build_dirs(tmp.path(), &startdir, "hello", "1.0", 1).unwrap();

        assert!(ctx.srcdir.exists());
        assert!(!tmp.path().join("hello-1.0/leftover").exists());
    }

    #[test]
    fn test_cleanup_removes_build_root() {
        let tmp = tempfile::tempdir().unwrap();
        let startdir = tmp.path().join("recipe");
        fs::create_dir_all(&startdir).unwrap();

        let ctx = setup_build_dirs(tmp.path(), &startdir, "hello", "1.0", 1).unwrap();
        assert!(ctx.build_root.exists());

        cleanup_build_dirs(&ctx.build_root).unwrap();
        assert!(!ctx.build_root.exists());
    }

    #[test]
    fn test_cleanup_noop_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nonexistent");
        assert!(cleanup_build_dirs(&missing).is_ok());
    }

    #[test]
    fn test_populate_srcdir_copies_files() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let srcdir = tmp.path().join("srcdir");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&srcdir).unwrap();

        fs::write(source.join("main.c"), "int main() {}").unwrap();
        fs::create_dir_all(source.join("lib")).unwrap();
        fs::write(source.join("lib/util.c"), "void util() {}").unwrap();

        populate_srcdir(&srcdir, &source).unwrap();

        assert!(srcdir.join("main.c").exists());
        assert!(srcdir.join("lib/util.c").exists());
    }
}

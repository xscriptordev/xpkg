//! Path checks.
//!
//! Detects files installed to non-standard or disallowed directories,
//! empty directories, and other filesystem layout issues.

use std::path::Path;

use crate::error::XpkgResult;

use super::rules::{LintResult, Severity};

/// Directories that should not appear in packages.
const FORBIDDEN_PATHS: &[&str] = &[
    "usr/local",
    "var/local",
    "opt",
    "home",
    "root",
    "tmp",
    "run",
    "dev",
    "proc",
    "sys",
    "mnt",
    "media",
];

/// Standard top-level directories that packages may install into.
const ALLOWED_TOPLEVEL: &[&str] = &["usr", "etc", "var", "srv", "boot", "opt"];

/// Run path checks on the package directory.
pub fn check_paths(pkgdir: &Path, result: &mut LintResult) -> XpkgResult<()> {
    check_forbidden_paths(pkgdir, result)?;
    check_empty_dirs(pkgdir, pkgdir, result)?;
    check_toplevel(pkgdir, result)?;
    Ok(())
}

/// Check for files in forbidden directories.
fn check_forbidden_paths(pkgdir: &Path, result: &mut LintResult) -> XpkgResult<()> {
    for forbidden in FORBIDDEN_PATHS {
        let path = pkgdir.join(forbidden);
        if path.exists() {
            result.add(
                Severity::Error,
                "paths-forbidden-directory",
                &format!("package installs files to forbidden directory /{forbidden}"),
                Some(forbidden),
            );
        }
    }
    Ok(())
}

/// Check for empty directories (warnings, not errors).
fn check_empty_dirs(root: &Path, current: &Path, result: &mut LintResult) -> XpkgResult<()> {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    let items: Vec<_> = entries.flatten().collect();

    if items.is_empty() && current != root {
        let rel = current
            .strip_prefix(root)
            .unwrap_or(current)
            .to_string_lossy()
            .to_string();
        result.add(
            Severity::Info,
            "paths-empty-directory",
            "empty directory in package",
            Some(&rel),
        );
        return Ok(());
    }

    for entry in items {
        let path = entry.path();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() && !meta.file_type().is_symlink() {
            check_empty_dirs(root, &path, result)?;
        }
    }

    Ok(())
}

/// Check that top-level directories are from the standard set.
fn check_toplevel(pkgdir: &Path, result: &mut LintResult) -> XpkgResult<()> {
    let entries = match std::fs::read_dir(pkgdir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip metadata files.
        if name.starts_with('.') {
            continue;
        }

        let meta = match std::fs::symlink_metadata(entry.path()) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() && !ALLOWED_TOPLEVEL.contains(&name.as_str()) {
            result.add(
                Severity::Warning,
                "paths-non-standard-toplevel",
                &format!("non-standard top-level directory /{name}"),
                Some(&name),
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_package_no_path_issues() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/bin")).unwrap();
        std::fs::write(pkgdir.join("usr/bin/hello"), "bin").unwrap();

        let mut result = LintResult::new();
        check_paths(pkgdir, &mut result).unwrap();
        assert_eq!(result.count(Severity::Error), 0);
        assert_eq!(result.count(Severity::Warning), 0);
    }

    #[test]
    fn test_forbidden_usr_local() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/local/bin")).unwrap();
        std::fs::write(pkgdir.join("usr/local/bin/hello"), "bin").unwrap();

        let mut result = LintResult::new();
        check_paths(pkgdir, &mut result).unwrap();
        assert!(result.has_errors());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.rule == "paths-forbidden-directory"));
    }

    #[test]
    fn test_empty_directory_info() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/share/empty")).unwrap();
        // usr/share/empty is empty.

        let mut result = LintResult::new();
        check_paths(pkgdir, &mut result).unwrap();
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.rule == "paths-empty-directory"));
    }

    #[test]
    fn test_non_standard_toplevel() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("weird")).unwrap();
        std::fs::write(pkgdir.join("weird/file"), "data").unwrap();

        let mut result = LintResult::new();
        check_paths(pkgdir, &mut result).unwrap();
        assert!(result.has_warnings());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.rule == "paths-non-standard-toplevel"));
    }
}

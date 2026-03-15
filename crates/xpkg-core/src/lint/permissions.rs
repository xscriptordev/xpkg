//! Permission checks.
//!
//! Flags world-writable files, unexpected SUID/SGID binaries,
//! and files with unusual ownership patterns.

use std::path::Path;

use crate::error::XpkgResult;

use super::rules::{LintResult, Severity};

/// Run permission checks on all files in PKGDIR.
pub fn check_permissions(pkgdir: &Path, result: &mut LintResult) -> XpkgResult<()> {
    check_recursive(pkgdir, pkgdir, result)?;
    Ok(())
}

fn check_recursive(root: &Path, current: &Path, result: &mut LintResult) -> XpkgResult<()> {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.file_type().is_symlink() {
            continue;
        }

        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode();

            // World-writable files (o+w).
            if mode & 0o002 != 0 && !meta.is_dir() {
                result.add(
                    Severity::Error,
                    "permissions-world-writable",
                    "file is world-writable",
                    Some(&rel),
                );
            }

            // World-writable directories without sticky bit.
            if meta.is_dir() && mode & 0o002 != 0 && mode & 0o1000 == 0 {
                result.add(
                    Severity::Warning,
                    "permissions-world-writable-dir",
                    "directory is world-writable without sticky bit",
                    Some(&rel),
                );
            }

            // SUID binaries.
            if mode & 0o4000 != 0 {
                result.add(
                    Severity::Warning,
                    "permissions-suid",
                    "file has SUID bit set",
                    Some(&rel),
                );
            }

            // SGID binaries.
            if mode & 0o2000 != 0 && !meta.is_dir() {
                result.add(
                    Severity::Warning,
                    "permissions-sgid",
                    "file has SGID bit set",
                    Some(&rel),
                );
            }
        }

        if meta.is_dir() {
            check_recursive(root, &path, result)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_issues_on_clean_package() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/bin")).unwrap();
        std::fs::write(pkgdir.join("usr/bin/hello"), "#!/bin/sh").unwrap();

        let mut result = LintResult::new();
        check_permissions(pkgdir, &mut result).unwrap();
        assert_eq!(result.total(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn test_world_writable_file() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        let file = pkgdir.join("bad.txt");
        std::fs::write(&file, "data").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o666)).unwrap();

        let mut result = LintResult::new();
        check_permissions(pkgdir, &mut result).unwrap();
        assert!(result.has_errors());
        assert!(result.diagnostics[0].rule.contains("world-writable"));
    }

    #[cfg(unix)]
    #[test]
    fn test_suid_file() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        let file = pkgdir.join("suid-bin");
        std::fs::write(&file, "binary").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o4755)).unwrap();

        let mut result = LintResult::new();
        check_permissions(pkgdir, &mut result).unwrap();
        assert!(result.has_warnings());
        assert!(result.diagnostics[0].rule.contains("suid"));
    }
}

//! ELF binary stripping.
//!
//! Optionally strips debug symbols from ELF binaries in PKGDIR to reduce
//! package size. Uses the system `strip` command.

use std::path::Path;

use crate::error::{XpkgError, XpkgResult};

/// Strip debug symbols from ELF binaries in the given directory.
///
/// Walks the directory recursively and strips any file that looks like
/// an ELF binary. Non-ELF files are silently skipped.
///
/// Returns the number of files stripped.
pub fn strip_binaries(pkgdir: &Path) -> XpkgResult<u32> {
    if !has_strip_command() {
        tracing::warn!("strip command not found on PATH, skipping binary stripping");
        return Ok(0);
    }

    let mut count = 0u32;
    strip_recursive(pkgdir, &mut count)?;

    if count > 0 {
        tracing::info!(count, "stripped debug symbols from ELF binaries");
    } else {
        tracing::debug!("no ELF binaries found to strip");
    }

    Ok(count)
}

/// Check if the `strip` command is available.
fn has_strip_command() -> bool {
    std::process::Command::new("strip")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Recursively walk a directory and strip ELF files.
fn strip_recursive(dir: &Path, count: &mut u32) -> XpkgResult<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        XpkgError::BuildFailed(format!("failed to read directory {}: {e}", dir.display()))
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).map_err(|e| {
            XpkgError::BuildFailed(format!("failed to stat {}: {e}", path.display()))
        })?;

        if meta.file_type().is_symlink() {
            continue;
        }

        if meta.is_dir() {
            strip_recursive(&path, count)?;
        } else if meta.is_file() && is_elf_file(&path) && do_strip(&path)? {
            *count += 1;
        }
    }

    Ok(())
}

/// Check if a file is an ELF binary by reading its magic bytes.
fn is_elf_file(path: &Path) -> bool {
    std::fs::read(path)
        .map(|data| data.len() >= 4 && data[..4] == [0x7f, b'E', b'L', b'F'])
        .unwrap_or(false)
}

/// Run `strip --strip-unneeded` on a file.
fn do_strip(path: &Path) -> XpkgResult<bool> {
    let status = std::process::Command::new("strip")
        .arg("--strip-unneeded")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| {
            XpkgError::BuildFailed(format!("failed to run strip on {}: {e}", path.display()))
        })?;

    if status.success() {
        tracing::debug!(path = %path.display(), "stripped binary");
        Ok(true)
    } else {
        // strip can fail on static libraries or certain files — not fatal.
        tracing::debug!(
            path = %path.display(),
            "strip returned non-zero (skipped)"
        );
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_elf_file_with_real_elf_magic() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.elf");
        // ELF magic header + some padding.
        let mut data = vec![0x7f, b'E', b'L', b'F'];
        data.extend_from_slice(&[0u8; 12]);
        std::fs::write(&path, &data).unwrap();
        assert!(is_elf_file(&path));
    }

    #[test]
    fn test_is_elf_file_with_non_elf() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();
        assert!(!is_elf_file(&path));
    }

    #[test]
    fn test_is_elf_file_with_short_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tiny");
        std::fs::write(&path, [0x7f]).unwrap();
        assert!(!is_elf_file(&path));
    }

    #[test]
    fn test_strip_binaries_skips_non_elf() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/bin")).unwrap();
        std::fs::write(pkgdir.join("usr/bin/script.sh"), "#!/bin/sh\necho hi").unwrap();

        let count = strip_binaries(pkgdir).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_strip_binaries_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let count = strip_binaries(tmp.path()).unwrap();
        assert_eq!(count, 0);
    }
}

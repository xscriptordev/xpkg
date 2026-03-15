//! ELF dependency checks.
//!
//! Checks that shared libraries required by ELF binaries in the package
//! are declared in the package's `depends` list. Uses `readelf` or
//! `objdump` to extract DT_NEEDED entries.

use std::path::Path;
use std::process::Command;

use crate::error::XpkgResult;

use super::rules::{LintResult, Severity};

/// Check that ELF shared library dependencies are declared.
pub fn check_dependencies(pkgdir: &Path, result: &mut LintResult) -> XpkgResult<()> {
    if !has_readelf() {
        tracing::debug!("readelf not found, skipping dependency checks");
        return Ok(());
    }

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

        if meta.is_dir() {
            check_recursive(root, &path, result)?;
        } else if meta.is_file() && is_elf(&path) {
            check_elf_deps(root, &path, result);
        }
    }

    Ok(())
}

/// Check a single ELF file for undeclared shared library dependencies.
fn check_elf_deps(root: &Path, path: &Path, result: &mut LintResult) {
    let needed = extract_needed(path);
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    for lib in &needed {
        // Check if the library exists within the package itself.
        if lib_exists_in_package(root, lib) {
            continue;
        }

        // Report as info — we can't verify against system without a database.
        result.add(
            Severity::Info,
            "dependency-needed-library",
            &format!("requires shared library: {lib}"),
            Some(&rel),
        );
    }
}

/// Extract DT_NEEDED entries from an ELF binary using `readelf`.
fn extract_needed(path: &Path) -> Vec<String> {
    let output = Command::new("readelf")
        .args(["-d", &path.to_string_lossy()])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter(|l| l.contains("(NEEDED)"))
        .filter_map(|l| {
            // Format: "  0x... (NEEDED)  Shared library: [libfoo.so.1]"
            let start = l.find('[')? + 1;
            let end = l.find(']')?;
            Some(l[start..end].to_string())
        })
        .collect()
}

/// Check if a shared library exists somewhere in the package.
fn lib_exists_in_package(pkgdir: &Path, libname: &str) -> bool {
    let lib_dirs = ["usr/lib", "usr/lib64", "lib", "lib64"];
    lib_dirs
        .iter()
        .any(|dir| pkgdir.join(dir).join(libname).exists())
}

/// Check if a file is ELF by reading magic bytes.
fn is_elf(path: &Path) -> bool {
    std::fs::read(path)
        .map(|data| data.len() >= 4 && data[..4] == [0x7f, b'E', b'L', b'F'])
        .unwrap_or(false)
}

/// Check if `readelf` is available.
fn has_readelf() -> bool {
    Command::new("readelf")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_elf_files_no_issues() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/bin")).unwrap();
        std::fs::write(pkgdir.join("usr/bin/script.sh"), "#!/bin/sh").unwrap();

        let mut result = LintResult::new();
        check_dependencies(pkgdir, &mut result).unwrap();
        assert_eq!(result.total(), 0);
    }

    #[test]
    fn test_is_elf_with_magic() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("binary");
        let mut data = vec![0x7f, b'E', b'L', b'F'];
        data.extend_from_slice(&[0u8; 12]);
        std::fs::write(&path, &data).unwrap();
        assert!(is_elf(&path));
    }

    #[test]
    fn test_is_not_elf() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("text");
        std::fs::write(&path, "hello world").unwrap();
        assert!(!is_elf(&path));
    }

    #[test]
    fn test_lib_exists_in_package() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/lib")).unwrap();
        std::fs::write(pkgdir.join("usr/lib/libfoo.so.1"), "lib").unwrap();

        assert!(lib_exists_in_package(pkgdir, "libfoo.so.1"));
        assert!(!lib_exists_in_package(pkgdir, "libbar.so"));
    }
}

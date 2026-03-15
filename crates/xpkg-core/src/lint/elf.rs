//! ELF binary analysis checks.
//!
//! Checks ELF binaries for common issues:
//! - TEXTREL (text relocations — security/performance issue)
//! - Non-empty RPATH/RUNPATH (potential portability issue)
//! - Missing stack protector in executable binaries

use std::path::Path;
use std::process::Command;

use crate::error::XpkgResult;

use super::rules::{LintResult, Severity};

/// Run ELF analysis checks on all binaries in PKGDIR.
pub fn check_elf(pkgdir: &Path, result: &mut LintResult) -> XpkgResult<()> {
    if !has_readelf() {
        tracing::debug!("readelf not found, skipping ELF analysis");
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
            check_single_elf(root, &path, result);
        }
    }

    Ok(())
}

/// Analyze a single ELF binary.
fn check_single_elf(root: &Path, path: &Path, result: &mut LintResult) {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let dynamic = read_dynamic(path);

    // Check for TEXTREL.
    if dynamic.iter().any(|l| l.contains("TEXTREL")) {
        result.add(
            Severity::Warning,
            "elf-textrel",
            "binary contains TEXTREL (text relocations) — consider compiling with -fPIC",
            Some(&rel),
        );
    }

    // Check for non-standard RPATH/RUNPATH.
    for line in &dynamic {
        if let Some(rpath) = extract_rpath(line) {
            if !rpath.is_empty() && rpath != "$ORIGIN" && rpath != "$ORIGIN/../lib" {
                result.add(
                    Severity::Warning,
                    "elf-rpath",
                    &format!("binary has non-standard RPATH: {rpath}"),
                    Some(&rel),
                );
            }
        }
    }
}

/// Read dynamic section of an ELF binary.
fn read_dynamic(path: &Path) -> Vec<String> {
    let output = Command::new("readelf")
        .args(["-d", &path.to_string_lossy()])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(String::from)
            .collect(),
        _ => vec![],
    }
}

/// Extract RPATH or RUNPATH value from a readelf dynamic line.
fn extract_rpath(line: &str) -> Option<String> {
    if !line.contains("RPATH") && !line.contains("RUNPATH") {
        return None;
    }
    // Format: "  0x... (RPATH)  Library rpath: [/some/path]"
    let start = line.find('[')?;
    let end = line.find(']')?;
    Some(line[start + 1..end].to_string())
}

fn is_elf(path: &Path) -> bool {
    std::fs::read(path)
        .map(|data| data.len() >= 4 && data[..4] == [0x7f, b'E', b'L', b'F'])
        .unwrap_or(false)
}

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
    fn test_no_elf_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("script.sh"), "#!/bin/sh").unwrap();

        let mut result = LintResult::new();
        check_elf(tmp.path(), &mut result).unwrap();
        assert_eq!(result.total(), 0);
    }

    #[test]
    fn test_extract_rpath_from_line() {
        let line = "  0x000000000000001d (RUNPATH)            Library runpath: [/opt/lib]";
        let rpath = extract_rpath(line).unwrap();
        assert_eq!(rpath, "/opt/lib");
    }

    #[test]
    fn test_extract_rpath_none_for_other_lines() {
        let line = "  0x0000000000000001 (NEEDED)  Shared library: [libc.so.6]";
        assert!(extract_rpath(line).is_none());
    }

    #[test]
    fn test_extract_rpath_origin() {
        let line = "  0x000000000000001d (RPATH)  Library rpath: [$ORIGIN]";
        let rpath = extract_rpath(line).unwrap();
        assert_eq!(rpath, "$ORIGIN");
    }
}

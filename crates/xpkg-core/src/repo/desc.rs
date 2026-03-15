//! Generate and parse ALPM-compatible `desc` and `depends` files.
//!
//! Each package entry in the repository database consists of two virtual files:
//!
//! - **desc** — identity, archive metadata, checksums.
//! - **depends** — runtime and build-time dependency lists.

use crate::error::XpkgResult;
use crate::repo::types::RepoEntry;

// ── Generation ──────────────────────────────────────────────────────────────

/// Render the `desc` file content for a repository entry.
pub fn generate_desc(entry: &RepoEntry) -> String {
    let mut out = String::with_capacity(512);

    write_field(&mut out, "FILENAME", &entry.filename);
    write_field(&mut out, "NAME", &entry.name);
    write_field(&mut out, "VERSION", &entry.full_version());
    write_field(&mut out, "DESC", &entry.description);
    write_field(&mut out, "CSIZE", &entry.compressed_size.to_string());
    write_field(&mut out, "ISIZE", &entry.installed_size.to_string());
    write_field(&mut out, "SHA256SUM", &entry.sha256sum);
    write_field(&mut out, "URL", &entry.url);
    write_field(&mut out, "LICENSE", &entry.license);
    write_field(&mut out, "ARCH", &entry.arch);
    write_field(&mut out, "BUILDDATE", &entry.build_date.to_string());
    write_field(&mut out, "PACKAGER", &entry.packager);

    out
}

/// Render the `depends` file content for a repository entry.
pub fn generate_depends(entry: &RepoEntry) -> String {
    let mut out = String::with_capacity(256);

    write_list(&mut out, "DEPENDS", &entry.depends);
    write_list(&mut out, "MAKEDEPENDS", &entry.makedepends);
    write_list(&mut out, "CHECKDEPENDS", &entry.checkdepends);
    write_list(&mut out, "OPTDEPENDS", &entry.optdepends);
    write_list(&mut out, "PROVIDES", &entry.provides);
    write_list(&mut out, "CONFLICTS", &entry.conflicts);
    write_list(&mut out, "REPLACES", &entry.replaces);

    out
}

fn write_field(buf: &mut String, key: &str, value: &str) {
    if !value.is_empty() {
        buf.push_str(&format!("%{key}%\n{value}\n\n"));
    }
}

fn write_list(buf: &mut String, key: &str, items: &[String]) {
    if !items.is_empty() {
        buf.push_str(&format!("%{key}%\n"));
        for item in items {
            buf.push_str(item);
            buf.push('\n');
        }
        buf.push('\n');
    }
}

// ── Parsing ─────────────────────────────────────────────────────────────────

/// Parse a `desc` + `depends` pair into a [`RepoEntry`].
///
/// Both contents are optional — missing sections are treated as empty.
pub fn parse_entry(desc: &str, depends: &str) -> XpkgResult<RepoEntry> {
    let desc_map = parse_sections(desc);
    let dep_map = parse_sections(depends);

    let version_full = get_one(&desc_map, "VERSION").unwrap_or_default();
    let (version, release) = split_version(&version_full);

    Ok(RepoEntry {
        name: get_one(&desc_map, "NAME").unwrap_or_default(),
        version,
        release,
        description: get_one(&desc_map, "DESC").unwrap_or_default(),
        url: get_one(&desc_map, "URL").unwrap_or_default(),
        arch: get_one(&desc_map, "ARCH").unwrap_or_default(),
        license: get_one(&desc_map, "LICENSE").unwrap_or_default(),
        filename: get_one(&desc_map, "FILENAME").unwrap_or_default(),
        compressed_size: get_one(&desc_map, "CSIZE")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        installed_size: get_one(&desc_map, "ISIZE")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        sha256sum: get_one(&desc_map, "SHA256SUM").unwrap_or_default(),
        build_date: get_one(&desc_map, "BUILDDATE")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        packager: get_one(&desc_map, "PACKAGER").unwrap_or_default(),
        depends: get_many(&dep_map, "DEPENDS"),
        makedepends: get_many(&dep_map, "MAKEDEPENDS"),
        checkdepends: get_many(&dep_map, "CHECKDEPENDS"),
        optdepends: get_many(&dep_map, "OPTDEPENDS"),
        provides: get_many(&dep_map, "PROVIDES"),
        conflicts: get_many(&dep_map, "CONFLICTS"),
        replaces: get_many(&dep_map, "REPLACES"),
    })
}

/// Split `"1.0.0-1"` into `("1.0.0", "1")`. If no dash, release defaults to `"1"`.
fn split_version(full: &str) -> (String, String) {
    if let Some(pos) = full.rfind('-') {
        (full[..pos].to_string(), full[pos + 1..].to_string())
    } else {
        (full.to_string(), "1".to_string())
    }
}

type SectionMap = std::collections::HashMap<String, Vec<String>>;

fn parse_sections(text: &str) -> SectionMap {
    let mut map = SectionMap::new();
    let mut current_key: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            current_key = None;
            continue;
        }
        if trimmed.starts_with('%') && trimmed.ends_with('%') {
            let key = trimmed[1..trimmed.len() - 1].to_string();
            current_key = Some(key.clone());
            map.entry(key).or_default();
        } else if let Some(ref key) = current_key {
            map.entry(key.clone())
                .or_default()
                .push(trimmed.to_string());
        }
    }

    map
}

fn get_one(map: &SectionMap, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.first()).cloned()
}

fn get_many(map: &SectionMap, key: &str) -> Vec<String> {
    map.get(key).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> RepoEntry {
        RepoEntry {
            name: "hello".into(),
            version: "1.0.0".into(),
            release: "1".into(),
            description: "Hello World program".into(),
            url: "https://example.com".into(),
            arch: "x86_64".into(),
            license: "MIT".into(),
            filename: "hello-1.0.0-1-x86_64.xp".into(),
            compressed_size: 4096,
            installed_size: 12288,
            sha256sum: "abc123".into(),
            build_date: 1700000000,
            packager: "Builder <builder@x.org>".into(),
            depends: vec!["glibc".into(), "gcc-libs".into()],
            makedepends: vec!["cmake".into()],
            checkdepends: vec![],
            optdepends: vec!["bash: for shell completion".into()],
            provides: vec![],
            conflicts: vec![],
            replaces: vec![],
        }
    }

    #[test]
    fn test_generate_desc() {
        let desc = generate_desc(&sample_entry());
        assert!(desc.contains("%NAME%\nhello\n"));
        assert!(desc.contains("%VERSION%\n1.0.0-1\n"));
        assert!(desc.contains("%CSIZE%\n4096\n"));
        assert!(desc.contains("%SHA256SUM%\nabc123\n"));
    }

    #[test]
    fn test_generate_depends() {
        let depends = generate_depends(&sample_entry());
        assert!(depends.contains("%DEPENDS%\nglibc\ngcc-libs\n"));
        assert!(depends.contains("%MAKEDEPENDS%\ncmake\n"));
        assert!(depends.contains("%OPTDEPENDS%\nbash: for shell completion\n"));
        assert!(!depends.contains("%CHECKDEPENDS%"));
        assert!(!depends.contains("%CONFLICTS%"));
    }

    #[test]
    fn test_roundtrip_parse() {
        let entry = sample_entry();
        let desc_text = generate_desc(&entry);
        let dep_text = generate_depends(&entry);
        let parsed = parse_entry(&desc_text, &dep_text).unwrap();

        assert_eq!(parsed.name, "hello");
        assert_eq!(parsed.version, "1.0.0");
        assert_eq!(parsed.release, "1");
        assert_eq!(parsed.description, "Hello World program");
        assert_eq!(parsed.compressed_size, 4096);
        assert_eq!(parsed.installed_size, 12288);
        assert_eq!(parsed.depends, vec!["glibc", "gcc-libs"]);
        assert_eq!(parsed.makedepends, vec!["cmake"]);
        assert_eq!(parsed.optdepends, vec!["bash: for shell completion"]);
    }

    #[test]
    fn test_parse_empty_strings() {
        let parsed = parse_entry("", "").unwrap();
        assert_eq!(parsed.name, "");
        assert!(parsed.depends.is_empty());
    }

    #[test]
    fn test_split_version() {
        let (v, r) = split_version("1.2.3-2");
        assert_eq!(v, "1.2.3");
        assert_eq!(r, "2");

        let (v, r) = split_version("1.0.0");
        assert_eq!(v, "1.0.0");
        assert_eq!(r, "1");
    }
}

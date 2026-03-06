//! Recipe module — XBUILD and PKGBUILD parsing for xpkg.
//!
//! This module handles reading, parsing, and validating build recipes
//! in both the native XBUILD (TOML) format and the legacy PKGBUILD
//! (bash) format for Arch Linux compatibility.

mod pkgbuild;
mod types;
mod validate;
mod xbuild;

pub use pkgbuild::parse_pkgbuild;
pub use types::{BuildSection, DependencySection, PackageSection, Recipe, SourceSection};
pub use validate::validate_recipe;
pub use xbuild::parse_xbuild;

/// Generate .SRCINFO-style text from a parsed recipe.
pub fn generate_srcinfo(recipe: &Recipe) -> String {
    let pkg = &recipe.package;
    let deps = &recipe.dependencies;
    let src = &recipe.source;

    let mut out = String::new();

    // pkgbase
    out.push_str(&format!("pkgbase = {}\n", pkg.name));

    // package section fields
    out.push_str(&format!("\tpkgdesc = {}\n", pkg.description));
    out.push_str(&format!("\tpkgver = {}\n", pkg.version));
    out.push_str(&format!("\tpkgrel = {}\n", pkg.release));

    if let Some(ref url) = pkg.url {
        out.push_str(&format!("\turl = {url}\n"));
    }

    for arch in &pkg.arch {
        out.push_str(&format!("\tarch = {arch}\n"));
    }

    for lic in &pkg.license {
        out.push_str(&format!("\tlicense = {lic}\n"));
    }

    // dependencies
    for dep in &deps.depends {
        out.push_str(&format!("\tdepends = {dep}\n"));
    }
    for dep in &deps.makedepends {
        out.push_str(&format!("\tmakedepends = {dep}\n"));
    }
    for dep in &deps.checkdepends {
        out.push_str(&format!("\tcheckdepends = {dep}\n"));
    }
    for dep in &deps.optdepends {
        out.push_str(&format!("\toptdepends = {dep}\n"));
    }

    // sources
    for url in &src.urls {
        out.push_str(&format!("\tsource = {url}\n"));
    }
    for sum in &src.sha256sums {
        out.push_str(&format!("\tsha256sums = {sum}\n"));
    }
    for sum in &src.sha512sums {
        out.push_str(&format!("\tsha512sums = {sum}\n"));
    }

    // pkgname
    out.push_str(&format!("\npkgname = {}\n", pkg.name));

    out
}

/// Generate a template XBUILD string for a given package name.
pub fn generate_template(pkgname: &str) -> String {
    format!(
        r#"[package]
name = "{pkgname}"
version = "0.1.0"
release = 1
description = "TODO: Package description"
url = "https://example.com/{pkgname}"
license = ["GPL-3.0-or-later"]
arch = ["x86_64"]

[dependencies]
depends = []
makedepends = []
checkdepends = []
optdepends = []

[source]
urls = [
    "https://example.com/{pkgname}/releases/{pkgname}-0.1.0.tar.gz",
]
sha256sums = [
    "SKIP",
]
# sha512sums = []
# patches = []

[build]
prepare = """
cd {pkgname}-0.1.0
# patch -p1 < ../fix.patch
"""

build = """
cd {pkgname}-0.1.0
# ./configure --prefix=/usr
# make
"""

check = """
cd {pkgname}-0.1.0
# make check
"""

package = """
cd {pkgname}-0.1.0
# make DESTDIR=$PKGDIR install
"""
"#
    )
}

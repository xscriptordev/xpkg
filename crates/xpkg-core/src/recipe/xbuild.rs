//! XBUILD parser — deserialize TOML into Recipe.

use std::path::Path;

use crate::error::XpkgError;
use crate::recipe::types::Recipe;

/// Parse an XBUILD file from a path.
pub fn parse_xbuild(path: &Path) -> Result<Recipe, XpkgError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| XpkgError::RecipeParse(format!("failed to read {}: {e}", path.display())))?;

    parse_xbuild_str(&contents)
}

/// Parse an XBUILD from a string (useful for testing).
pub fn parse_xbuild_str(input: &str) -> Result<Recipe, XpkgError> {
    let recipe: Recipe = toml::from_str(input)
        .map_err(|e| XpkgError::RecipeParse(format!("XBUILD parse error: {e}")))?;

    Ok(recipe)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_XBUILD: &str = r#"
[package]
name = "hello"
version = "1.0.0"
description = "Hello world"
"#;

    const FULL_XBUILD: &str = r#"
[package]
name = "example"
version = "2.1.0"
release = 3
description = "An example package"
url = "https://example.com"
license = ["MIT", "Apache-2.0"]
arch = ["x86_64", "aarch64"]
provides = ["libexample"]
conflicts = ["example-git"]
replaces = ["old-example"]

[dependencies]
depends = ["glibc", "openssl>=1.1"]
makedepends = ["cmake", "ninja"]
checkdepends = ["python"]
optdepends = ["docs: HTML documentation"]

[source]
urls = [
    "https://example.com/releases/example-2.1.0.tar.gz",
    "fix-build.patch",
]
sha256sums = [
    "abc123def456",
    "SKIP",
]
sha512sums = []
patches = ["fix-build.patch"]

[build]
prepare = """
cd example-2.1.0
patch -p1 < ../fix-build.patch
"""
build = """
cd example-2.1.0
cmake -B build -G Ninja
ninja -C build
"""
check = """
cd example-2.1.0
ninja -C build test
"""
package = """
cd example-2.1.0
DESTDIR=$PKGDIR ninja -C build install
"""
"#;

    #[test]
    fn test_parse_minimal_xbuild() {
        let recipe = parse_xbuild_str(MINIMAL_XBUILD).unwrap();
        assert_eq!(recipe.package.name, "hello");
        assert_eq!(recipe.package.version, "1.0.0");
        assert_eq!(recipe.package.release, 1); // default
        assert!(recipe.dependencies.depends.is_empty());
        assert!(recipe.source.urls.is_empty());
        assert!(recipe.build.build.is_empty());
    }

    #[test]
    fn test_parse_full_xbuild() {
        let recipe = parse_xbuild_str(FULL_XBUILD).unwrap();
        assert_eq!(recipe.package.name, "example");
        assert_eq!(recipe.package.version, "2.1.0");
        assert_eq!(recipe.package.release, 3);
        assert_eq!(recipe.package.license, vec!["MIT", "Apache-2.0"]);
        assert_eq!(recipe.package.arch, vec!["x86_64", "aarch64"]);
        assert_eq!(recipe.package.provides, vec!["libexample"]);
        assert_eq!(recipe.package.conflicts, vec!["example-git"]);

        assert_eq!(recipe.dependencies.depends, vec!["glibc", "openssl>=1.1"]);
        assert_eq!(recipe.dependencies.makedepends, vec!["cmake", "ninja"]);
        assert_eq!(recipe.dependencies.checkdepends, vec!["python"]);
        assert_eq!(
            recipe.dependencies.optdepends,
            vec!["docs: HTML documentation"]
        );

        assert_eq!(recipe.source.urls.len(), 2);
        assert_eq!(recipe.source.sha256sums, vec!["abc123def456", "SKIP"]);
        assert_eq!(recipe.source.patches, vec!["fix-build.patch"]);

        assert!(recipe.build.prepare.contains("patch -p1"));
        assert!(recipe.build.build.contains("cmake"));
        assert!(recipe.build.check.contains("test"));
        assert!(recipe.build.package.contains("DESTDIR=$PKGDIR"));
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = parse_xbuild_str("this is not valid toml [[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_fields() {
        // Missing name and version
        let result = parse_xbuild_str("[package]\ndescription = \"test\"");
        assert!(result.is_err());
    }
}

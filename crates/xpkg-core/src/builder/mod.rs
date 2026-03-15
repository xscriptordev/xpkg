//! Build engine for xpkg.
//!
//! This module implements the build pipeline that transforms a parsed recipe
//! and downloaded sources into a package directory ready for archiving.
//!
//! ## Pipeline
//!
//! 1. **Setup** — create isolated build directories (SRCDIR, PKGDIR)
//! 2. **Prepare** — run prepare() script (apply patches, configure)
//! 3. **Build** — run build() script (compile)
//! 4. **Check** — run check() script (test, optional)
//! 5. **Package** — run package() with fakeroot (install into PKGDIR)
//! 6. **Cleanup** — remove build directory (optional)

mod dirs;
mod env;
mod exec;
mod log;
mod pipeline;
mod types;

pub use pipeline::build_package;
pub use types::{BuildContext, BuildOptions, BuildPhase, BuildResult};

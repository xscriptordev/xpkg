# Packaging Guide

Step-by-step tutorial to build your first `.xp` package with xpkg.

---

## Prerequisites

- xpkg installed and in your `PATH` (see [Installation Guide](INSTALLATION.md))
- Build tools for your software (e.g. `gcc`, `make`, `cmake`, `cargo`)
- Source code or tarball of the software to package

---

## 1. Create the Recipe

Start with a template:

```bash
xpkg new mypackage
cd mypackage
```

This creates an `XBUILD` file with placeholders. Edit it for your project.

---

## 2. Write the XBUILD

### Minimal example — a Rust project

```toml
[package]
name = "xfetch"
version = "0.1.0"
release = 1
description = "System information tool written in Rust"
url = "https://github.com/xscriptor/xfetch"
license = ["GPL-3.0-or-later"]
arch = ["x86_64"]

[dependencies]
depends = ["glibc"]
makedepends = ["cargo", "gcc"]

[source]
urls = ["https://github.com/xscriptor/xfetch/archive/v0.1.0.tar.gz"]
sha256sums = ["SKIP"]

[build]
build = """
cd xfetch-0.1.0
cargo build --release
"""

package = """
cd xfetch-0.1.0
install -Dm755 target/release/xfetch "$PKGDIR/usr/bin/xfetch"
"""
```

### Minimal example — a C project with autotools

```toml
[package]
name = "hello"
version = "2.12"
release = 1
description = "GNU Hello — the friendly greeter"
url = "https://www.gnu.org/software/hello/"
license = ["GPL-3.0-or-later"]
arch = ["x86_64"]

[dependencies]
depends = ["glibc"]
makedepends = ["gcc", "make"]

[source]
urls = ["https://ftp.gnu.org/gnu/hello/hello-2.12.tar.gz"]
sha256sums = ["cf04af86dc085268c5f4470fbae49b18afbc221b78096aab842d934a76bad0ab"]

[build]
build = """
cd hello-2.12
./configure --prefix=/usr
make
"""

check = """
cd hello-2.12
make check
"""

package = """
cd hello-2.12
make DESTDIR=$PKGDIR install
"""
```

### Using a local source directory

If you have the source code locally and don't want to download it, leave
`[source]` empty and copy the files manually in `prepare`:

```toml
[source]
urls = []

[build]
prepare = """
cp -r /path/to/my/source ./my-source
"""

build = """
cd my-source
cargo build --release
"""

package = """
cd my-source
install -Dm755 target/release/mybinary "$PKGDIR/usr/bin/mybinary"
"""
```

---

## 3. Build the Package

```bash
xpkg build
```

This will:

1. Parse and validate the XBUILD
2. Download and verify sources (if declared)
3. Run `prepare()` → `build()` → `check()` → `package()` in order
4. Strip ELF binaries (if configured)
5. Create the `.xp` archive in the output directory

**Useful flags:**

```bash
xpkg build --no-check          # Skip the check() phase
xpkg build -o ./packages       # Output to a specific directory
xpkg build -d /tmp/mybuild     # Use a specific build directory
xpkg build --sign              # Sign the package after building
xpkg build -vv                 # Verbose output for debugging
```

---

## 4. Inspect the Result

```bash
# View package metadata
xpkg info mypackage-0.1.0-1-x86_64.xp

# List files inside the package
xpkg info mypackage-0.1.0-1-x86_64.xp --files

# Machine-readable output
xpkg info mypackage-0.1.0-1-x86_64.xp --json
```

---

## 5. Lint for Quality Issues

```bash
xpkg lint mypackage-0.1.0-1-x86_64.xp
```

The linter checks for:
- Files with incorrect permissions or ownership
- Files in non-standard paths
- Missing or incomplete `.PKGINFO`
- ELF binaries with missing dependencies
- Potential security issues

Fix any errors before distributing the package.

---

## 6. Install with xpm

```bash
sudo xpm install mypackage-0.1.0-1-x86_64.xp
```

---

## 7. Publish to a Repository (Optional)

```bash
# Add to a repository database
xpkg repo-add myrepo.db.tar.zst mypackage-0.1.0-1-x86_64.xp

# Sign the database
xpkg repo-add myrepo.db.tar.zst mypackage-0.1.0-1-x86_64.xp --sign
```

See [Repository Management](REPOSITORY.md) for hosting details.

---

## Build Environment Variables

During the build, these variables are available in your scripts:

| Variable | Description |
|----------|-------------|
| `$PKGDIR` | Install destination — treat this as the filesystem root |
| `$SRCDIR` | Directory containing extracted source files |
| `$BUILDDIR` | Top-level build working directory |
| `$MAKEFLAGS` | Make parallelism flags from config (e.g. `-j8`) |
| `$CFLAGS` | C compiler flags from config |
| `$CXXFLAGS` | C++ compiler flags from config |
| `$LDFLAGS` | Linker flags from config |

**Important:** Always install files to `$PKGDIR`, never to `/`. The
`package()` function runs under fakeroot so files are recorded as owned
by root without actually needing root privileges.

---

## The `package()` Function

This is the most important part of the recipe. It must install all files
into `$PKGDIR` as if it were the root filesystem. Common patterns:

```bash
# Using make DESTDIR
make DESTDIR=$PKGDIR install

# Using install(1) for individual files
install -Dm755 mybinary "$PKGDIR/usr/bin/mybinary"
install -Dm644 mylib.so "$PKGDIR/usr/lib/mylib.so"
install -Dm644 README.md "$PKGDIR/usr/share/doc/mypackage/README.md"
install -Dm644 mypackage.1 "$PKGDIR/usr/share/man/man1/mypackage.1"

# Using cmake --install
cmake --install build --prefix "$PKGDIR/usr"

# Using cargo install
cargo install --path . --root "$PKGDIR/usr"
rm "$PKGDIR/usr/.crates.toml" "$PKGDIR/usr/.crates2.json"
```

---

## Tips

- **Use `--no-check` during development** to speed up iteration
- **Use `-vv`** to see detailed build output when something fails
- **Use `SKIP` for checksums** during development, replace with real
  checksums before releasing
- **Run `xpkg lint`** before every release to catch common issues
- Files are automatically compressed with zstd level 19 by default;
  adjust `compress_level` in config for faster builds during development

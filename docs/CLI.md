# xpkg CLI Reference

Complete reference for all xpkg commands, flags, and usage patterns.

---

## Global Flags

These flags can be used with any subcommand.

| Flag | Short | Value | Description |
|------|-------|-------|-------------|
| `--config` | `-c` | `PATH` | Path to configuration file (default: `~/.config/xpkg/xpkg.conf`) |
| `--verbose` | `-v` | — | Increase verbosity (`-v`, `-vv`, `-vvv`) |
| `--no-confirm` | — | — | Suppress confirmation prompts |
| `--no-color` | — | — | Disable colored output |

---

## Commands

### `build` — Build a Package

Build a `.xp` package from an XBUILD or PKGBUILD recipe. Runs the full
pipeline: parse recipe → fetch sources → prepare → build → check → package →
strip → archive → sign.

```bash
xpkg build [OPTIONS]
```

| Flag | Short | Value | Description |
|------|-------|-------|-------------|
| `--file` | `-f` | `PATH` | Path to the XBUILD or PKGBUILD file (default: `./XBUILD`) |
| `--pkgbuild` | — | — | Parse the recipe as a PKGBUILD instead of XBUILD |
| `--builddir` | `-d` | `PATH` | Alternative build directory (overrides config) |
| `--outdir` | `-o` | `PATH` | Output directory for the built `.xp` package (overrides config) |
| `--no-check` | — | — | Skip the `check()` phase |
| `--sign` | — | — | Sign the package after building (requires `sign_key` in config) |

**Examples:**

```bash
xpkg build                             # Build from ./XBUILD
xpkg build -f path/to/XBUILD          # Build from a specific file
xpkg build --pkgbuild -f ./PKGBUILD   # Build from a PKGBUILD
xpkg build --no-check -o ./out        # Skip tests, output to ./out
xpkg build --sign                      # Build and sign the package
xpkg build -d /tmp/mybuild -o ./pkgs  # Custom build and output dirs
```

**Build pipeline steps:**

1. Parse and validate the recipe
2. Apply CLI overrides (builddir, outdir)
3. Run the build pipeline (prepare → build → check → package)
4. Strip ELF binaries (if `strip_binaries = true` in config)
5. Create `.xp` archive (tar.zst by default)
6. Sign the package (if `--sign` or `sign = true` in config)

---

### `lint` — Lint a Package Archive

Run quality checks on a built `.xp` package archive. Extracts the archive,
reads `.PKGINFO`, and runs all lint rules.

```bash
xpkg lint <PACKAGE> [OPTIONS]
```

| Argument | Description |
|----------|-------------|
| `PACKAGE` | Path to the `.xp` package archive to lint |

| Flag | Description |
|------|-------------|
| `--strict` | Treat lint warnings as errors (exit code 1) |

**Examples:**

```bash
xpkg lint hello-2.12-1-x86_64.xp          # Run lint checks
xpkg lint hello-2.12-1-x86_64.xp --strict # Fail on any warning
```

**Lint categories:**

- Permission checks — world-writable files, suid/sgid, ownership
- Path checks — files in non-standard locations (`/usr/local`, etc.)
- Metadata checks — `.PKGINFO` completeness and correctness
- Dependency checks — ELF dependencies vs declared depends
- ELF analysis — RPATH, TEXTREL, stack protector

See [Linting Rules](LINTING.md) for the complete list.

---

### `info` — Display Package Metadata

Display metadata from a built `.xp` package archive without installing it.

```bash
xpkg info <PACKAGE> [OPTIONS]
```

| Argument | Description |
|----------|-------------|
| `PACKAGE` | Path to the `.xp` package archive |

| Flag | Short | Description |
|------|-------|-------------|
| `--files` | `-l` | List all files contained in the package |
| `--json` | — | Output metadata as JSON (machine-readable) |

**Examples:**

```bash
xpkg info hello-2.12-1-x86_64.xp           # Human-readable metadata
xpkg info hello-2.12-1-x86_64.xp --files   # Include file listing
xpkg info hello-2.12-1-x86_64.xp --json    # JSON output for scripting
```

**Sample output:**

```
Name              : hello
Version           : 2.12-1
Description       : GNU Hello — the friendly greeter
Architecture      : x86_64
URL               : https://www.gnu.org/software/hello/
License           : GPL-3.0-or-later
Installed Size    : 48.2 KiB
Compressed Size   : 18.7 KiB
Build Date        : 2026-03-15 12:30:00 UTC
SHA-256           : a948904f2f0f479b...
Depends On        : glibc
Make Depends      : gcc  make
Optional Deps     : None
```

---

### `verify` — Verify Package Integrity

Verify the OpenPGP detached signature of a `.xp` package. Looks for a
`.xp.sig` file alongside the package.

```bash
xpkg verify <PACKAGE> [OPTIONS]
```

| Argument | Description |
|----------|-------------|
| `PACKAGE` | Path to the `.xp` package archive |

| Flag | Short | Value | Description |
|------|-------|-------|-------------|
| `--key` | `-k` | `PATH` | Path to a public key or keyring file |

**Examples:**

```bash
xpkg verify hello-2.12-1-x86_64.xp --key packager.pub
xpkg verify hello-2.12-1-x86_64.xp -k /etc/xpkg/trusted.gpg
```

See [Package Signing](SIGNING.md) for key generation and setup.

---

### `new` — Create an XBUILD Template

Generate a new XBUILD template directory for a package.

```bash
xpkg new <PKGNAME> [OPTIONS]
```

| Argument | Description |
|----------|-------------|
| `PKGNAME` | Name of the package to create a template for |

| Flag | Short | Value | Description |
|------|-------|-------|-------------|
| `--outdir` | `-o` | `PATH` | Output directory (default: `./<PKGNAME>/`) |

**Examples:**

```bash
xpkg new hello                  # Create hello/XBUILD
xpkg new mylib -o packages/     # Create packages/XBUILD
```

The generated XBUILD contains all sections with placeholder values and
comments explaining each field.

---

### `srcinfo` — Generate Source Info

Produce `.SRCINFO`-style output from a parsed XBUILD recipe. The recipe
is validated before generating the output.

```bash
xpkg srcinfo [OPTIONS]
```

| Flag | Short | Value | Description |
|------|-------|-------|-------------|
| `--file` | `-f` | `PATH` | Path to the XBUILD file (default: `./XBUILD`) |

**Examples:**

```bash
xpkg srcinfo                     # Generate from ./XBUILD
xpkg srcinfo -f path/to/XBUILD  # Generate from a specific file
xpkg srcinfo > .SRCINFO          # Write output to .SRCINFO
```

---

### `repo-add` — Add Package to Repository

Add a `.xp` package to a repository database. If the package already exists
in the database, the entry is updated.

```bash
xpkg repo-add <DB> <PACKAGE> [OPTIONS]
```

| Argument | Description |
|----------|-------------|
| `DB` | Path to the repository database file (e.g. `myrepo.db.tar.zst`) |
| `PACKAGE` | Path to the `.xp` package to add |

| Flag | Description |
|------|-------------|
| `--sign` | Sign the database after modification |

**Examples:**

```bash
xpkg repo-add myrepo.db.tar.zst hello-2.12-1-x86_64.xp
xpkg repo-add myrepo.db.tar.zst hello-2.12-1-x86_64.xp --sign
```

The database is created automatically if it does not exist. Supported formats:
`.db.tar.zst`, `.db.tar.gz`, `.db.tar.xz`.

See [Repository Management](REPOSITORY.md) for hosting instructions.

---

### `repo-remove` — Remove Package from Repository

Remove a package entry from a repository database by name.

```bash
xpkg repo-remove <DB> <PKGNAME> [OPTIONS]
```

| Argument | Description |
|----------|-------------|
| `DB` | Path to the repository database file |
| `PKGNAME` | Name of the package to remove |

| Flag | Description |
|------|-------------|
| `--sign` | Sign the database after modification |

**Examples:**

```bash
xpkg repo-remove myrepo.db.tar.zst hello
xpkg repo-remove myrepo.db.tar.zst hello --sign
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error (invalid recipe, build failure, lint errors, bad signature) |
| `2` | Invalid usage (missing arguments, unknown flags) |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Override tracing log level filter (e.g. `RUST_LOG=debug`) |

During builds, these variables are set in the build environment:

| Variable | Description |
|----------|-------------|
| `PKGDIR` | Destination directory for installed files |
| `SRCDIR` | Directory containing extracted source files |
| `BUILDDIR` | Top-level build directory |
| `MAKEFLAGS` | Make flags from config |
| `CFLAGS` | C compiler flags from config |
| `CXXFLAGS` | C++ compiler flags from config |
| `LDFLAGS` | Linker flags from config |

---

## Configuration

xpkg reads its configuration from `~/.config/xpkg/xpkg.conf` (or the
path given via `--config`). See [`etc/xpkg.conf.example`](../etc/xpkg.conf.example)
for all available options.

Key configuration sections:

- **`[options]`** — builddir, outdir, sign, sign_key, compress method/level, strip_binaries
- **`[environment]`** — MAKEFLAGS, CFLAGS, CXXFLAGS, LDFLAGS
- **`[lint]`** — enable/disable linting, strict mode

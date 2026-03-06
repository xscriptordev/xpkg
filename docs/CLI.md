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

Build a `.xp` package from an XBUILD or PKGBUILD recipe.

```bash
xpkg build [OPTIONS]
```

| Flag | Short | Value | Description |
|------|-------|-------|-------------|
| `--file` | `-f` | `PATH` | Path to the XBUILD or PKGBUILD file (default: `./XBUILD`) |
| `--pkgbuild` | — | — | Parse the recipe as a PKGBUILD instead of XBUILD |
| `--builddir` | `-d` | `PATH` | Alternative build directory |
| `--outdir` | `-o` | `PATH` | Output directory for the built `.xp` package |
| `--no-check` | — | — | Skip the `check()` phase |
| `--sign` | — | — | Sign the package after building |

**Examples:**

```bash
xpkg build                             # Build from ./XBUILD
xpkg build -f path/to/XBUILD          # Build from a specific file
xpkg build --pkgbuild -f ./PKGBUILD   # Build from a PKGBUILD
xpkg build --no-check -o ./out        # Skip tests, output to ./out
xpkg build --sign                      # Build and sign the package
```

> **Status:** not yet implemented — planned for Phase 4.

---

### `lint` — Lint a Package Archive

Run quality checks on a built `.xp` package archive.

```bash
xpkg lint <PACKAGE> [OPTIONS]
```

| Argument | Description |
|----------|-------------|
| `PACKAGE` | Path to the `.xp` package archive to lint |

| Flag | Description |
|------|-------------|
| `--strict` | Treat lint warnings as errors |

**Examples:**

```bash
xpkg lint hello-2.12-1-x86_64.xp          # Run lint checks
xpkg lint hello-2.12-1-x86_64.xp --strict # Fail on any warning
```

> **Status:** not yet implemented — planned for Phase 7.

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
comments explaining each field. Edit the template and fill in the real
values before building.

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

### `info` — Display Package Metadata

Display metadata from a built `.xp` package archive without installing it.

```bash
xpkg info <PACKAGE>
```

| Argument | Description |
|----------|-------------|
| `PACKAGE` | Path to the `.xp` package archive |

**Examples:**

```bash
xpkg info hello-2.12-1-x86_64.xp
```

> **Status:** not yet implemented — planned for Phase 9.

---

### `verify` — Verify Package Integrity

Verify the integrity and signature of a `.xp` package archive.

```bash
xpkg verify <PACKAGE>
```

| Argument | Description |
|----------|-------------|
| `PACKAGE` | Path to the `.xp` package archive |

**Examples:**

```bash
xpkg verify hello-2.12-1-x86_64.xp
```

> **Status:** not yet implemented — planned for Phase 9.

---

### `repo-add` — Add Package to Repository

Add a `.xp` package to a repository database.

```bash
xpkg repo-add <DB> <PACKAGE> [OPTIONS]
```

| Argument | Description |
|----------|-------------|
| `DB` | Path to the repository database file |
| `PACKAGE` | Path to the `.xp` package to add |

| Flag | Description |
|------|-------------|
| `--sign` | Sign the database after modification |

**Examples:**

```bash
xpkg repo-add myrepo.db.tar.zst hello-2.12-1-x86_64.xp
xpkg repo-add myrepo.db.tar.zst hello-2.12-1-x86_64.xp --sign
```

> **Status:** not yet implemented — planned for Phase 8.

---

### `repo-remove` — Remove Package from Repository

Remove a package entry from a repository database.

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

> **Status:** not yet implemented — planned for Phase 8.

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error (invalid recipe, build failure, etc.) |
| `2` | Invalid usage (missing arguments, unknown flags) |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Override tracing log level filter (e.g. `RUST_LOG=debug`) |

---

## Configuration

xpkg reads its configuration from `~/.config/xpkg/xpkg.conf` (or the
path given via `--config`). See `etc/xpkg.conf.example` for all
available options.

Key configuration sections:

- **`[options]`** — builddir, outdir, sign, compress method/level, strip
- **`[environment]`** — MAKEFLAGS, CFLAGS, CXXFLAGS, LDFLAGS
- **`[lint]`** — enable/disable linting, strict mode

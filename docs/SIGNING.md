# Package Signing

How to generate keys, sign packages, and verify signatures with xpkg.

---

## Overview

xpkg uses **OpenPGP detached signatures** for package and repository
authentication. The cryptography is provided by
[sequoia-openpgp](https://sequoia-pgp.org/) — a pure Rust OpenPGP
implementation. No GPG daemon or external tools are required.

When you sign a package, xpkg creates a `.sig` file alongside the `.xp`
archive. When you verify, it reads the `.sig` and checks it against the
original file and a public key you trust.

---

## Generate a Key Pair

xpkg uses standard OpenPGP keys. You can generate them with GPG or any
OpenPGP-compatible tool:

```bash
# Using GPG (recommended for first-time setup)
gpg --full-generate-key
# Choose: RSA and RSA, 4096 bits, no expiration (or your preference)
# Enter your name and email

# Export the secret key (for signing)
gpg --export-secret-keys --armor your@email.com > ~/.config/xpkg/signing.key

# Export the public key (for verification / distribution)
gpg --export --armor your@email.com > ~/.config/xpkg/signing.pub
```

Alternatively, any OpenPGP key in binary or ASCII-armored format works.

---

## Configure Signing

Edit `~/.config/xpkg/xpkg.conf`:

```toml
[options]
sign = true
sign_key = "/home/youruser/.config/xpkg/signing.key"
```

| Setting | Description |
|---------|-------------|
| `sign` | `true` to sign all packages by default |
| `sign_key` | Absolute path to the secret key file |

---

## Sign Packages

### Automatic (during build)

```bash
# If sign = true in config
xpkg build

# Or override with --sign flag
xpkg build --sign
```

Output:

```
==> Built hello-2.12-1 in 4.2s
    Stripped 1 ELF binaries
==> Package: hello-2.12-1-x86_64.xp
    Size: 18.7 KiB
    Signature: hello-2.12-1-x86_64.xp.sig (287 bytes, key A1B2C3D4)
```

### Sign repository databases

```bash
xpkg repo-add myrepo.db.tar.zst hello-2.12-1-x86_64.xp --sign
```

This signs the database file as well, producing `myrepo.db.tar.zst.sig`.

---

## Verify Signatures

```bash
# Verify a package with a specific public key
xpkg verify hello-2.12-1-x86_64.xp --key signing.pub

# Verify with a keyring (multiple keys)
xpkg verify hello-2.12-1-x86_64.xp --key trusted-keyring.gpg
```

**Outcomes:**

| Result | Meaning |
|--------|---------|
| `✓ Valid signature (key A1B2C3D4)` | Signature is good, key is trusted |
| `signature made by an unknown key` | Signature is valid but the key is not in your keyring |
| `bad signature: ...` | File has been tampered with or signature is corrupt |

---

## Key Management

### Keyring files

A keyring is a single file containing multiple public keys concatenated
together. xpkg automatically detects whether a `--key` argument is a
single certificate or a keyring.

```bash
# Create a keyring with multiple trusted packagers
cat alice.pub bob.pub charlie.pub > trusted.gpg
xpkg verify package.xp --key trusted.gpg
```

### Key ID matching

xpkg matches keys by their fingerprint or short key ID. The signature
output shows which key was used, so you can verify it matches the
expected packager.

---

## Security Notes

- **Keep secret keys safe.** The `sign_key` file should be readable only
  by your user (`chmod 600`).
- **Distribute public keys** through a trusted channel (your repository,
  HTTPS website, etc.).
- **Always verify** packages from untrusted sources before installing.
- xpkg uses sequoia-openpgp with the `crypto-rust` backend — no
  dependency on OpenSSL or libgcrypt.

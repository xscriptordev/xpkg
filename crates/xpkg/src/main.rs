//! xpkg — Package builder for X Distribution
//!
//! Entry point for the xpkg binary. Handles CLI parsing, configuration loading,
//! logging initialization, and dispatching to the appropriate subcommand handler.

mod cli;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::Level;
use tracing_subscriber::EnvFilter;

use cli::{Cli, Command};
use xpkg_core::recipe;
use xpkg_core::XpkgConfig;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // ── Initialize logging ──────────────────────────────────────────────
    let log_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(log_level.into())
                .from_env_lossy(),
        )
        .with_target(false)
        .init();

    tracing::debug!("xpkg v{}", env!("CARGO_PKG_VERSION"));

    // ── Load configuration ──────────────────────────────────────────────
    let config_path = cli.config.clone().unwrap_or_else(XpkgConfig::default_path);

    let config = XpkgConfig::load_or_default(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path.display()))?;

    tracing::info!(
        builddir = %config.options.builddir.display(),
        outdir = %config.options.outdir.display(),
        "configuration loaded"
    );

    // ── Dispatch subcommands ────────────────────────────────────────────
    match &cli.command {
        Command::Build(args) => cmd_build(&config, args),
        Command::Lint(args) => cmd_lint(&config, args),
        Command::New(args) => cmd_new(args),
        Command::Srcinfo(args) => cmd_srcinfo(args),
        Command::Info(args) => cmd_info(args),
        Command::Verify(args) => cmd_verify(args),
        Command::RepoAdd(args) => cmd_repo_add(&config, args),
        Command::RepoRemove(args) => cmd_repo_remove(args),
    }
}

// ── Subcommand stubs ────────────────────────────────────────────────────────
//
// Each function below is a placeholder that will be filled with real logic
// as the corresponding roadmap phase is implemented.

fn cmd_build(config: &XpkgConfig, args: &cli::BuildArgs) -> Result<()> {
    use xpkg_core::archive::{create_package, strip_binaries};
    use xpkg_core::builder::{build_package, BuildOptions};

    // ── Resolve recipe path ─────────────────────────────────────────
    let recipe_path = args
        .file
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("XBUILD"));

    let recipe_dir = recipe_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    let recipe_dir = if recipe_dir.as_os_str().is_empty() {
        std::path::PathBuf::from(".")
    } else {
        recipe_dir
    };

    // ── Parse recipe ────────────────────────────────────────────────
    let raw_recipe = if args.pkgbuild {
        recipe::parse_pkgbuild(&recipe_path)
            .with_context(|| format!("failed to parse PKGBUILD {}", recipe_path.display()))?
    } else {
        recipe::parse_xbuild(&recipe_path)
            .with_context(|| format!("failed to parse {}", recipe_path.display()))?
    };

    recipe::validate_recipe(&raw_recipe)
        .with_context(|| format!("recipe validation failed for {}", recipe_path.display()))?;

    tracing::info!(
        name = %raw_recipe.package.name,
        version = %raw_recipe.package.version,
        "recipe loaded"
    );

    // ── Apply CLI overrides to config ───────────────────────────────
    let mut build_config = config.clone();
    if let Some(ref builddir) = args.builddir {
        build_config.options.builddir = builddir.clone();
    }
    if let Some(ref outdir) = args.outdir {
        build_config.options.outdir = outdir.clone();
    }

    // ── Build options ───────────────────────────────────────────────
    let options = BuildOptions {
        skip_check: args.no_check,
        keep_builddir: false,
    };

    // ── Run build pipeline ──────────────────────────────────────────
    let result = build_package(&build_config, &raw_recipe, &recipe_dir, None, &options)?;

    println!(
        "==> Built {}-{}-{} in {:.1}s",
        result.pkgname,
        result.pkgver,
        result.pkgrel,
        result.duration.as_secs_f64()
    );

    // ── Strip binaries (optional) ───────────────────────────────────
    if build_config.options.strip_binaries {
        let stripped =
            strip_binaries(&result.pkgdir).with_context(|| "failed to strip binaries")?;
        if stripped > 0 {
            println!("    Stripped {stripped} ELF binaries");
        }
    }

    // ── Create .xp archive ──────────────────────────────────────────
    let outdir = &build_config.options.outdir;
    let pkg = create_package(&build_config, &raw_recipe, &result.pkgdir, outdir)
        .with_context(|| "failed to create package archive")?;

    println!("==> Package: {}", pkg.archive_path.display());
    println!("    Size: {:.1} KiB", pkg.archive_size as f64 / 1024.0);

    // ── Sign package (optional) ─────────────────────────────────────
    if args.sign || build_config.options.sign {
        use xpkg_core::signing::{load_secret_key, sign_file};

        let key_path = std::path::PathBuf::from(&build_config.options.sign_key);
        if key_path.as_os_str().is_empty() {
            anyhow::bail!("--sign requires a signing key; set sign_key in xpkg.conf");
        }

        let secret_key =
            load_secret_key(&key_path).with_context(|| "failed to load signing key")?;
        let sig = sign_file(&pkg.archive_path, &secret_key, false)
            .with_context(|| "failed to sign package")?;

        println!(
            "    Signature: {} ({} bytes, key {})",
            sig.sig_path.display(),
            sig.sig_size,
            sig.key_id
        );
    }

    Ok(())
}

fn cmd_lint(_config: &XpkgConfig, args: &cli::LintArgs) -> Result<()> {
    use xpkg_core::lint::{format_report, lint_package, ReportFormat};

    let archive_path = &args.package;
    tracing::info!(path = %archive_path.display(), "linting package");

    // ── Extract the archive to a temp directory ─────────────────────
    let tmp = tempfile::tempdir().with_context(|| "failed to create temp directory")?;
    let extract_dir = tmp.path();

    let file = std::fs::File::open(archive_path)
        .with_context(|| format!("failed to open {}", archive_path.display()))?;

    let decoder = zstd::Decoder::new(file)
        .with_context(|| format!("failed to decompress {}", archive_path.display()))?;
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(extract_dir)
        .with_context(|| format!("failed to extract {}", archive_path.display()))?;

    // ── Read .PKGINFO if present ────────────────────────────────────
    let pkginfo_path = extract_dir.join(".PKGINFO");
    let pkginfo_content = if pkginfo_path.exists() {
        Some(std::fs::read_to_string(&pkginfo_path).with_context(|| "failed to read .PKGINFO")?)
    } else {
        None
    };

    // ── Run lint checks ─────────────────────────────────────────────
    println!("==> Linting {}", archive_path.display());

    let result = lint_package(extract_dir, pkginfo_content.as_deref(), args.strict)
        .with_context(|| "lint checks failed")?;

    let report = format_report(&result, ReportFormat::Human);
    print!("{report}");

    if result.has_errors() {
        anyhow::bail!(
            "lint failed with {} error(s)",
            result.count(xpkg_core::lint::Severity::Error)
        );
    }

    if !result.has_warnings() && result.total() == 0 {
        println!("==> Package passed all lint checks");
    }

    Ok(())
}

fn cmd_new(args: &cli::NewArgs) -> Result<()> {
    let template = recipe::generate_template(&args.pkgname);

    let outdir = args
        .outdir
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from(&args.pkgname));

    std::fs::create_dir_all(&outdir)
        .with_context(|| format!("failed to create directory {}", outdir.display()))?;

    let xbuild_path = outdir.join("XBUILD");
    std::fs::write(&xbuild_path, &template)
        .with_context(|| format!("failed to write {}", xbuild_path.display()))?;

    println!("Created {}", xbuild_path.display());
    tracing::info!(path = %xbuild_path.display(), "generated XBUILD template");
    Ok(())
}

fn cmd_srcinfo(args: &cli::SrcinfoArgs) -> Result<()> {
    let path = args
        .file
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("XBUILD"));

    let raw_recipe = recipe::parse_xbuild(&path)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    recipe::validate_recipe(&raw_recipe)
        .with_context(|| format!("validation failed for {}", path.display()))?;

    let srcinfo = recipe::generate_srcinfo(&raw_recipe);
    print!("{srcinfo}");
    Ok(())
}

fn cmd_info(_args: &cli::InfoArgs) -> Result<()> {
    tracing::info!("info: not yet implemented");
    println!("xpkg info — not yet implemented");
    Ok(())
}

fn cmd_verify(args: &cli::VerifyArgs) -> Result<()> {
    use xpkg_core::signing::{load_cert, load_keyring, verify_file, VerifyOutcome};

    let package_path = &args.package;
    let sig_path = package_path.with_extension(format!(
        "{}.sig",
        package_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
    ));

    if !sig_path.exists() {
        anyhow::bail!("signature file not found: {}", sig_path.display());
    }

    let certs = match &args.key {
        Some(key_path) => {
            // Try as keyring first, fall back to single cert.
            load_keyring(key_path).or_else(|_| load_cert(key_path).map(|c| vec![c]))?
        }
        None => {
            anyhow::bail!("no public key specified; use --key <path> to provide one");
        }
    };

    println!("==> Verifying {}", package_path.display());

    let outcome = verify_file(package_path, &sig_path, &certs)
        .with_context(|| "signature verification failed")?;

    match outcome {
        VerifyOutcome::Good { key_id } => {
            println!("    ✓ Valid signature (key {key_id})");
        }
        VerifyOutcome::UnknownKey => {
            anyhow::bail!("signature made by an unknown key");
        }
        VerifyOutcome::Bad { reason } => {
            anyhow::bail!("bad signature: {reason}");
        }
    }

    Ok(())
}

fn cmd_repo_add(config: &XpkgConfig, args: &cli::RepoAddArgs) -> Result<()> {
    use xpkg_core::repo::{add_entry, entry_from_package, read_db, write_db};

    let db_path = &args.db;
    let package_path = &args.package;

    tracing::info!(
        db = %db_path.display(),
        package = %package_path.display(),
        "adding package to repository"
    );

    // Derive repo name from the db path (e.g. "myrepo.db.tar.zst" → "myrepo").
    let repo_name = db_path
        .file_name()
        .and_then(|f| f.to_str())
        .and_then(|f| f.split('.').next())
        .unwrap_or("repo");

    let mut db =
        read_db(db_path, repo_name).with_context(|| "failed to read repository database")?;

    let entry = entry_from_package(package_path)
        .with_context(|| format!("failed to inspect {}", package_path.display()))?;

    let pkg_display = format!("{}-{}", entry.name, entry.full_version());

    add_entry(&mut db, entry);
    write_db(&db).with_context(|| "failed to write repository database")?;

    println!("==> Added {pkg_display} to {}", db_path.display());
    println!("    Repository now contains {} package(s)", db.len());

    // ── Sign database (optional) ────────────────────────────────────
    if args.sign {
        use xpkg_core::signing::{load_secret_key, sign_file};

        let key_path = std::path::PathBuf::from(&config.options.sign_key);
        if key_path.as_os_str().is_empty() {
            anyhow::bail!("--sign requires a signing key; set sign_key in xpkg.conf");
        }

        let secret_key =
            load_secret_key(&key_path).with_context(|| "failed to load signing key")?;
        let sig =
            sign_file(db_path, &secret_key, false).with_context(|| "failed to sign database")?;

        println!(
            "    Database signed: {} (key {})",
            sig.sig_path.display(),
            sig.key_id
        );
    }

    Ok(())
}

fn cmd_repo_remove(args: &cli::RepoRemoveArgs) -> Result<()> {
    use xpkg_core::repo::{read_db, remove_entry, write_db};

    let db_path = &args.db;
    let pkgname = &args.pkgname;

    tracing::info!(db = %db_path.display(), package = %pkgname, "removing package from repository");

    let repo_name = db_path
        .file_name()
        .and_then(|f| f.to_str())
        .and_then(|f| f.split('.').next())
        .unwrap_or("repo");

    let mut db =
        read_db(db_path, repo_name).with_context(|| "failed to read repository database")?;

    match remove_entry(&mut db, pkgname) {
        Some(_) => {
            write_db(&db).with_context(|| "failed to write repository database")?;
            println!("==> Removed {pkgname} from {}", db_path.display());
            println!("    Repository now contains {} package(s)", db.len());
        }
        None => {
            anyhow::bail!("package '{pkgname}' not found in {}", db_path.display());
        }
    }

    Ok(())
}

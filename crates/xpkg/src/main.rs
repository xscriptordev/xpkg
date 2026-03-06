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
        Command::RepoAdd(args) => cmd_repo_add(args),
        Command::RepoRemove(args) => cmd_repo_remove(args),
    }
}

// ── Subcommand stubs ────────────────────────────────────────────────────────
//
// Each function below is a placeholder that will be filled with real logic
// as the corresponding roadmap phase is implemented.

fn cmd_build(_config: &XpkgConfig, _args: &cli::BuildArgs) -> Result<()> {
    tracing::info!("build: not yet implemented");
    println!("xpkg build — not yet implemented");
    Ok(())
}

fn cmd_lint(_config: &XpkgConfig, _args: &cli::LintArgs) -> Result<()> {
    tracing::info!("lint: not yet implemented");
    println!("xpkg lint — not yet implemented");
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

fn cmd_verify(_args: &cli::VerifyArgs) -> Result<()> {
    tracing::info!("verify: not yet implemented");
    println!("xpkg verify — not yet implemented");
    Ok(())
}

fn cmd_repo_add(_args: &cli::RepoAddArgs) -> Result<()> {
    tracing::info!("repo-add: not yet implemented");
    println!("xpkg repo-add — not yet implemented");
    Ok(())
}

fn cmd_repo_remove(_args: &cli::RepoRemoveArgs) -> Result<()> {
    tracing::info!("repo-remove: not yet implemented");
    println!("xpkg repo-remove — not yet implemented");
    Ok(())
}

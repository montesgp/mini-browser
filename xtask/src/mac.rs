// Derived from the cef-rs `bundle-cef-app` tool (https://github.com/tauri-apps/cef-rs),
// licensed Apache-2.0 OR MIT.

use std::{env, fs, path::Path, path::PathBuf, process::Command};

use anyhow::{Context, bail};
use cef::build_util::{
    mac::{BundleInfo, build_bundle, bundle},
    metadata::get_cargo_metadata,
};
use clap::Parser;
use semver::Version;

/// The app icon; see `assets/icon/icon.svg`.
const ICON: &[u8] = include_bytes!("../../assets/icon/icon.icns");

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Package and executable name of the app.
    name: String,
    /// Build optimized binaries (cef-rs only bundles debug builds on macOS).
    #[arg(long, default_value_t = false)]
    release: bool,
    /// Directory that receives the `.app` bundle; defaults to the current directory.
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(short, long, default_value = "dev.minibrowser")]
    identifier: String,
    #[arg(short, long, default_value = "Mini Browser")]
    display_name: String,
    #[arg(short, long, default_value = "English")]
    region: String,
    #[arg(short, long, default_value = env!("CARGO_PKG_VERSION"))]
    version: String,
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let output = match args.output {
        Some(output) => output,
        None => env::current_dir()?,
    };

    let bundle_info = BundleInfo {
        name: args.name.clone(),
        identifier: args.identifier,
        display_name: args.display_name,
        development_region: args.region,
        version: Version::parse(&args.version)?,
    };

    let bundle_path = if args.release {
        build_release_bundle(&output, &args.name, bundle_info)?
    } else {
        build_bundle(&output, &args.name, bundle_info)?
    };
    set_icon(&bundle_path)?;
    println!("Run the app from {}", bundle_path.display());
    Ok(())
}

/// Release counterpart of cef-rs `build_bundle`: builds the app and its helper with
/// `--release`, then lays out the `.app` from `target/release`.
fn build_release_bundle(
    output: &std::path::Path,
    name: &str,
    bundle_info: BundleInfo,
) -> anyhow::Result<PathBuf> {
    let metadata = get_cargo_metadata()?;
    let bundle_metadata = metadata.parse_bundle_metadata(name)?;
    for bin in [name, bundle_metadata.helper_name.as_str()] {
        cargo_build_release(bin)?;
    }
    let target_path = metadata.target_directory().join("release");
    Ok(bundle(
        output,
        &target_path,
        name,
        &bundle_metadata.helper_name,
        bundle_metadata.resources_path,
        bundle_info,
    )?)
}

/// Copies the app icon into the `.app` and names it in `Info.plist` (`CFBundleIconFile`).
fn set_icon(app: &Path) -> anyhow::Result<()> {
    let contents = app.join("Contents");
    fs::write(contents.join("Resources").join("icon.icns"), ICON)?;
    let status = Command::new("plutil")
        .args(["-replace", "CFBundleIconFile", "-string", "icon.icns"])
        .arg(contents.join("Info.plist"))
        .status()
        .context("failed to run plutil")?;
    if !status.success() {
        bail!("plutil could not set CFBundleIconFile: {status}");
    }
    Ok(())
}

fn cargo_build_release(bin: &str) -> anyhow::Result<()> {
    println!("Building {bin} (release)...");
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .args(["build", "--release", "--bin", bin])
        .status()
        .with_context(|| format!("failed to run cargo build for {bin}"))?;
    if !status.success() {
        bail!("cargo build --release --bin {bin} failed with {status}");
    }
    Ok(())
}

// Derived from the cef-rs `bundle-cef-app` tool (https://github.com/tauri-apps/cef-rs),
// licensed Apache-2.0 OR MIT.

use std::{env, path::PathBuf};

use cef::build_util::mac::{BundleInfo, build_bundle};
use clap::Parser;
use semver::Version;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Package and executable name of the app.
    name: String,
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

    let bundle_path = build_bundle(&output, &args.name, bundle_info)?;
    println!("Run the app from {}", bundle_path.display());
    Ok(())
}

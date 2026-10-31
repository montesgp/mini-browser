// Derived from the cef-rs `bundle-cef-app` tool (https://github.com/tauri-apps/cef-rs),
// licensed Apache-2.0 OR MIT.

use std::{env, path::PathBuf};

use cef::build_util::linux::build_bundle;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Package and executable name of the app.
    name: String,
    #[arg(long, default_value_t = false)]
    release: bool,
    /// Bundle directory; defaults to the current directory.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let output = match args.output {
        Some(output) => output,
        None => env::current_dir()?,
    };

    let bundle_path = build_bundle(&output, &args.name, args.release)?;
    println!("Run the app from {}", bundle_path.display());
    Ok(())
}

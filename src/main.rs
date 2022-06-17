use anyhow::Result;
use clap::Parser;

mod gui;
mod installer;

const ICON: &'static [u8] = include_bytes!("../quilt.png");

/// An installer for quilt-loader
#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Start the installer in no-gui mode
    #[clap(long)]
    no_gui: bool
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.no_gui {
        println!("No gui mode")
    } else {
        gui::run()?;
    }

    Ok(())
}


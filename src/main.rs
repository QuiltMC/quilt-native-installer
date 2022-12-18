#![windows_subsystem = "windows"]

use anyhow::Result;
use clap::Parser;

mod gui;
mod installer;

const ICON: &[u8] = include_bytes!("../quilt.png");

#[derive(Default, Parser)]
#[clap(about, version)]
pub struct Args {
    /// Start the installer without a gui
    #[clap(long)]
    no_gui: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.no_gui {
        println!("No gui mode")
    } else {
        gui::run(args)?;
    }

    Ok(())
}

#![windows_subsystem = "windows"]

use clap::Parser;

mod cli;
mod gui;
mod installer;

const ICON: &[u8] = include_bytes!("../quilt.png");

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    if let Some(subcommand) = args.subcommand {
        tokio::runtime::Runtime::new().unwrap().block_on(async {cli::cli(subcommand).await.expect("Installation failed! Exiting.")});
    } else {
        println!("quilt-installer can also be used as a cli! Run with --help for more information.");
        gui::run()?
    }

    Ok(())
}

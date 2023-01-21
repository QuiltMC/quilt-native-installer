#![windows_subsystem = "windows"]

use clap::Parser;

mod cli;
mod gui;
mod installer;

const ICON: &[u8] = include_bytes!("../quilt.png");

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    if let Some(subcommand) = args.subcommand {
        match subcommand {
            cli::SubCommands::Client => println!("Installing client..."),
            cli::SubCommands::Server => println!("Installing server..."),
        }
        println!("The CLI hasn't been implemented yet!");
    } else {
        gui::run()?;
    }

    Ok(())
}

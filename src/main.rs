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
        std::thread::sleep(std::time::Duration::from_secs(2));
        println!("Just kidding, we haven't implemented the CLI yet :)");
    } else {
        gui::run()?;
    }

    Ok(())
}

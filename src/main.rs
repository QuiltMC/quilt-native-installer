#![windows_subsystem = "windows"]

use anyhow::Context;
use clap::Parser;

mod cli;
mod gui;
mod installer;

const ICON: &[u8] = include_bytes!("../quilt.png");

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            '/',
            env!("CARGO_PKG_VERSION"),
        ))
        .build()
        .unwrap();

    if args.subcommand.is_some() {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(cli::cli(client, args))
            .context("Installation failed!")
    } else {
        println!("quilt-installer can also be used as a CLI! Run with --help for more information");
        gui::run(client)
    }
}

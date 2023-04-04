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
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .build()
        .unwrap();

    if let Some(subcommand) = args.subcommand {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                cli::cli(client, subcommand)
                    .await
                    .context("Installation failed!")
            })
    } else {
        println!(
            "quilt-installer can also be used as a CLI! Run with --help for more information."
        );
        gui::run(client)
    }
}

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(about, version)]
#[clap(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Option<SubCommands>,
}

#[derive(Subcommand)]
pub enum SubCommands {
    Client,
    Server,
}

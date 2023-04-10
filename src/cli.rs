use crate::installer::{
    self, ClientInstallation, LoaderVersion, MinecraftVersion, ServerInstallation,
};
use anyhow::Context;
use anyhow::Result;
use clap::{Parser, Subcommand};
use derive_more::Display;
use reqwest::Client;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about, version, propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Option<Subcommands>,
    /// The Minecraft version to install
    ///
    /// Pick between the
    /// latest `stable` version,
    /// latest `snapshot`,
    /// or a specific version number.
    #[arg(short = 'm', long, default_value_t)]
    minecraft_version: MCVersionCLI,
    /// The Quilt loader version to install
    ///
    /// Pick between the
    /// latest `stable` version,
    /// latest `beta`,
    /// or a specific version number.
    #[arg(short = 'l', long, default_value_t)]
    loader_version: LoaderVersionCLI,
}

#[derive(Subcommand)]
pub enum Subcommands {
    /// Install the Quilt Loader client
    Client {
        /// Don't create a profile
        #[arg(short = 'p', long)]
        no_profile: bool,
        /// The directory to install to
        #[arg(
            short = 'o',
            long,
            default_value_os_t = installer::get_default_client_directory()
        )]
        install_dir: PathBuf,
    },
    /// Install the Quilt standalone server
    Server {
        /// Do not generate launch scripts
        #[arg(short = 's', long)]
        no_script: bool,
        /// Do not download the server jar
        #[arg(short = 'j', long)]
        no_jar: bool,
        /// The directory to install to
        #[arg(short = 'o', long)]
        install_dir: PathBuf,
    },
}
#[derive(Clone, PartialEq, Eq, Default, Display)]
pub enum MCVersionCLI {
    #[default]
    Stable,
    Snapshot,
    Custom(String),
}

#[derive(Clone, PartialEq, Eq, Default, Display)]
pub enum LoaderVersionCLI {
    #[default]
    Stable,
    Beta,
    Custom(String),
}

impl From<String> for MCVersionCLI {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_ref() {
            "stable" => Self::Stable,
            "snapshot" => Self::Snapshot,
            _ => Self::Custom(s),
        }
    }
}

impl From<String> for LoaderVersionCLI {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_ref() {
            "stable" => Self::Stable,
            "beta" => Self::Beta,
            _ => Self::Custom(s),
        }
    }
}

pub async fn cli(client: Client, args: Args) -> Result<()> {
    let (minecraft_version, loader_version) =
        get_versions(client.clone(), args.minecraft_version, args.loader_version).await?;

    match args.subcommand.unwrap() {
        Subcommands::Client {
            no_profile,
            install_dir,
        } => {
            installer::install_client(
                client,
                ClientInstallation {
                    minecraft_version,
                    loader_version,
                    install_dir,
                    generate_profile: !no_profile,
                },
            )
            .await
        }
        Subcommands::Server {
            no_script,
            no_jar,
            install_dir,
        } => {
            installer::install_server(
                client,
                ServerInstallation {
                    minecraft_version,
                    loader_version,
                    install_dir,
                    download_jar: !no_jar,
                    generate_script: !no_script,
                },
            )
            .await
        }
    }
}

async fn get_versions(
    client: Client,
    minecraft_version: MCVersionCLI,
    loader_version: LoaderVersionCLI,
) -> Result<(MinecraftVersion, LoaderVersion)> {
    let minecraft_versions = installer::fetch_minecraft_versions(client.clone()).await?;
    let loader_versions = installer::fetch_loader_versions(client).await?;

    Ok((
        match minecraft_version {
            MCVersionCLI::Stable => minecraft_versions.into_iter().find(|v| v.stable).unwrap(),
            MCVersionCLI::Snapshot => minecraft_versions.into_iter().find(|v| !v.stable).unwrap(),
            MCVersionCLI::Custom(input) => minecraft_versions
                .into_iter()
                .find(|v| v.version == input)
                .context(format!("Could not find Minecraft version {}", input))?,
        },
        match loader_version {
            LoaderVersionCLI::Stable => loader_versions
                .into_iter()
                .find(|v| v.version.pre.is_empty())
                .unwrap(),
            LoaderVersionCLI::Beta => loader_versions
                .into_iter()
                .find(|v| !v.version.pre.is_empty())
                .unwrap(),
            LoaderVersionCLI::Custom(input) => loader_versions
                .into_iter()
                .find(|v| v.to_string() == input)
                .context(format!("Could not find Quilt Loader version {}", input))?,
        },
    ))
}

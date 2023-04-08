use crate::installer::{
    self, ClientInstallation, LoaderVersion, MinecraftVersion, ServerInstallation,
};
use anyhow::Context;
use clap::{Parser, Subcommand};
use reqwest::Client;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(about, version)]
#[clap(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Option<Subcommands>,
    /// The Minecraft version to install
    ///
    /// Pick between the
    /// latest `stable` version (default),
    /// latest `snapshot`,
    /// or a specific version number.
    #[arg(short = 'm', long)]
    minecraft_version: Option<String>,
    /// The Quilt loader version to install
    ///
    /// Pick between the
    /// latest `stable` version (default),
    /// latest `beta`,
    /// or a specific version number.
    #[arg(short = 'l', long)]
    loader_version: Option<String>,
}

#[derive(Subcommand)]
pub enum Subcommands {
    /// Install the Quilt Loader client
    Client {
        /// Don't create a profile
        #[arg(short = 'p', long)]
        no_profile: bool,
        /// The directory to install to
        #[arg(short = 'o', long)]
        install_dir: Option<PathBuf>,
    },
    Server {
        /// Create launch scripts
        #[arg(short = 's', long, default_value_t = true)]
        generate_script: bool,
        /// Download the server jar
        #[arg(short, long, default_value_t = true)]
        download_jar: bool,
        /// The directory to install to
        #[arg(short = 'o', long)]
        install_dir: PathBuf,
    },
}

#[derive(Clone, PartialEq, Eq, Default)]
pub enum MCVersionCLI {
    #[default]
    Stable,
    Snapshot,
    Custom(String),
}

#[derive(Clone, PartialEq, Eq, Default)]
pub enum LoaderVersionCLI {
    #[default]
    Stable,
    Beta,
    Custom(String),
}

impl From<Option<String>> for MCVersionCLI {
    fn from(s: Option<String>) -> Self {
        if let Some(s) = s {
            match s.to_lowercase().as_ref() {
                "stable" => Self::Stable,
                "snapshot" => Self::Snapshot,
                _ => Self::Custom(s),
            }
        } else {
            Self::default()
        }
    }
}

impl From<Option<String>> for LoaderVersionCLI {
    fn from(s: Option<String>) -> Self {
        if let Some(s) = s {
            match s.to_lowercase().as_ref() {
                "stable" => Self::Stable,
                "beta" => Self::Beta,
                _ => Self::Custom(s),
            }
        } else {
            Self::default()
        }
    }
}

pub async fn cli(client: Client, args: Args) -> anyhow::Result<()> {
    let (minecraft_version, loader_version) = get_versions(
        client.clone(),
        args.minecraft_version.into(),
        args.loader_version.into(),
    )
    .await?;

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
                    install_dir: install_dir
                        .unwrap_or_else(installer::get_default_client_directory),
                    generate_profile: !no_profile,
                },
            )
            .await
        }
        Subcommands::Server {
            generate_script,
            download_jar,
            install_dir,
        } => {
            installer::install_server(ServerInstallation {
                minecraft_version,
                loader_version,
                install_dir,
                download_jar,
                generate_script,
            })
            .await
        }
    }
}

async fn get_versions(
    client: Client,
    minecraft_version: MCVersionCLI,
    loader_version: LoaderVersionCLI,
) -> anyhow::Result<(MinecraftVersion, LoaderVersion)> {
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
                .find(|v| v.version.to_string() == input)
                .context(format!("Could not find Quilt Loader version {}", input))?,
        },
    ))
}

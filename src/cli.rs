use std::path::PathBuf;
use anyhow::anyhow;
use clap::{Parser, Subcommand};
use reqwest::Client;
use crate::installer;
use crate::installer::{ClientInstallation, LoaderVersion, MinecraftVersion};

#[derive(Parser)]
#[clap(about, version)]
#[clap(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Option<SubCommands>,
}

#[derive(Subcommand)]
pub enum SubCommands {
    Install {
        #[command(subcommand)]
        subcommand: InstallSubcommands
    }
}

#[derive(Subcommand)]
pub enum InstallSubcommands {
    Client {
        #[arg(short='M',long)]
        minecraft_version: Option<String>,

        #[arg(short='L',long)]
        loader_version: Option<String>,
        #[arg(short='s',long)]
        snapshot: bool,
        #[arg(short='b',long)]
        loader_beta: bool,

        #[arg(short, long)]
        no_profile: bool,
        #[arg(short, long, value_name = "'.minecraft' DIRECTORY")]
        install_dir: Option<PathBuf>
    },
    Server {
        #[arg(short='M',long)]
        minecraft_version: Option<String>,
        #[arg(short='L',long)]
        loader_version: Option<String>,

        #[arg(short, long)]
        create_scripts: bool,
        #[arg(short, long)]
        download_server: bool,
        #[arg(short, long)]
        install_dir: PathBuf
    }
}


pub async fn cli(client: Client, args: SubCommands) -> anyhow::Result<()> {
    match args {
            SubCommands::Install { subcommand} => {
                match subcommand {
                    InstallSubcommands::Client { minecraft_version, loader_version, snapshot, loader_beta, no_profile, install_dir } => {
                        let (mc_version_to_install, loader_version_to_install) =  get_versions(client.clone(), minecraft_version, loader_version, snapshot, loader_beta).await?;
                        let install_dir = install_dir.unwrap_or_else(installer::get_default_client_directory);

                            installer::install_client(client, ClientInstallation {
                                minecraft_version: mc_version_to_install,
                                loader_version: loader_version_to_install,
                                install_dir,
                                generate_profile: !no_profile}).await?;

                    }
                    InstallSubcommands::Server { .. } => {}
                }
            }
        }

    Ok(())
}

async fn get_versions(client: Client, minecraft_version: Option<String>, loader_version: Option<String>, snapshot: bool, loader_beta: bool) -> anyhow::Result<(MinecraftVersion, LoaderVersion)> {
    let mc_versions = installer::fetch_minecraft_versions(client.clone());
    let loader_versions = installer::fetch_loader_versions(client);

    let mc_version_to_install: MinecraftVersion;
    let loader_version_to_install: LoaderVersion;

    if let Some(minecraft_version) = minecraft_version {
        if let Some(found) = mc_versions.await?.iter().find(|v| v.version.eq(&minecraft_version)).cloned() {
            mc_version_to_install = found;
        } else {
            return Err(anyhow!("Could not find Minecraft version {}", minecraft_version))
        }
    } else {
        mc_version_to_install = mc_versions.await?.iter().find(|v| snapshot || v.stable).cloned().expect("Unable to select a Minecraft version automatically")
    }
    // Yes, this duplicated code could be abstracted by being clever with generics, but this is easier.
    if let Some(loader_version) = loader_version {
        if let Some(found) = loader_versions.await?.iter().find(|v| v.version.to_string().eq(&loader_version)).cloned() {
            loader_version_to_install = found;
        } else {
            return Err(anyhow!("Could not find Loader version {}", loader_version))
        }
    } else {
        loader_version_to_install = loader_versions.await?.iter().find(|v| loader_beta || v.version.pre.is_empty()).cloned().expect("Unable to select a Loader version automatically")
    }

    Ok((mc_version_to_install, loader_version_to_install))
}
use std::path::PathBuf;
use anyhow::anyhow;
use clap::{Parser, Subcommand};
use crate::installer;
use crate::installer::{ClientInstallation, LoaderVersion, MinecraftVersion, ServerInstallation};

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
        #[arg(short, long, value_name = "'.minecraft' DIRECTORY")]
        install_dir: Option<PathBuf>,

        #[arg(short='s',long)]
        snapshot: bool,
        #[arg(short='b',long)]
        loader_beta: bool,

        #[arg(short, long)]
        no_profile: bool
    },
    Server {
        #[arg(short='M',long)]
        minecraft_version: Option<String>,
        #[arg(short='L',long)]
        loader_version: Option<String>,
        #[arg(short, long)]
        install_dir: PathBuf,

        #[arg(short='s',long)]
        snapshot: bool,
        #[arg(short='b',long)]
        loader_beta: bool,

        #[arg(short, long)]
        create_scripts: bool,
        // #[arg(short, long)]
        // download_server: bool
    }
}

pub async fn cli(args: SubCommands) -> anyhow::Result<()> {
    match args {
            SubCommands::Install { subcommand} => {
                match subcommand {
                    InstallSubcommands::Client { minecraft_version, loader_version, snapshot, loader_beta, no_profile, install_dir } => {
                        let (mc_version_to_install, loader_version_to_install) =  get_versions(minecraft_version, loader_version, snapshot, loader_beta).await?;
                        let install_dir = install_dir.unwrap_or_else(installer::get_default_client_directory);

                        installer::install_client(ClientInstallation {
                            minecraft_version: mc_version_to_install,
                            loader_version: loader_version_to_install,
                            install_dir,
                            generate_profile: !no_profile
                        }).await?;
                    }

                    InstallSubcommands::Server { minecraft_version, loader_version, install_dir, snapshot, loader_beta, create_scripts, /*download_server*/ } => {
                        let (mc_version_to_install, loader_version_to_install) = get_versions(minecraft_version, loader_version, snapshot, loader_beta).await?;

                        installer::install_server(ServerInstallation {
                            minecraft_version: mc_version_to_install,
                            loader_version: loader_version_to_install,
                            install_location: install_dir,
                            download_jar: false,
                            generate_script: create_scripts
                        }).await?;
                    }
                }
            }
        }

    Ok(())
}

async fn get_versions(minecraft_version: Option<String>, loader_version: Option<String>, snapshot: bool, loader_beta: bool) -> anyhow::Result<(MinecraftVersion, LoaderVersion)> {
    let mc_versions = installer::fetch_minecraft_versions();
    let loader_versions = installer::fetch_loader_versions();

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
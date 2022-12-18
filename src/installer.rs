use std::fmt::Display;
use std::fs::File;
use std::path::PathBuf;
use std::{collections::HashMap, fs::OpenOptions};

use crate::ICON;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Installation {
    #[default]
    Client,
    Server,
}

#[derive(Debug, Clone)]
pub struct ClientInstallation {
    pub minecraft_version: MinecraftVersion,
    pub loader_version: LoaderVersion,
    pub install_location: PathBuf,
    pub generate_profile: bool,
}

#[derive(Debug, Clone)]
pub struct ServerInstallation {
    pub minecraft_version: MinecraftVersion,
    pub loader_version: LoaderVersion,
    pub install_location: PathBuf,
    pub download_jar: bool,
    pub generate_script: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MinecraftVersion {
    pub version: String,
    pub stable: bool,
}

impl Display for MinecraftVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LoaderVersion {
    pub separator: String,
    pub build: u32,
    pub maven: String,
    pub version: String,
}

impl Display for LoaderVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}

pub async fn fetch_minecraft_versions() -> Result<Vec<MinecraftVersion>> {
    Ok(reqwest::get("https://meta.quiltmc.org/v3/versions/game")
        .await?
        .json()
        .await?)
}

pub async fn fetch_loader_versions() -> Result<Vec<LoaderVersion>> {
    Ok(reqwest::get("https://meta.quiltmc.org/v3/versions/loader")
        .await?
        .json()
        .await?)
}

#[derive(Serialize, Deserialize)]
struct LaunchProfiles {
    profiles: HashMap<String, Profile>,
    settings: serde_json::Value,
    version: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    name: String,
    created: Option<DateTime<Utc>>,
    last_version_id: String,
    icon: String,
}

pub async fn install_client(args: ClientInstallation) -> Result<()> {
    println!("Installing client: {:#?}", args);

    // Verify install location
    if !args.install_location.exists() {
        return Err(anyhow!(
            "Target directory doesn't exist: {:?}",
            args.install_location
        ));
    }

    // Resolve profile directory
    let profile_name = format!(
        "quilt-loader-{}-{}",
        args.loader_version.version, args.minecraft_version.version
    );
    let profile_dir = args.install_location.join("versions").join(&profile_name);

    if profile_dir.exists() {
        // Delete existing profile
        std::fs::remove_dir_all(&profile_dir)?;
    } else {
        // Else create the directory
        std::fs::create_dir_all(&profile_dir)?;
    }

    // An empty jar file to make the vanilla launcher happy
    File::create(profile_dir.join(&profile_name).with_extension("jar"))?;

    // Create launch json
    let mut file = File::create(profile_dir.join(&profile_name).with_extension("json"))?;

    // Download launch json
    let response = reqwest::get(format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
        &args.minecraft_version.version, &args.loader_version.version
    ))
    .await?
    .text()
    .await?;

    // Hack-Fix:
    // Quilt-meta specifies both hashed and intermediary, but providing both to quilt-loader causes it to silently fail remapping.
    // This really shouldn't be fixed here in the installer, but we need a solution now.
    let mut json: serde_json::Value = serde_json::from_str(&response).unwrap();
    let libs = json
        .as_object_mut()
        .unwrap()
        .get_mut("libraries")
        .unwrap()
        .as_array_mut()
        .unwrap();
    libs.retain(|lib| {
        !lib.as_object()
            .unwrap()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("org.quiltmc:hashed")
    });
    let response = serde_json::to_string(&json).unwrap();
    // End of hack-fix

    std::io::copy(&mut response.as_bytes(), &mut file)?;

    // Generate profile
    if args.generate_profile {
        let file = OpenOptions::new().read(true).write(true).open(
            args.install_location
                .join("launcher_profiles")
                .with_extension("json"),
        )?;
        let mut launch_profiles: LaunchProfiles = serde_json::from_reader(&file)?;

        launch_profiles.profiles.insert(
            profile_name.clone(),
            Profile {
                name: format!("quilt-loader-{}", &args.minecraft_version.version),
                created: Some(Utc::now()),
                last_version_id: profile_name,
                icon: format!("data:image/png;base64,{}", base64::encode(ICON)),
            },
        );

        serde_json::to_writer_pretty(file, &launch_profiles)?;
    }

    Ok(())
}

pub async fn install_server(args: ServerInstallation) -> Result<()> {
    println!("Not installing server :(\n{:#?}", args);
    Ok(())
}

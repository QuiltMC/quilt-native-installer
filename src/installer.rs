use std::fmt::Display;
use std::{
    fs::{create_dir_all, remove_dir_all, File, OpenOptions},
    io::{copy, Seek},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::get;
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Value};

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
    Ok(get("https://meta.quiltmc.org/v3/versions/game")
        .await?
        .json()
        .await?)
}

pub async fn fetch_loader_versions() -> Result<Vec<LoaderVersion>> {
    Ok(get("https://meta.quiltmc.org/v3/versions/loader")
        .await?
        .json()
        .await?)
}

/// `Deserialize` is not implemented for a reason
///
/// DO NOT deserialise `launcher_profiles.json` into this incomplete struct and write it back as it will cause **data loss**
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    name: String,
    #[serde(rename = "type")]
    profile_type: String,
    created: DateTime<Utc>,
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
        remove_dir_all(&profile_dir)?;
    }
    create_dir_all(&profile_dir)?;

    // An empty jar file to make the vanilla launcher happy
    File::create(profile_dir.join(&profile_name).with_extension("jar"))?;

    // Create launch json
    let mut file = File::create(profile_dir.join(&profile_name).with_extension("json"))?;

    // Download launch json
    let response = get(format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
        &args.minecraft_version.version, &args.loader_version.version
    ))
    .await?
    .text()
    .await?;

    // Hack-Fix:
    // Quilt-meta specifies both hashed and intermediary, but providing both to quilt-loader causes it to silently fail remapping.
    // This really shouldn't be fixed here in the installer, but we need a solution now.
    let mut json: Value = serde_json::from_str(&response).unwrap();
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

    copy(&mut response.as_bytes(), &mut file)?;

    // Generate profile
    if args.generate_profile {
        let mut file = OpenOptions::new().read(true).write(true).open(
            args.install_location
                .join("launcher_profiles")
                .with_extension("json"),
        )?;

        let mut launcher_profiles: Value = serde_json::from_reader(&file)?;
        file.set_len(0)?;
        file.rewind()?;

        launcher_profiles
            .get_mut("profiles")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(
                profile_name.clone(),
                to_value(Profile {
                    name: format!("Quilt Loader {}", &args.minecraft_version.version),
                    profile_type: "custom".into(),
                    created: Utc::now(),
                    last_version_id: profile_name,
                    icon: format!("data:image/png;base64,{}", base64::encode(crate::ICON)),
                })?,
            );

        serde_json::to_writer_pretty(file, &launcher_profiles)?;
    }

    Ok(())
}

pub async fn install_server(args: ServerInstallation) -> Result<()> {
    println!("Not installing server :(\n{:#?}", args);
    Ok(())
}

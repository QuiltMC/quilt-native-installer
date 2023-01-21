use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::ICON;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installation {
    Client,
    Server,
}

#[derive(Debug, Clone)]
pub struct ClientInstallation {
    pub minecraft_version: MinecraftVersion,
    pub loader_version: LoaderVersion,
    pub install_location: PathBuf,
    pub generate_profile: bool
}

#[derive(Debug, Clone)]
pub struct ServerInstallation {
    pub minecraft_version: MinecraftVersion,
    pub loader_version: LoaderVersion,
    pub install_location: PathBuf,
    pub download_jar: bool,
    pub generate_script: bool
}


#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MinecraftVersion {
    pub version: String,
    pub stable: bool
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
    Ok(reqwest::get("https://meta.quiltmc.org/v3/versions/game").await?.json().await?)
}

pub async fn fetch_loader_versions() -> Result<Vec<LoaderVersion>> {
    Ok(reqwest::get("https://meta.quiltmc.org/v3/versions/loader").await?.json().await?)
}

#[derive(Serialize, Deserialize)]
struct LaunchProfiles {
    profiles: HashMap<String, serde_json::Value>,
    settings: serde_json::Value,
    version: u32
}

pub async fn install_client(args: ClientInstallation) -> Result<()> {
    println!("Installing client: {:#?}", args);

    // Verify install location
    if !args.install_location.exists() {
        return Err(anyhow!("Target directory doesn't exist: {:?}", args.install_location));
    }

    // Resolve profile directory
    let profile_name = format!("quilt-loader-{}-{}", args.loader_version.version, args.minecraft_version.version);
    let short_profile_name = format!("quilt-loader-{}", &args.minecraft_version.version);
    let mut profile_dir = args.install_location.clone();
    profile_dir.push("versions");
    profile_dir.push(&profile_name);

    // Delete existing profile
    if profile_dir.exists() {
        std::fs::remove_dir_all(&profile_dir)?;
    }

    // Create directory
    std::fs::create_dir_all(&profile_dir)?;

    // NOTE: This is an empty jar file to make the vanilla launcher happy
    let mut jar_path = profile_dir.clone();
    jar_path.push(format!("{}.jar", &profile_name));
    File::create(jar_path)?;
    
    // Create launch json
    let mut json_path = profile_dir.clone();
    json_path.push(format!("{}.json", &profile_name));
    let mut file = File::create(json_path)?;

    // Download launch json
    let response = reqwest::get(format!("https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json", &args.minecraft_version.version, &args.loader_version.version)).await?.text().await?;

    // Hack-Fix:
    // Quilt-meta specifies both hashed and intermediary, but providing both to quilt-loader causes it to silently fail remapping.
    // This really shouldn't be fixed here in the installer, but we need a solution now.
    let mut json: serde_json::Value = serde_json::from_str(&response).unwrap();
    let libs = json.as_object_mut().unwrap().get_mut("libraries").unwrap().as_array_mut().unwrap();
    libs.retain(|lib| !lib.as_object().unwrap().get("name").unwrap().as_str().unwrap().starts_with("org.quiltmc:hashed"));
    let response = serde_json::to_string(&json).unwrap();
    // End of hack-fix

    std::io::copy(&mut response.as_bytes(), &mut file)?;


    // Generate profile
    if args.generate_profile {
        let mut profiles_json = args.install_location.clone();
        profiles_json.push("launcher_profiles.json");

        let read_file = File::open(&profiles_json)?;
        let mut profiles: LaunchProfiles = serde_json::from_reader(read_file)?;

        let mut new_profile = profiles.profiles[&short_profile_name].as_object().unwrap_or(&serde_json::Map::new()).clone();
        new_profile.insert("name".into(), serde_json::Value::String(format!("quilt-loader-{}", &args.minecraft_version.version)));
        new_profile.insert("type".into(), serde_json::Value::String("custom".into()));
        new_profile.insert("created".into(), serde_json::Value::String(format!("{:?}", Utc::now())));
        new_profile.insert("lastVersionId".into(), serde_json::Value::String(profile_name.clone()));
        new_profile.insert("icon".into(), serde_json::Value::String(format!("data:image/png;base64,{}", base64::encode(ICON))));
        profiles.profiles.insert(short_profile_name, serde_json::Value::Object(new_profile));

        let write_file = File::create(&profiles_json)?;
        serde_json::to_writer_pretty(write_file, &profiles)?;
    }

    Ok(())
}

pub async fn install_server(args: ServerInstallation) -> Result<()> {
    println!("Installing server: {:#?}", args);
    Ok(())
}

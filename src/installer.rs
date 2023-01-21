use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Seek},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use reqwest::get;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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

impl std::fmt::Display for MinecraftVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LoaderVersion {
    pub separator: char,
    pub build: u32,
    pub maven: String,
    pub version: Version,
}

impl std::fmt::Display for LoaderVersion {
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherProfiles {
    profiles: HashMap<String, Profile>,
    #[serde(flatten)]
    other: Map<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    name: String,
    #[serde(rename = "type")]
    profile_type: String,
    created: DateTime<Utc>,
    last_version_id: String,
    icon: String,
    #[serde(flatten)]
    other: Map<String, Value>,
}

pub async fn install_client(args: ClientInstallation) -> Result<()> {
    println!("Installing client: {:#?}", args);

    // Verify install location
    if !args.install_location.join("launcher_profiles.json").exists() {
        return Err(anyhow!(
            "{} is not a valid installation directory",
            args.install_location.display(),
        ));
    }

    // Resolve profile directory
    let profile_name = format!("quilt-loader-{}-{}", args.loader_version, args.minecraft_version);
    let profile_dir = args.install_location.join("versions").join(&profile_name);

    // Delete existing profile
    if profile_dir.exists() {
        fs::remove_dir_all(&profile_dir)?;
    }

    // Create profile directory
    fs::create_dir_all(&profile_dir)?;

    // Create an empty jar file to make the vanilla launcher happy
    File::create(profile_dir.join(profile_name.clone() + ".jar"))?;

    // Create launch json
    let mut file = File::create(profile_dir.join(profile_name.clone() + ".json"))?;

    // Download launch json
    let mut response = get(format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
        &args.minecraft_version.version, &args.loader_version.version
    ))
    .await?
    .text()
    .await?;

    // Hack-Fix:
    // Was fixed in versions above 0.17.7
    if args.loader_version.version < Version::new(0, 17, 7) {
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
        response = serde_json::to_string(&json).unwrap();
    }
    // End of hack-fix

    io::copy(&mut response.as_bytes(), &mut file)?;

    // Generate profile
    if args.generate_profile {
        let mut file = fs::OpenOptions::new().read(true).write(true).open(
            args.install_location
                .join("launcher_profiles")
                .with_extension("json"),
        )?;

        let mut launcher_profiles: LauncherProfiles = serde_json::from_reader(&file)?;
        file.set_len(0)?;
        file.rewind()?;

        launcher_profiles.profiles.insert(
            profile_name.clone(),
            Profile {
                name: format!("Quilt Loader {}", &args.minecraft_version.version),
                profile_type: "custom".into(),
                created: Utc::now(),
                last_version_id: profile_name,
                icon: format!("data:image/png;base64,{}", BASE64.encode(crate::ICON)),
                other: Map::new(),
            },
        );

        serde_json::to_writer_pretty(file, &launcher_profiles)?;
    }

    Ok(())
}

pub async fn install_server(args: ServerInstallation) -> Result<()> {
    println!("Installing server\n{:#?}", args);
    println!("Server installation hasn't been implemented yet!");
    Ok(())
}

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Seek, Write},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use iced::futures::future::try_join_all;
use reqwest::get;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use zip::CompressionMethod;

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
    pub install_dir: PathBuf,
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


#[cfg(target_os = "windows")]
pub fn get_default_client_directory() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").unwrap()).join(".minecraft")
}

#[cfg(target_os = "macos")]
pub fn get_default_client_directory() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap())
        .join("Library")
        .join("Application Support")
        .join("minecraft")
}

#[cfg(target_os = "linux")]
pub fn get_default_client_directory() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap()).join(".minecraft")
}

pub async fn install_client(args: ClientInstallation) -> Result<()> {
    // TODO: make pretty
    println!("Installing client: {args:#?}");

    // Verify install location
    if !args.install_dir.join("launcher_profiles.json").exists() {
        return Err(anyhow!(
            "{} is not a valid installation directory",
            args.install_dir.display(),
        ));
    }

    // Resolve profile directory
    let profile_name = format!("quilt-loader-{}-{}", args.loader_version, args.minecraft_version);
    let profile_dir = args.install_dir.join("versions").join(&profile_name);

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

    response = hack_fix(response, args.loader_version);

    io::copy(&mut response.as_bytes(), &mut file)?;

    // Generate profile
    if args.generate_profile {
        let mut file = fs::OpenOptions::new().read(true).write(true).open(
            args.install_dir
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

    println!("Client installed successfully.");
    Ok(())
}

pub fn hack_fix(response: String, loader_version: LoaderVersion) -> String {
    if loader_version.version < Version::new(0, 17, 7) {
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

        return serde_json::to_string(&json).unwrap()
    }

    response
}

pub async fn install_server(args: ServerInstallation) -> Result<()> {
    println!("Installing server\n{args:#?}");

    // Download launch json
    let mut response = get(format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/server/json",
        &args.minecraft_version.version, &args.loader_version.version
    ))
        .await?
        .text()
        .await?;

    response = hack_fix(response, args.loader_version);

    let json: Value = serde_json::from_str(&response).unwrap();
    let json = json.as_object().unwrap();

    // first, read libs
    let libraries = json.get("libraries").unwrap().as_array().unwrap();

    let mut lib_path = args.install_location.clone();
    lib_path.push("libraries/");

    let library_futures: Vec<_> = libraries.iter().map(|lib| {
        let lib = lib.as_object().unwrap();
        let name = lib.get("name").expect("Library had no name").as_str().unwrap().to_string();
        let maven = lib.get("url").expect("Library had no maven url").as_str().unwrap().to_string();

        download_library(&lib_path, name, maven)
    }).collect();

    let library_paths: Vec<PathBuf> = try_join_all(library_futures).await?;
    let mut launch_jar_path = args.install_location.clone();
    launch_jar_path.push("quilt-server-launch.jar");

    create_launch_jar(launch_jar_path, json.get("launcherMainClass").unwrap().as_str().unwrap().to_string(), library_paths).await?;


    Ok(())
}

async fn download_library(dir: &PathBuf, name: String, maven: String) -> Result<PathBuf> {
    let response = get(maven_to_url(maven, &name));
    let mut file_path = dir.clone();

    file_path.push(split_artifact(&name));

    let _ = fs::create_dir_all(file_path.parent().unwrap());
    let mut file = tokio::fs::File::create(&file_path).await?;
    let mut content = io::Cursor::new(response.await?.bytes().await?);

    tokio::io::copy(&mut content, &mut file).await?;

    Ok(file_path)
}

fn maven_to_url(maven_url: String, artifact_notation: &String) -> String {
    return maven_url + split_artifact(artifact_notation).as_str();
}

fn split_artifact(artifact_notation: &String) -> String {
    let parts: Vec<&str> = artifact_notation.splitn(3, ":").collect();

    parts[0].replace('.', "/") + // group
        "/" + parts[1] + // artifact name
        "/" + parts[2] + // version
        "/" + parts[1] +
        "-" + parts[2] + ".jar"
}


async fn create_launch_jar(path: PathBuf, main_class: String, libraries: Vec<PathBuf>) -> Result<()> {
    if path.try_exists()? {
        fs::remove_file(&path).expect("Failed to delete old launch jar");
    }

    let parent_dir = path.parent().unwrap();
    let file = File::create(&path)?;
    let mut zip = zip::ZipWriter::new(file);

    // TODO: there's probably a more efficient way to write this, but does it really matter?
    let options = zip::write::FileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("META-INF/MANIFEST.MF", options)?;
    // create class-path entry
    let mut cp: String = String::new();

    for x in libraries {
        cp.push(' ');
        cp.push_str(x.strip_prefix(parent_dir)?.as_os_str().to_str().unwrap());
    }

    zip.write_all(format!("Manifest-Version: 1.0\nMain-Class: {main_class}\nClass-Path:{cp}").as_bytes())?;
    zip.finish()?;
    Ok(())
}
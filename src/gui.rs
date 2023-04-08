use std::borrow::Cow;
use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::{anyhow, Error, Result};
use iced::widget::{Radio, Space};
use iced::window::{Icon, Settings as WindowSettings};
use iced::Theme;
use iced::{
    alignment::{Alignment, Horizontal},
    executor,
    widget::{Button, Checkbox, Column, PickList, ProgressBar, Row, Rule, Text, TextInput},
    Application, Command, Element, Length, Settings,
};
use native_dialog::{FileDialog, MessageDialog, MessageType};
use png::Transformations;
use reqwest::Client;

use crate::installer::{
    fetch_loader_versions, fetch_minecraft_versions, get_default_client_directory, install_client,
    install_server, ClientInstallation, Installation, LoaderVersion, MinecraftVersion,
    ServerInstallation,
};

pub fn run(client: Client) -> Result<()> {
    State::run(Settings {
        window: WindowSettings {
            size: (600, 300),
            resizable: false,
            icon: Some(create_icon()?),
            ..Default::default()
        },
        flags: client,
        ..Default::default()
    })?;

    Ok(())
}

fn create_icon() -> Result<Icon> {
    let mut decoder = png::Decoder::new(crate::ICON);
    decoder.set_transformations(Transformations::EXPAND);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;
    let bytes = &buffer[..info.buffer_size()];
    Ok(Icon::from_rgba(bytes.to_vec(), info.width, info.height)?)
}

#[derive(Debug, Default)]
struct State {
    // Minecraft version picker
    minecraft_versions: Vec<MinecraftVersion>,
    selected_minecraft_version: Option<MinecraftVersion>,
    show_snapshots: bool,

    // Quilt Loader version picker
    loader_versions: Vec<LoaderVersion>,
    selected_loader_version: Option<LoaderVersion>,
    show_betas: bool,

    installation_type: Installation,

    // Client settings
    client_location: PathBuf,
    generate_profile: bool,

    // Server settings
    server_location: PathBuf,
    download_server_jar: bool,
    generate_launch_script: bool,

    // Progress information
    is_installing: bool,
    progress: f32,

    // HTTP reqwest client
    client: Client,
}

#[derive(Debug)]
enum Message {
    Interaction(Interaction),
    Install,
    BrowseClientLocation,
    BrowseServerLocation,
    SetMcVersions(Result<Vec<MinecraftVersion>>),
    SetLoaderVersions(Result<Vec<LoaderVersion>>),
    DoneInstalling(Result<()>),
    Error(Error),
}

#[derive(Debug, Clone)]
enum Interaction {
    ChangeClientLocation(PathBuf),
    BrowseClientLocation,
    Install,
    SelectInstallation(Installation),
    SelectLoaderVersion(LoaderVersion),
    SelectMcVersion(MinecraftVersion),
    SetShowSnapshots(bool),
    SetShowBetas(bool),
    GenerateLaunchScript(bool),
    GenerateProfile(bool),
    ChangeServerLocation(PathBuf),
    BrowseServerLocation,
    DownloadServerJar(bool),
}

impl From<Message> for Command<Message> {
    fn from(m: Message) -> Self {
        Self::perform(async { m }, |t| t)
    }
}

impl Application for State {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = Client;
    type Theme = Theme;

    fn theme(&self) -> Self::Theme {
        use dark_light::Mode;
        match dark_light::detect() {
            Mode::Light => Theme::Light,
            Mode::Dark | Mode::Default => Theme::Dark,
        }
    }

    fn new(client: Client) -> (Self, Command<Self::Message>) {
        (
            State {
                client_location: get_default_client_directory(),
                generate_profile: true,
                server_location: std::env::current_dir().unwrap_or_default(),
                download_server_jar: true,
                generate_launch_script: true,
                client: client.clone(),
                ..Default::default()
            },
            Command::batch([
                Command::perform(
                    fetch_minecraft_versions(client.clone()),
                    Message::SetMcVersions,
                ),
                Command::perform(fetch_loader_versions(client), Message::SetLoaderVersions),
            ]),
        )
    }

    fn title(&self) -> String {
        "Quilt Installer".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Interaction(interaction) => match interaction {
                Interaction::ChangeClientLocation(location) => self.client_location = location,
                Interaction::BrowseClientLocation => return Message::BrowseClientLocation.into(),
                Interaction::Install => return Message::Install.into(),
                Interaction::SelectInstallation(i) => self.installation_type = i,
                Interaction::SelectLoaderVersion(v) => self.selected_loader_version = Some(v),
                Interaction::SelectMcVersion(v) => self.selected_minecraft_version = Some(v),
                Interaction::SetShowSnapshots(enable) => {
                    self.show_snapshots = enable;
                    self.selected_minecraft_version = self
                        .minecraft_versions
                        .iter()
                        .find(|v| enable || v.stable)
                        .cloned();
                }
                Interaction::SetShowBetas(enable) => {
                    self.show_betas = enable;
                    self.selected_loader_version = self
                        .loader_versions
                        .iter()
                        .find(|v| enable || v.version.pre.is_empty())
                        .cloned();
                }
                Interaction::GenerateLaunchScript(value) => self.generate_launch_script = value,
                Interaction::GenerateProfile(value) => self.generate_profile = value,
                Interaction::ChangeServerLocation(location) => self.server_location = location,
                Interaction::BrowseServerLocation => return Message::BrowseServerLocation.into(),
                Interaction::DownloadServerJar(value) => self.download_server_jar = value,
            },
            Message::SetMcVersions(result) => {
                match result {
                    Ok(versions) => self.minecraft_versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }
                if self.selected_minecraft_version.is_none() {
                    self.selected_minecraft_version =
                        self.minecraft_versions.iter().find(|v| v.stable).cloned();
                }
            }
            Message::SetLoaderVersions(result) => {
                match result {
                    Ok(versions) => self.loader_versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }
                if self.selected_loader_version.is_none() {
                    self.selected_loader_version = self
                        .loader_versions
                        .iter()
                        .find(|v| v.version.pre.is_empty())
                        .cloned();
                }
            }
            Message::BrowseClientLocation => {
                let mut dialog = FileDialog::new();
                let working_dir = std::env::current_dir();
                if self.client_location.is_dir() {
                    dialog = dialog.set_location(&self.client_location);
                } else if working_dir.is_ok() {
                    dialog = dialog.set_location(working_dir.as_deref().unwrap())
                }
                let result = dialog.show_open_single_dir();
                match result {
                    Ok(Some(path)) => self.client_location = path,
                    Ok(None) => (),
                    Err(error) => return Message::Error(error.into()).into(),
                }
            }
            Message::BrowseServerLocation => {
                let mut dialog = FileDialog::new();
                let working_dir = std::env::current_dir();
                if self.client_location.is_dir() {
                    dialog = dialog.set_location(&self.server_location);
                } else if working_dir.is_ok() {
                    dialog = dialog.set_location(working_dir.as_deref().unwrap())
                }
                let result = dialog.show_open_single_dir();
                match result {
                    Ok(Some(path)) => self.server_location = path,
                    Ok(None) => (),
                    Err(error) => return Message::Error(error.into()).into(),
                }
            }
            Message::Install => {
                self.is_installing = true;
                self.progress = 0.0;

                return match self.installation_type {
                    Installation::Client => Command::perform(
                        install_client(
                            self.client.clone(),
                            ClientInstallation {
                                minecraft_version: match &self.selected_minecraft_version {
                                    Some(s) => s.clone(),
                                    None => {
                                        return Message::Error(anyhow!(
                                            "No Minecraft version selected!"
                                        ))
                                        .into()
                                    }
                                },
                                loader_version: match &self.selected_loader_version {
                                    Some(s) => s.clone(),
                                    None => {
                                        return Message::Error(anyhow!(
                                            "No Loader version selected!"
                                        ))
                                        .into()
                                    }
                                },
                                install_dir: self.client_location.clone(),
                                generate_profile: self.generate_profile,
                            },
                        ),
                        Message::DoneInstalling,
                    ),
                    Installation::Server => Command::perform(
                        install_server(ServerInstallation {
                            minecraft_version: match &self.selected_minecraft_version {
                                Some(s) => s.clone(),
                                None => {
                                    return Message::Error(anyhow!(
                                        "No Minecraft version selected!"
                                    ))
                                    .into()
                                }
                            },
                            loader_version: match &self.selected_loader_version {
                                Some(s) => s.clone(),
                                None => {
                                    return Message::Error(anyhow!("No Loader version selected!"))
                                        .into()
                                }
                            },
                            install_dir: self.server_location.clone(),
                            download_jar: self.download_server_jar,
                            generate_script: self.generate_launch_script,
                        }),
                        Message::DoneInstalling,
                    ),
                };
            }
            Message::DoneInstalling(res) => {
                self.is_installing = false;
                self.progress = 1.0;

                if let Err(e) = res {
                    return Message::Error(e).into();
                }
            }
            Message::Error(error) => {
                eprintln!("{error:?}");
                MessageDialog::new()
                    .set_title("Quilt Installer Error")
                    .set_text(&error.to_string())
                    .set_type(MessageType::Error)
                    .show_alert()
                    .unwrap();
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let installation_label = Text::new("Installation:").width(140);
        let installation_client = Radio::new(
            Installation::Client,
            "Client",
            Some(self.installation_type),
            Interaction::SelectInstallation,
        );
        let installation_server = Radio::new(
            Installation::Server,
            "Server",
            Some(self.installation_type),
            Interaction::SelectInstallation,
        );
        let installation_row = Row::new()
            .push(installation_label)
            .push(installation_client)
            .push(installation_server)
            .width(Length::Fill)
            .align_items(Alignment::Fill)
            .spacing(50)
            .padding(5);

        let minecraft_version_label = Text::new("Minecraft version:").width(140);
        let minecraft_version_list = PickList::new(
            Cow::from_iter(
                self.minecraft_versions
                    .iter()
                    .filter(|v| self.show_snapshots || v.stable)
                    .cloned(),
            ),
            self.selected_minecraft_version.clone(),
            Interaction::SelectMcVersion,
        )
        .width(200);
        let enable_snapshots = Checkbox::new(
            "Show snapshots",
            self.show_snapshots,
            Interaction::SetShowSnapshots,
        );
        let mc_row = Row::new()
            .push(minecraft_version_label)
            .push(minecraft_version_list)
            .push(Space::new(20, 0))
            .push(enable_snapshots)
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(5)
            .padding(5);

        let loader_version_label = Text::new("Loader version:").width(140);
        let loader_version_list = PickList::new(
            Cow::from_iter(
                self.loader_versions
                    .iter()
                    .filter(|v| self.show_betas || v.version.pre.is_empty())
                    .cloned(),
            ),
            self.selected_loader_version.clone(),
            Interaction::SelectLoaderVersion,
        )
        .width(200);
        let enable_betas = Checkbox::new("Show betas", self.show_betas, Interaction::SetShowBetas);
        let loader_row = Row::new()
            .push(loader_version_label)
            .push(loader_version_list)
            .push(Space::new(20, 0))
            .push(enable_betas)
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(5)
            .padding(5);

        let client_location_label = Text::new("Directory:").width(140);
        let client_location_input = TextInput::new(
            "Install location",
            self.client_location.to_str().unwrap(),
            |s| Interaction::ChangeClientLocation(PathBuf::from(s)),
        )
        .padding(5);
        let client_loction_browse =
            Button::new(Text::new("Browse...")).on_press(Interaction::BrowseClientLocation);
        let client_location_row = Row::new()
            .push(client_location_label)
            .push(client_location_input)
            .push(client_loction_browse)
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(5)
            .padding(5);

        let client_options_label = Text::new("Options:").width(140);
        let create_profile = Checkbox::new(
            "Generate profile",
            self.generate_profile,
            Interaction::GenerateProfile,
        );
        let client_options_row = Row::new()
            .push(client_options_label)
            .push(create_profile)
            .align_items(Alignment::Center)
            .spacing(5)
            .padding(5);

        let server_location_label = Text::new("Directory:").width(140);
        let server_location_input = TextInput::new(
            "Install location",
            self.server_location.to_str().unwrap(),
            |s| Interaction::ChangeServerLocation(PathBuf::from(s)),
        )
        .padding(5);
        let server_loction_browse =
            Button::new(Text::new("Browse...")).on_press(Interaction::BrowseServerLocation);
        let server_location_row = Row::new()
            .push(server_location_label)
            .push(server_location_input)
            .push(server_loction_browse)
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(5)
            .padding(5);

        let server_options_label = Text::new("Options:").width(140);
        let download_server_jar = Checkbox::new(
            "Download server jar",
            self.download_server_jar,
            Interaction::DownloadServerJar,
        );
        let generate_launch_script = Checkbox::new(
            "Generate launch script",
            self.generate_launch_script,
            Interaction::GenerateLaunchScript,
        );
        let server_options_row = Row::new()
            .push(server_options_label)
            .push(download_server_jar)
            .push(Space::new(35, 0))
            .push(generate_launch_script)
            .align_items(Alignment::Center)
            .spacing(5)
            .padding(5);

        let mut column = Column::new()
            .padding(5)
            .spacing(5)
            .push(installation_row)
            .push(mc_row)
            .push(loader_row)
            .push(Rule::horizontal(5));

        column = match self.installation_type {
            Installation::Client => column.push(client_location_row).push(client_options_row),
            Installation::Server => column.push(server_location_row).push(server_options_row),
        };

        let button_label = Text::new("Install")
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill);
        let mut button = Button::new(button_label).width(Length::Fill);
        if !self.is_installing {
            button = button.on_press(Interaction::Install);
        }
        let progress = ProgressBar::new(0.0..=1.0, self.progress);
        column = column.push(button).push(progress);

        let content: Element<Interaction> = column.into();
        content.map(Message::Interaction)
    }
}

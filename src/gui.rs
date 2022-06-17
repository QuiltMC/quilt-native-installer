use std::borrow::Cow;
use std::fmt::{Display, Debug};
use std::path::PathBuf;

use anyhow::{Result, Error, anyhow};
use iced::window::Icon;
use iced::{Settings, PickList, pick_list, Application, executor, Command, Clipboard, Column, Text, Checkbox, Length, Row, Align, Rule, TextInput, text_input, Button, button, ProgressBar, HorizontalAlignment, Element};
use native_dialog::FileDialog;
use png::Transformations;

use crate::Args;
use crate::installer::{Installation, ClientInstallation, fetch_minecraft_versions, fetch_loader_versions, LoaderVersion, MinecraftVersion, install_client, ServerInstallation, install_server};

pub fn run(args: Args) -> Result<()> {
    let mut setttings = Settings::default();
    setttings.flags = args;
    setttings.window.size = (600, 300);
    setttings.window.resizable = false;
    setttings.window.icon = Some(create_icon()?);
    State::run(setttings)?;
    
    Ok(())
}


fn create_icon() -> Result<Icon> {
    let mut decoder = png::Decoder::new(crate::ICON);
    decoder.set_transformations(Transformations::EXPAND);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;
    let bytes = &buffer[..info.buffer_size()];
    let icon = Icon::from_rgba(bytes.to_vec(), info.width, info.height)?;
    Ok(icon)
}

#[derive(Debug, Default)]
struct State {
    minecraft_pick_list: pick_list::State<MinecraftVersion>,
    minecraft_versions: Vec<MinecraftVersion>,
    selected_minecraft_version: Option<MinecraftVersion>,
    show_snapshots: bool,

    loader_pick_list: pick_list::State<LoaderVersion>,
    loader_versions: Vec<LoaderVersion>,
    selected_loader_version: Option<LoaderVersion>,
    show_betas: bool,

    installation_pick_list: pick_list::State<Installation>,
    selected_installation: Installation,

    client_location_input: text_input::State,
    client_location: PathBuf,
    client_location_browse: button::State,
    generate_profile: bool,
    
    server_location_input: text_input::State,
    server_location: PathBuf,
    server_location_browse: button::State,
    download_server_jar: bool,
    generate_launch_script: bool,

    install_button: button::State,
    is_installing: bool,

    progress: f32,
}


impl Installation {
    const ALL: &'static [Installation] = &[Self::Client, Self::Server];
}

impl Display for Installation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Installation::Client => write!(f, "Client"),
            Installation::Server => write!(f, "Server"),
        }
    }
}

impl Default for Installation {
    fn default() -> Self {
        Self::Client
    }
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

// Gui interactions
#[derive(Debug, Clone)]
enum Interaction {
    ChangeClientLocation(PathBuf),
    BrowseClientLocation,
    Install,
    SelectInstallation(Installation),
    SelectLoaderVersion(LoaderVersion),
    SelectMcVersion(MinecraftVersion),
    EnableSnapshots(bool),
    EnableBetas(bool),
    GenerateLaunchScript(bool),
    GenerateProfile(bool),
    ChangeServerLocation(PathBuf),
    BrowseServerLocation,
    DownloadServerJar(bool),
}

impl Into<Command<Message>> for Message {
    fn into(self) -> Command<Message> {
        async {
            self
        }.into()
    }
}

#[cfg(target_os = "windows")]
fn get_default_client_directory() -> PathBuf {
    let mut dir = PathBuf::from(std::env::var("APPDATA").unwrap());
    dir.push(".minecraft");
    dir
}

#[cfg(target_os = "macos")]
fn get_default_client_directory() {
    let mut dir = PathBuf::from(std::env::var("HOME").unwrap());
    dir.push("Library");
    dir.push("Application Support");
    dir.push("minecraft");
    dir
}

#[cfg(target_os = "linux")]
fn get_default_client_directory() -> PathBuf {
    let mut dir = PathBuf::from(std::env::var("HOME").unwrap());
    dir.push(".minecraft");
    dir
}

impl Application for State {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = Args;

    fn new(_args: Args) -> (Self, Command<Self::Message>) {
        (
            State{
                client_location: get_default_client_directory(),
                generate_profile: true,
                server_location: std::env::current_dir().unwrap_or_default(),
                download_server_jar: true,
                generate_launch_script: true,
                ..Default::default()
            },
            Command::batch([
                Command::perform(fetch_minecraft_versions(), Message::SetMcVersions),
                Command::perform(fetch_loader_versions(), Message::SetLoaderVersions),
            ])
        )
    }

    fn title(&self) -> String {
        "Quilt Installer".to_owned()
    }

    fn update(&mut self, message: Self::Message, _clipboard: &mut Clipboard) -> Command<Self::Message> {
        match message {
            Message::Interaction(interaction) => match interaction {
                Interaction::ChangeClientLocation(location) => self.client_location = location,
                Interaction::BrowseClientLocation => return Message::BrowseClientLocation.into(),
                Interaction::Install => return Message::Install.into(),
                Interaction::SelectInstallation(installation) => self.selected_installation = installation,
                Interaction::SelectLoaderVersion(version) => self.selected_loader_version = Some(version),
                Interaction::SelectMcVersion(version) => self.selected_minecraft_version = Some(version),
                Interaction::EnableSnapshots(enable) => self.show_snapshots = enable,
                Interaction::EnableBetas(enable) => self.show_betas = enable,
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
                    self.selected_minecraft_version = self.minecraft_versions.iter().filter(|v| v.stable).next().cloned();
                }
            },
            Message::SetLoaderVersions(result) => {
                match result {
                    Ok(versions) => self.loader_versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }
                if self.selected_loader_version.is_none() {
                    self.selected_loader_version = self.loader_versions.iter().filter(|v| !v.version.contains("beta")).next().cloned();
                }
            },
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
            },
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
            },
            Message::Install => {
                self.is_installing = true;
                self.progress = 0.0;

                match self.selected_installation {
                    Installation::Client => {
                        
                        if self.selected_minecraft_version.is_none() {
                            return Message::Error(anyhow!("No Minecraft version selected!")).into()
                        }
                        
                        if self.selected_loader_version.is_none() {
                            return Message::Error(anyhow!("No Loader version selected!")).into()
                        }

                        return Command::perform(
                            install_client(ClientInstallation{
                                minecraft_version: self.selected_minecraft_version.clone().unwrap(),
                                loader_version: self.selected_loader_version.clone().unwrap(),
                                install_location: self.client_location.clone(),
                                generate_profile: self.generate_profile
                            }),
                            Message::DoneInstalling
                        );
                    },
                    Installation::Server => {
                        if self.selected_minecraft_version.is_none() {
                            return Message::Error(anyhow!("No Minecraft version selected!")).into()
                        }
                        
                        if self.selected_loader_version.is_none() {
                            return Message::Error(anyhow!("No Loader version selected!")).into()
                        }

                        return Command::perform(
                            install_server(ServerInstallation {
                                minecraft_version: self.selected_minecraft_version.clone().unwrap(),
                                loader_version: self.selected_loader_version.clone().unwrap(),
                                install_location: self.server_location.clone(),
                                download_jar: self.download_server_jar,
                                generate_script: self.generate_launch_script,
                            }),
                            Message::DoneInstalling
                        );
                    },
                }
            }
            Message::DoneInstalling(res) => {
                self.is_installing = false;
                self.progress = 1.0;

                match res {
                    Ok(_) => (),
                    Err(e) => return Message::Error(e).into(),
                }
            }
            Message::Error(error) => {
                eprintln!("{:?}", error);
            }
        }

        Command::none()
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        let installation_label = Text::new("Installation:").width(Length::Units(140));
        let installation_list = PickList::new(
            &mut self.installation_pick_list,
            Installation::ALL,
            Some(self.selected_installation),
            Interaction::SelectInstallation
        ).width(Length::Fill);
        let installation_row = Row::new()
            .push(installation_label)
            .push(installation_list)
            .width(Length::Fill)
            .align_items(Align::Center)
            .spacing(5)
            .padding(5);


        let minecraft_version_label = Text::new("Minecraft version:").width(Length::Units(140));
        let minecraft_version_list = PickList::new(
            &mut self.minecraft_pick_list,
            Cow::from_iter(self.minecraft_versions.iter().filter(|v| self.show_snapshots || v.stable).cloned()),
            self.selected_minecraft_version.clone(),
            Interaction::SelectMcVersion
        ).width(Length::Fill);
        let enable_snapshots = Checkbox::new(self.show_snapshots, "Show snapshots", Interaction::EnableSnapshots);
        let mc_row = Row::new()
            .push(minecraft_version_label)
            .push(minecraft_version_list)
            .push(enable_snapshots)
            .width(Length::Fill)
            .align_items(Align::Center)
            .spacing(5)
            .padding(5);

            
        let loader_version_label = Text::new("Loader version:").width(Length::Units(140));
        let loader_version_list = PickList::new(
            &mut self.loader_pick_list,
            Cow::from_iter(self.loader_versions.iter().filter(|v| self.show_betas || !v.version.contains("beta")).cloned()),
            self.selected_loader_version.clone(),
            Interaction::SelectLoaderVersion
        ).width(Length::Fill);
        let enable_betas = Checkbox::new(self.show_betas, "Show betas", Interaction::EnableBetas);
        let loader_row = Row::new()
            .push(loader_version_label)
            .push(loader_version_list)
            .push(enable_betas)
            .width(Length::Fill)
            .align_items(Align::Center)
            .spacing(5)
            .padding(5);

        let client_location_label = Text::new("Directory:").width(Length::Units(140));
        let client_location_input = TextInput::new(
            &mut self.client_location_input,
            "Install location",
            self.client_location.to_str().unwrap(),
            |s| Interaction::ChangeClientLocation(PathBuf::from(s))
        ).padding(5);
        let client_loction_browse = Button::new(&mut self.client_location_browse, Text::new("Browse...")).on_press(Interaction::BrowseClientLocation);
        let client_location_row = Row::new()
            .push(client_location_label)
            .push(client_location_input)
            .push(client_loction_browse)
            .width(Length::Fill)
            .align_items(Align::Center)
            .spacing(5)
            .padding(5);
        
        let client_options_label = Text::new("Options:").width(Length::Units(140));
        let create_profile = Checkbox::new(self.generate_profile, "Generate profile", Interaction::GenerateProfile);
        let client_options_row = Row::new()
            .push(client_options_label)
            .push(create_profile)
            .align_items(Align::Center)
            .spacing(5)
            .padding(5);


        let server_location_label = Text::new("Directory:").width(Length::Units(140));
        let server_location_input = TextInput::new(
            &mut self.server_location_input,
            "Install location",
            self.server_location.to_str().unwrap(),
            |s| Interaction::ChangeServerLocation(PathBuf::from(s))
        ).padding(5);
        let server_loction_browse = Button::new(&mut self.server_location_browse, Text::new("Browse...")).on_press(Interaction::BrowseServerLocation);
        let server_location_row = Row::new()
            .push(server_location_label)
            .push(server_location_input)
            .push(server_loction_browse)
            .width(Length::Fill)
            .align_items(Align::Center)
            .spacing(5)
            .padding(5);
        
        let server_options_label = Text::new("Options:").width(Length::Units(140));
        let download_server_jar = Checkbox::new(self.download_server_jar, "Download server jar", Interaction::DownloadServerJar);
        let generate_launch_script = Checkbox::new(self.generate_launch_script, "Generate launch script", Interaction::GenerateLaunchScript);
        let server_options_row = Row::new()
            .push(server_options_label)
            .push(download_server_jar)
            .push(generate_launch_script)
            .align_items(Align::Center)
            .spacing(5)
            .padding(5);


        let mut column = Column::new()
            .padding(5)
            .spacing(5)
            .push(installation_row)
            .push(mc_row)
            .push(loader_row)
            .push(Rule::horizontal(5));

        match self.selected_installation {
            Installation::Client => {
                column = column
                    .push(client_location_row)
                    .push(client_options_row);
            },
            Installation::Server => {
                column = column
                    .push(server_location_row)
                    .push(server_options_row);
            },
        }

        let button_label = Text::new("Install")
            .horizontal_alignment(HorizontalAlignment::Center)
            .width(Length::Fill);
        let mut button = Button::new(&mut self.install_button, button_label)
            .width(Length::Fill);
        if !self.is_installing {
            button = button.on_press(Interaction::Install);
        }
        let progress = ProgressBar::new(0.0..=1.0, self.progress);
        column = column.push(button).push(progress);
        
        let content: Element<Interaction> = column.into();
        content.map(Message::Interaction)
    }
}

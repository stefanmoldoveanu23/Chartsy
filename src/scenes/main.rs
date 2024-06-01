use directories::ProjectDirs;
use iced::widget::image::Handle;
use image::{load_from_memory_with_format, ImageFormat};
use json::JsonValue;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::{fs, io};

use crate::errors::error::Error;
use crate::utils::icons::{Icon, ICON};
use crate::utils::theme;
use crate::widgets::card::Card;
use crate::widgets::modal_stack::ModalStack;
use crate::{database, debug_message, services, LOADING_IMAGE};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, Column, Container, Image, Row, Scrollable, Space, Text};
use iced::{Alignment, Command, Element, Length, Renderer, Theme};
use mongodb::bson::{Bson, Document, Uuid, UuidRepresentation};

use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::scenes::auth::AuthOptions;
use crate::scenes::data::auth::AuthTabIds;
use crate::scenes::scenes::Scenes;

use crate::scenes::data::drawing::SaveMode;
use crate::scenes::drawing::DrawingOptions;
use crate::widgets::closeable::Closeable;
use crate::widgets::tabs::Tabs;

use crate::scenes::data::main::*;

use super::data::posts::PixelImage;

/// The [Messages](SceneMessage) of the main [Scene].
#[derive(Clone)]
pub enum MainMessage {
    /// Opens or closes the given modal.
    ToggleModal(ModalType),

    /// Triggered when the drawings(either online or offline) are loaded.
    LoadedDrawings(Vec<(Uuid, String)>, MainTabIds),

    /// Triggered when a preview image has been loaded.
    LoadedImage(Uuid, Arc<PixelImage>),

    /// Deletes the given drawing.
    DeleteDrawing(Uuid, SaveMode),

    /// Logs out the user from their account.
    LogOut,

    /// Changes the tab for the drawings online/offline tab bar.
    SelectTab(MainTabIds),

    /// Handles errors.
    ErrorHandler(Error),
}

impl Into<Message> for MainMessage {
    fn into(self) -> Message {
        Message::DoAction(Box::new(self))
    }
}

impl SceneMessage for MainMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            Self::ToggleModal { .. } => String::from("Toggle modal"),
            Self::LoadedDrawings(_, _) => String::from("Loaded drawings"),
            Self::LoadedImage(_, _) => String::from("Loaded image"),
            Self::DeleteDrawing(_, _) => String::from("Delete drawing"),
            Self::LogOut => String::from("Logged out"),
            Self::SelectTab(_) => String::from("Select tab"),
            Self::ErrorHandler(_) => String::from("Handle error"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneMessage + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn SceneMessage + 'static>> for Box<MainMessage> {
    fn into(self) -> Box<dyn SceneMessage + 'static> {
        Box::new(*self)
    }
}

/// The main [Scene] of the [Application](crate::Chartsy).
#[derive(Clone)]
pub struct Main {
    /// The modal stack. Used for displaying modals.
    modals: ModalStack<ModalType>,

    /// The list of the users' drawings that are stored online.
    drawings_online: Option<Vec<(Uuid, String)>>,

    /// The list of the users' drawings that are stored offline.
    drawings_offline: Option<Vec<(Uuid, String)>>,

    /// The hashmap of previews.
    previews: HashMap<Uuid, Arc<PixelImage>>,

    /// The id of the active tab on the drawing selection tab bar.
    active_tab: MainTabIds,
}

/// The [Main] scene has no optional data.
#[derive(Debug, Clone, Copy)]
pub struct MainOptions {}

impl Main {
    /// Gets the handle of an image from its id.
    fn get_image(&self, id: Uuid) -> Handle {
        match self.previews.get(&id) {
            Some(pixels) => Handle::from_rgba(
                pixels.get_width(),
                pixels.get_height(),
                pixels.get_data().clone(),
            ),
            None => Handle::from_bytes(LOADING_IMAGE),
        }
    }

    /// Toggles a modal.
    fn toggle_modal(&mut self, modal: &ModalType, globals: &mut Globals) -> Command<Message> {
        self.modals.toggle_modal(modal.clone());

        if modal.clone() == ModalType::ShowingDrawings {
            self.update(globals, &MainMessage::SelectTab(self.active_tab))
        } else {
            Command::none()
        }
    }

    /// Sets the drawings on the given tab.
    fn loaded_drawings(
        &mut self,
        tab: &MainTabIds,
        drawings: &Vec<(Uuid, String)>,
        globals: &mut Globals,
    ) -> Command<Message> {
        let cache = globals.get_cache();

        match tab {
            MainTabIds::Offline => {
                self.drawings_offline = Some(drawings.clone());

                Command::batch(drawings.iter().map(|(id, _)| {
                    let cache = cache.clone();
                    let id = *id;

                    Command::perform(
                        async move {
                            let proj_dirs = match ProjectDirs::from("", "CharMe", "Chartsy") {
                                Some(proj_dirs) => proj_dirs,
                                None => {
                                    return Err(Arc::new(
                                        debug_message!("Could not open local project directory.")
                                            .into(),
                                    ));
                                }
                            };

                            let dir_path = proj_dirs.data_local_dir();
                            let file_path = dir_path.join(id.to_string()).join("data.webp");

                            cache
                                .try_get_with(id, async move {
                                    match tokio::fs::read(file_path).await {
                                        Ok(data) => load_from_memory_with_format(
                                            data.as_slice(),
                                            ImageFormat::WebP,
                                        )
                                        .map(|data| Arc::new(data.into()))
                                        .map_err(|err| debug_message!("{}", err).into()),
                                        Err(err) => Err(debug_message!("{}", err).into()),
                                    }
                                })
                                .await
                                .map(|image| (id, image))
                        },
                        |result: Result<(Uuid, Arc<PixelImage>), Arc<Error>>| match result {
                            Ok((id, image)) => MainMessage::LoadedImage(id, image).into(),
                            Err(err) => Message::Error(err.as_ref().clone()),
                        },
                    )
                }))
            }
            MainTabIds::Online => {
                self.drawings_online = Some(drawings.clone());
                let user_id = globals.get_user().unwrap().get_id();

                Command::batch(drawings.iter().map(|(id, _)| {
                    let cache = cache.clone();
                    let id = *id;

                    Command::perform(
                        async move {
                            cache
                                .try_get_with(id, async move {
                                    match database::base::download_file(format!(
                                        "/{}/{}.webp",
                                        user_id, id
                                    ))
                                    .await
                                    {
                                        Ok(data) => load_from_memory_with_format(
                                            data.as_slice(),
                                            ImageFormat::WebP,
                                        )
                                        .map(|data| Arc::new(data.into()))
                                        .map_err(|err| debug_message!("{}", err).into()),
                                        Err(err) => Err(debug_message!("{}", err).into()),
                                    }
                                })
                                .await
                                .map(|image| (id, image))
                        },
                        |result: Result<(Uuid, Arc<PixelImage>), Arc<Error>>| match result {
                            Ok((id, image)) => MainMessage::LoadedImage(id, image).into(),
                            Err(err) => Message::Error(err.as_ref().clone()),
                        },
                    )
                }))
            }
        }
    }

    /// Logs out the currently authenticated user.
    fn log_out(&mut self, globals: &mut Globals) -> Command<Message> {
        globals.set_user(None);
        self.drawings_online = None;

        Command::perform(
            async {
                let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
                    .ok_or(debug_message!("Unable to find project directory.").into())?;
                let dir_path = proj_dirs.data_local_dir();
                let file_path = dir_path.join("./token");

                tokio::fs::remove_file(file_path)
                    .await
                    .map_err(|err| debug_message!("{}", err).into())
            },
            |result: Result<(), Error>| match result {
                Ok(_) => Message::None,
                Err(err) => Message::Error(err),
            },
        )
    }

    /// Returns the ids of the drawings stored locally.
    async fn get_drawings_offline() -> Result<Vec<(Uuid, String)>, Error> {
        let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy")
            .ok_or(debug_message!("Unable to find project directory.").into())?;

        let dir_path = proj_dirs.data_local_dir();
        tokio::fs::create_dir_all(dir_path)
            .await
            .map_err(|err| debug_message!("{}", err).into())?;

        let file_path = dir_path.join("drawings.json");
        let input = match fs::read_to_string(file_path.clone()) {
            Ok(input) => input,
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    tokio::fs::write(file_path, json::stringify(JsonValue::Array(vec![])))
                        .await
                        .map_err(|err| debug_message!("{}", err).into())?;
                }

                return Ok(vec![]);
            }
        };
        let mut list = vec![];

        let json = json::parse(&*input).map_err(|err| debug_message!("{}", err).into())?;
        if let JsonValue::Array(drawings) = json {
            for drawing in drawings {
                if let JsonValue::Object(drawing) = drawing {
                    let name = if let Some(JsonValue::Short(name)) = drawing.get("name") {
                        name.to_string()
                    } else if let Some(JsonValue::String(name)) = drawing.get("name") {
                        name.clone()
                    } else {
                        String::from("New drawing")
                    };

                    if let Some(JsonValue::String(id)) = drawing.get("id") {
                        if let Ok(id) = Uuid::parse_str(id) {
                            list.push((id, name));
                        }
                    }
                }
            }
        }

        Ok(list)
    }

    /// Returns the ids of the drawings stored in a database that belong to the currently
    /// authenticated user.
    fn get_drawings_online(drawings: &Vec<Document>) -> Vec<(Uuid, String)> {
        let mut list = vec![];
        for document in drawings {
            if let Some(Bson::Binary(bin)) = document.get("id") {
                if let Ok(uuid) = bin.to_uuid_with_representation(UuidRepresentation::Standard) {
                    list.push((
                        uuid,
                        if let Ok(name) = document.get_str("name") {
                            String::from(name)
                        } else {
                            String::from("New drawing")
                        },
                    ));
                }
            }
        }

        list
    }

    /// Switches to the tab of locally stored drawings.
    fn select_offline_tab(&mut self, _globals: &mut Globals) -> Command<Message> {
        if self.drawings_offline.is_none() {
            Command::perform(
                async { Main::get_drawings_offline().await },
                |result| match result {
                    Ok(list) => MainMessage::LoadedDrawings(list, MainTabIds::Offline).into(),
                    Err(err) => Message::Error(err),
                },
            )
        } else {
            Command::none()
        }
    }

    /// Switches to the tab of remotely stored drawings.
    fn select_online_tab(&mut self, globals: &mut Globals) -> Command<Message> {
        if self.drawings_online.is_none() {
            if let (Some(db), Some(user)) = (globals.get_db(), globals.get_user()) {
                let user_id = user.get_id();

                Command::perform(
                    async move { database::main::get_drawings(&db, user_id).await },
                    |result| match result {
                        Ok(ref documents) => MainMessage::LoadedDrawings(
                            Main::get_drawings_online(documents),
                            MainTabIds::Online,
                        )
                        .into(),
                        Err(err) => Message::Error(err),
                    },
                )
            } else {
                Command::none()
            }
        } else {
            Command::none()
        }
    }

    /// Sets the tab to the given value.
    fn select_tab(&mut self, tab_id: &MainTabIds, globals: &mut Globals) -> Command<Message> {
        self.active_tab = tab_id.clone();

        match tab_id {
            MainTabIds::Offline => self.select_offline_tab(globals),
            MainTabIds::Online => self.select_online_tab(globals),
        }
    }
}

impl Scene for Main {
    type Message = MainMessage;
    type Options = MainOptions;

    fn new(options: Option<Self::Options>, _: &mut Globals) -> (Self, Command<Message>)
    where
        Self: Sized,
    {
        let mut main = Main {
            modals: ModalStack::new(),
            drawings_online: None,
            drawings_offline: None,
            previews: HashMap::new(),
            active_tab: MainTabIds::Offline,
        };
        if let Some(options) = options {
            main.apply_options(options);
        }

        (main, Command::none())
    }

    fn get_title(&self) -> String {
        String::from("Main")
    }

    fn apply_options(&mut self, _options: Self::Options) {}

    fn update(&mut self, globals: &mut Globals, message: &Self::Message) -> Command<Message> {
        match message {
            MainMessage::ToggleModal(modal) => self.toggle_modal(&modal, globals),
            MainMessage::LoadedDrawings(drawings, tab) => {
                self.loaded_drawings(&tab, &drawings, globals)
            }
            MainMessage::LoadedImage(id, image) => {
                self.previews.insert(*id, image.clone());
                Command::none()
            }
            MainMessage::DeleteDrawing(id, save_mode) => {
                let globals = globals.clone();

                match save_mode {
                    SaveMode::Offline => {
                        self.drawings_offline
                            .as_mut()
                            .unwrap()
                            .retain(|(drawing_id, _)| *drawing_id != *id);
                    }
                    SaveMode::Online => {
                        self.drawings_online
                            .as_mut()
                            .unwrap()
                            .retain(|(drawing_id, _)| *drawing_id != *id);
                    }
                }

                let id = *id;
                let save_mode = save_mode.clone();

                Command::perform(
                    async move {
                        match save_mode {
                            SaveMode::Offline => {
                                services::drawings::delete_drawing_offline(id).await
                            }
                            SaveMode::Online => {
                                services::drawings::delete_drawing_online(id, &globals).await
                            }
                        }
                    },
                    |result| match result {
                        Ok(_) => Message::None,
                        Err(err) => Message::Error(err),
                    },
                )
            }
            MainMessage::LogOut => self.log_out(globals),
            MainMessage::SelectTab(tab_id) => self.select_tab(&tab_id, globals),
            MainMessage::ErrorHandler(_) => Command::none(),
        }
    }

    fn view(&self, globals: &Globals) -> Element<Message, Theme, Renderer> {
        let container_auth = if let Some(user) = globals.get_user() {
            let welcome_message = Text::new(format!("Welcome, {}!", user.get_username()))
                .vertical_alignment(Vertical::Bottom);
            let settings_button = Button::new("Settings")
                .padding(8)
                .on_press(Message::ChangeScene(Scenes::Settings(None)));
            let logout_button = Button::new("Log Out")
                .padding(8)
                .on_press(MainMessage::LogOut.into());

            Row::with_children(vec![
                Space::with_width(Length::Fill).into(),
                Row::with_children(vec![
                    welcome_message.into(),
                    settings_button.into(),
                    logout_button.into(),
                ])
                .align_items(Alignment::Center)
                .width(Length::Shrink)
                .spacing(20)
                .into(),
            ])
        } else {
            let register_button =
                Button::new("Register")
                    .padding(8)
                    .on_press(Message::ChangeScene(Scenes::Auth(Some(AuthOptions::new(
                        AuthTabIds::Register,
                    )))));
            let login_button = Button::new("Log In")
                .padding(8)
                .on_press(Message::ChangeScene(Scenes::Auth(Some(AuthOptions::new(
                    AuthTabIds::LogIn,
                )))));

            Row::with_children(vec![
                Space::with_width(Length::Fill).into(),
                Row::with_children(vec![register_button.into(), login_button.into()])
                    .width(Length::Shrink)
                    .spacing(10)
                    .into(),
            ])
        };

        let title = Container::new(Text::new("Chartsy").width(Length::Shrink).size(50))
            .height(Length::FillPortion(2))
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);

        let start_drawing_button = Button::new("Start new Drawing")
            .padding(8)
            .on_press(MainMessage::ToggleModal(ModalType::SelectingSaveMode).into());
        let continue_drawing_button = Button::new("Continue drawing")
            .padding(8)
            .on_press(MainMessage::ToggleModal(ModalType::ShowingDrawings).into());
        let browse_posts_button = Button::new("Browse posts")
            .padding(8)
            .on_press(Message::ChangeScene(Scenes::Posts(None)));
        let quit_button = Button::new("Quit").padding(8).on_press(Message::Quit);

        let column_buttons = Column::with_children(
            if globals.get_db().is_some() && globals.get_user().is_some() {
                vec![
                    start_drawing_button.into(),
                    continue_drawing_button.into(),
                    browse_posts_button.into(),
                    quit_button.into(),
                ]
            } else {
                vec![
                    start_drawing_button.into(),
                    continue_drawing_button.into(),
                    quit_button.into(),
                ]
            },
        )
        .spacing(20)
        .height(Length::FillPortion(3))
        .width(Length::Fill)
        .align_items(Alignment::Center);

        let container_entrance: Container<Message, Theme, Renderer> = Container::new(
            Column::with_children(vec![
                container_auth.into(),
                title.into(),
                column_buttons.into(),
            ])
            .spacing(20)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center),
        );

        let modal_generator = |modal_type: ModalType| match modal_type {
            ModalType::ShowingDrawings => {
                let display_drawing = move |id, name: String, save_mode| {
                    Button::new(
                        Row::<Message, Theme, Renderer>::with_children(vec![
                            Text::new(name.clone())
                                .width(Length::FillPortion(1))
                                .horizontal_alignment(Horizontal::Center)
                                .into(),
                            Space::with_width(Length::FillPortion(1)).into(),
                            Image::new(self.get_image(id))
                                .width(Length::FillPortion(1))
                                .height(Length::Fixed(150.0))
                                .into(),
                            Button::new(
                                Text::new(Icon::Trash.to_string())
                                    .font(ICON)
                                    .style(theme::text::danger),
                            )
                            .style(iced::widget::button::text)
                            .on_press(MainMessage::DeleteDrawing(id, save_mode).into())
                            .into(),
                        ])
                        .align_items(Alignment::Center),
                    )
                    .on_press(Message::ChangeScene(Scenes::Drawing(Some(
                        DrawingOptions::new(Some(id), Some(name), Some(save_mode)),
                    ))))
                    .style(iced::widget::button::primary)
                    .width(Length::Fill)
                    .padding(10.0)
                    .into()
                };

                let online_tab = Container::new(Scrollable::new(
                    Column::<Message, Theme, Renderer>::with_children(
                        if let Some(drawings) = self.drawings_online.clone() {
                            drawings
                                .clone()
                                .iter()
                                .map(|(uuid, name)| {
                                    display_drawing(*uuid, name.clone(), SaveMode::Online)
                                })
                                .collect()
                        } else {
                            vec![]
                        },
                    )
                    .spacing(20.0),
                ))
                .width(Length::Fixed(500.0))
                .height(Length::Fixed(300.0))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Top);

                let offline_tab = Container::new(Scrollable::new(
                    Column::<Message, Theme, Renderer>::with_children(
                        if let Some(drawings) = self.drawings_offline.clone() {
                            drawings
                                .clone()
                                .iter()
                                .map(|(uuid, name)| {
                                    display_drawing(*uuid, name.clone(), SaveMode::Offline)
                                })
                                .collect()
                        } else {
                            vec![]
                        },
                    )
                    .spacing(20.0),
                ))
                .width(Length::Fixed(500.0))
                .height(Length::Fixed(300.0))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Top);

                let title = Text::new("Your drawings")
                    .horizontal_alignment(Horizontal::Center)
                    .size(25);
                let tabs = Tabs::new_with_tabs(
                    vec![
                        (
                            MainTabIds::Offline,
                            Text::new("Offline").into(),
                            offline_tab.into(),
                        ),
                        (
                            MainTabIds::Online,
                            Text::new("Online").into(),
                            online_tab.into(),
                        ),
                    ],
                    |tab| MainMessage::SelectTab(tab).into(),
                )
                .selected(self.active_tab)
                .height(Length::Fixed(600.0))
                .width(Length::Fill);

                Closeable::<Message, Theme, Renderer>::new(
                    Card::new(title, tabs)
                        .width(Length::Fixed(900.0))
                        .height(Length::Fixed(600.0)),
                )
                .style(theme::closeable::Closeable::Transparent)
                .on_close(
                    Into::<Message>::into(MainMessage::ToggleModal(ModalType::ShowingDrawings)),
                    32.0,
                )
                .into()
            }
            ModalType::SelectingSaveMode => {
                let offline_button = Button::new("Offline")
                    .padding(8)
                    .width(Length::FillPortion(1))
                    .on_press(Message::ChangeScene(Scenes::Drawing(Some(
                        DrawingOptions::new(None, None, Some(SaveMode::Offline)),
                    ))));
                let online_button = if globals.get_db().is_some() && globals.get_user().is_some() {
                    Button::new("Online").on_press(Message::ChangeScene(Scenes::Drawing(Some(
                        DrawingOptions::new(None, None, Some(SaveMode::Online)),
                    ))))
                } else {
                    Button::new("Online")
                }
                .padding(8)
                .width(Length::FillPortion(1));

                Closeable::<Message, Theme, Renderer>::new(
                    Card::new(
                        Text::new("Create new drawing"),
                        Column::with_children(vec![
                            Space::with_height(Length::Fill).into(),
                            Row::with_children(vec![
                                offline_button.into(),
                                Space::with_width(Length::FillPortion(2)).into(),
                                online_button.into(),
                            ])
                            .into(),
                        ])
                        .height(Length::Fixed(150.0)),
                    )
                    .width(Length::Fixed(300.0)),
                )
                .on_close(
                    Into::<Message>::into(MainMessage::ToggleModal(ModalType::SelectingSaveMode)),
                    25.0,
                )
                .into()
            }
        };

        self.modals.get_modal(container_entrance, modal_generator)
    }

    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message> {
        self.update(globals, &MainMessage::ErrorHandler(error.clone()))
    }

    fn clear(&self, _globals: &mut Globals) {}
}

use std::any::Any;

use crate::database;
use crate::utils::errors::Error;
use crate::widgets::ModalStack;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, Column, Container, Text};
use iced::{Alignment, Command, Element, Length, Renderer, Theme};
use mongodb::bson::Uuid;

use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::scenes::scenes::Scenes;
use crate::scenes::services;

use crate::scenes::data::drawing::SaveMode;
use crate::scenes::drawing::DrawingOptions;

use crate::scenes::data::main::*;

/// The [Messages](SceneMessage) of the main [Scene].
#[derive(Clone)]
pub enum MainMessage {
    /// Opens or closes the given modal.
    ToggleModal(ModalType),

    /// Triggered when the drawings(either online or offline) are loaded.
    LoadedDrawings(Vec<(Uuid, String)>, MainTabIds),

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

    /// The id of the active tab on the drawing selection tab bar.
    active_tab: MainTabIds,
}

/// The [Main] scene has no optional data.
#[derive(Debug, Clone, Copy)]
pub struct MainOptions {}

impl Main {
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
    ) -> Command<Message> {
        match tab {
            MainTabIds::Offline => {
                self.drawings_offline = Some(drawings.clone());
            }
            MainTabIds::Online => {
                self.drawings_online = Some(drawings.clone());
            }
        }

        let tab = *tab;

        Command::perform(async {}, move |_| MainMessage::SelectTab(tab).into())
    }

    pub fn load_previews(&self, globals: &Globals) -> Command<Message> {
        let commands_offline = self
            .drawings_offline
            .clone()
            .map_or(Command::none(), |drawings| {
                globals.get_cache().insert_if_not(
                    drawings.iter().map(|(id, _)| *id),
                    std::convert::identity,
                    services::main::load_preview_offline,
                )
            });

        let commands_online = self
            .drawings_online
            .clone()
            .map_or(Command::none(), |drawings| {
                let user_id = globals.get_user().unwrap().get_id();

                globals.get_cache().insert_if_not(
                    drawings.iter().map(|(id, _)| (*id, user_id)),
                    |(id, _)| id,
                    services::main::load_preview_online,
                )
            });

        Command::batch(vec![commands_offline, commands_online])
    }

    /// Logs out the currently authenticated user.
    fn log_out(&mut self, globals: &mut Globals) -> Command<Message> {
        globals.set_user(None);
        self.drawings_online = None;

        Command::perform(
            async { services::main::delete_token_file().await },
            |result: Result<(), Error>| match result {
                Ok(_) => Message::None,
                Err(err) => Message::Error(err),
            },
        )
    }

    /// Switches to the tab of locally stored drawings.
    fn select_offline_tab(&mut self, _globals: &mut Globals) -> Command<Message> {
        if self.drawings_offline.is_none() {
            Command::perform(
                async { services::main::get_drawings_offline().await },
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
                            services::main::get_drawings_online(documents),
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
            MainMessage::LoadedDrawings(drawings, tab) => self.loaded_drawings(&tab, &drawings),
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
                                services::drawing::delete_drawing_offline(id).await
                            }
                            SaveMode::Online => {
                                services::drawing::delete_drawing_online(id, &globals).await
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
            services::main::auth_logged_in(&user)
        } else {
            services::main::auth_logged_out()
        };

        let title = Container::new(Text::new("Chartsy").width(Length::Shrink).size(50))
            .height(Length::FillPortion(2))
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);

        let column_buttons =
            services::main::main_column(globals.get_db().is_some() && globals.get_user().is_some());

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
                let online_tab =
                    services::main::drawings_tab(&self.drawings_online, SaveMode::Online, globals);

                let offline_tab = services::main::drawings_tab(
                    &self.drawings_offline,
                    SaveMode::Offline,
                    globals,
                );

                let title = Text::new("Your drawings")
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill)
                    .size(25)
                    .into();
                let tabs = services::main::drawings_tabs(offline_tab, online_tab, self.active_tab);

                services::main::display_drawings(title, tabs)
            }
            ModalType::SelectingSaveMode => {
                let offline_button = Button::new("Offline")
                    .padding(8)
                    .width(Length::FillPortion(1))
                    .on_press(Message::ChangeScene(Scenes::Drawing(Some(
                        DrawingOptions::new(None, None, Some(SaveMode::Offline)),
                    ))))
                    .into();

                let online_button = if globals.get_db().is_some() && globals.get_user().is_some() {
                    Button::new("Online").on_press(Message::ChangeScene(Scenes::Drawing(Some(
                        DrawingOptions::new(None, None, Some(SaveMode::Online)),
                    ))))
                } else {
                    Button::new("Online")
                }
                .padding(8)
                .width(Length::FillPortion(1))
                .into();

                services::main::create_drawing(offline_button, online_button)
            }
        };

        self.modals.get_modal(container_entrance, modal_generator)
    }

    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message> {
        self.update(globals, &MainMessage::ErrorHandler(error.clone()))
    }

    fn clear(&self, _globals: &mut Globals) {}
}

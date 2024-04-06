use std::any::Any;
use std::fs;
use directories::ProjectDirs;

use crate::errors::error::Error;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Container, Scrollable, Space, Row, Button, Text};
use iced::{Alignment, Command, Element, Length, Renderer};
use iced_aw::{Tabs, TabLabel};
use mongodb::bson::{Bson, Uuid, UuidRepresentation, Document};
use crate::icons::{Icon, ICON};
use crate::mongo;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::card::Card;

use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::auth::AuthOptions;
use crate::scenes::data::auth::AuthTabIds;
use crate::scenes::scenes::Scenes;

use crate::scenes::drawing::DrawingOptions;
use crate::scenes::data::drawing::SaveMode;
use crate::theme::Theme;
use crate::widgets::closeable::Closeable;

use crate::scenes::data::main::*;

/// The [Messages](Action) of the main [Scene].
#[derive(Clone)]
enum MainAction {
    /// Opens or closes the given modal.
    ToggleModal(ModalType),

    /// Triggered when the drawings(either online or offline) are loaded.
    LoadedDrawings(Vec<Uuid>, MainTabIds),

    /// Logs out the user from their account.
    LogOut,

    /// Changes the tab for the drawings online/offline tab bar.
    SelectTab(MainTabIds),

    /// Handles errors.
    ErrorHandler(Error),
}

impl Action for MainAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            MainAction::ToggleModal {..} => String::from("Toggle modal"),
            MainAction::LoadedDrawings(_, _) => String::from("Loaded drawings"),
            MainAction::LogOut => String::from("Logged out"),
            MainAction::SelectTab(_) => String::from("Select tab"),
            MainAction::ErrorHandler(_) => String::from("Handle error"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn Action + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn Action + 'static>> for Box<MainAction> {
    fn into(self) -> Box<dyn Action + 'static> {
        Box::new(*self)
    }
}

/// The main [Scene] of the [Application](crate::Chartsy).
#[derive(Clone)]
pub struct Main {
    /// The modal stack. Used for displaying modals.
    modals: ModalStack<ModalType>,

    /// The list of the users' drawings that are stored online.
    drawings_online: Option<Vec<Uuid>>,

    /// The list of the users' drawings that are stored offline.
    drawings_offline: Option<Vec<Uuid>>,

    /// The id of the active tab on the drawing selection tab bar.
    active_tab: MainTabIds,
}

/// The [Main] scene has no optional data.
#[derive(Debug, Clone, Copy)]
pub struct MainOptions {}

impl SceneOptions<Main> for MainOptions {
    fn apply_options(&self, _scene: &mut Main) {}

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Main>> {
        Box::new((*self).clone())
    }
}

impl Main {
    /// Toggles a modal.
    fn toggle_modal(&mut self, modal: &ModalType, globals: &mut Globals) -> Command<Message>
    {
        self.modals.toggle_modal(modal.clone());

        if modal.clone() == ModalType::ShowingDrawings {
            self.update(globals, Box::new(MainAction::SelectTab(self.active_tab)))
        } else {
            Command::none()
        }
    }

    /// Sets the drawings on the given tab.
    fn loaded_drawings(&mut self, tab: &MainTabIds, drawings: &Vec<Uuid>, _globals: &mut Globals
    ) -> Command<Message> {
        match tab {
            MainTabIds::Offline => {
                self.drawings_offline = Some(drawings.clone());
            }
            MainTabIds::Online => {
                self.drawings_online = Some(drawings.clone());
            }
        }

        Command::none()
    }

    /// Logs out the currently authenticated user.
    fn log_out(&mut self, globals: &mut Globals) -> Command<Message>
    {
        globals.set_user(None);
        self.drawings_online = None;

        let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
        let dir_path = proj_dirs.data_local_dir();
        let file_path = dir_path.join("./token");

        fs::remove_file(file_path).unwrap();

        Command::none()
    }

    /// Returns the ids of the drawings stored locally.
    async fn get_drawings_offline() -> Vec<Uuid>
    {
        let proj_dirs = ProjectDirs::from(
            "",
            "CharMe",
            "Chartsy"
        ).unwrap();

        let dir_path = proj_dirs.data_local_dir();
        fs::create_dir_all(dir_path).unwrap();
        let mut list = vec![];

        for entry in fs::read_dir(dir_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                if let Ok(uuid) = Uuid::parse_str(path.iter().last().unwrap().to_str().unwrap()) {
                    list.push(uuid);
                }
            }
        }

        list
    }

    /// Returns the ids of the drawings stored in a database that belong to the currently
    /// authenticated user.
    fn get_drawings_online(drawings: &Vec<Document>) -> Vec<Uuid>
    {
        let mut list = vec![];
        for document in drawings {
            if let Some(Bson::Binary(bin)) = document.get("id") {
                if let Ok(uuid) =
                    bin.to_uuid_with_representation(UuidRepresentation::Standard)
                {
                    list.push(uuid);
                }
            }
        }

        list
    }

    /// Switches to the tab of locally stored drawings.
    fn select_offline_tab(&mut self, _globals: &mut Globals) -> Command<Message>
    {
        if self.drawings_offline.is_none() {
            Command::perform(
                async {
                    Main::get_drawings_offline().await
                },
                |list| Message::DoAction(Box::new(
                    MainAction::LoadedDrawings(list, MainTabIds::Offline)
                ))
            )
        } else {
            Command::none()
        }
    }

    /// Switches to the tab of remotely stored drawings.
    fn select_online_tab(&mut self, globals: &mut Globals) -> Command<Message>
    {
        if self.drawings_online.is_none() {
            if let (Some(db), Some(user)) = (globals.get_db(), globals.get_user()) {
                let user_id = user.get_id();

                Command::perform(
                    async move {
                        mongo::main::get_drawings(&db, user_id).await
                    },
                    |result| {
                        match result {
                            Ok(ref documents) => {
                                Message::DoAction(Box::new(MainAction::LoadedDrawings(
                                    Main::get_drawings_online(documents),
                                    MainTabIds::Online
                                )))
                            }
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            } else {
                Command::none()
            }
        } else {
            Command::none()
        }
    }

    /// Sets the tab to the given value.
    fn select_tab(&mut self, tab_id: &MainTabIds, globals: &mut Globals) -> Command<Message>
    {
        self.active_tab = tab_id.clone();

        match tab_id {
            MainTabIds::Offline => {
                self.select_offline_tab(globals)
            }
            MainTabIds::Online => {
                self.select_online_tab(globals)
            }
        }
    }
}

impl Scene for Main {
    fn new(
        options: Option<Box<dyn SceneOptions<Main>>>,
        _: &mut Globals,
    ) -> (Self, Command<Message>)
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
            options.apply_options(&mut main);
        }

        (
            main,
            Command::none(),
        )
    }

    fn get_title(&self) -> String {
        String::from("Main")
    }

    fn update(&mut self, globals: &mut Globals, message: Box<dyn Action>) -> Command<Message> {
        let message: &MainAction = message
            .as_any()
            .downcast_ref::<MainAction>()
            .expect("Panic downcasting to MainAction");

        match message {
            MainAction::ToggleModal(modal) => {
                self.toggle_modal(modal, globals)
            }
            MainAction::LoadedDrawings(drawings, tab) => {
                self.loaded_drawings(tab, drawings, globals)
            }
            MainAction::LogOut => {
                self.log_out(globals)
            }
            MainAction::SelectTab(tab_id) => {
                self.select_tab(tab_id, globals)
            }
            MainAction::ErrorHandler(_) => Command::none(),
        }
    }

    fn view(&self, globals: &Globals) -> Element<Message, Theme, Renderer> {
        let container_auth: Element<Message, Theme, Renderer> =
            if let Some(user) = globals.get_user() {
                Row::with_children(vec![
                    Space::with_width(Length::Fill).into(),
                    Row::with_children(vec![
                        Text::new(format!("Welcome, {}!", user.get_username()))
                            .vertical_alignment(Vertical::Bottom)
                            .into(),
                        Button::new("Log Out")
                            .padding(8)
                            .on_press(Message::DoAction(Box::new(MainAction::LogOut)))
                            .into(),
                    ])
                        .align_items(Alignment::Center)
                        .width(Length::Shrink)
                        .spacing(20)
                        .into()
                ])
                .into()
            } else {
                Row::with_children(vec![
                    Space::with_width(Length::Fill).into(),
                    Row::with_children(vec![
                        Button::new("Register")
                            .padding(8)
                            .on_press(Message::ChangeScene(Scenes::Auth(Some(Box::new(
                                AuthOptions::new(AuthTabIds::Register)
                            )))))
                            .into(),
                        Button::new("Log In")
                            .padding(8)
                            .on_press(Message::ChangeScene(Scenes::Auth(Some(Box::new(
                                AuthOptions::new(AuthTabIds::LogIn)
                            )))))
                            .into(),
                    ])
                        .width(Length::Shrink)
                        .spacing(10)
                        .into()
                ])
                .into()
            };

        let container_entrance: Container<Message, Theme, Renderer> = Container::new(
            Column::with_children(vec![
                container_auth,
                Column::with_children(vec![
                    Text::new("Chartsy")
                        .width(Length::Shrink)
                        .size(50)
                        .into()
                ])
                    .height(Length::FillPortion(2))
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .into(),
                Column::with_children(vec![
                    Button::new("Start new Drawing")
                        .padding(8)
                        .on_press(Message::DoAction(Box::new(MainAction::ToggleModal(ModalType::SelectingSaveMode))))
                        .into(),
                    Button::new("Continue drawing")
                        .padding(8)
                        .on_press(Message::DoAction(Box::new(MainAction::ToggleModal(ModalType::ShowingDrawings))))
                        .into(),
                    if globals.get_db().is_some() && globals.get_user().is_some() {
                        Element::<Message, Theme, Renderer>::from(
                            Button::new("Browse posts")
                                .padding(8)
                                .on_press(Message::ChangeScene(Scenes::Posts(None)))
                        )
                    } else {
                        Space::with_height(Length::Shrink)
                            .into()
                    }
                ])
                    .spacing(20)
                    .height(Length::FillPortion(3))
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .into()
            ])
                .spacing(20)
                .padding(20)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_items(Alignment::Center)
        );

        let modal_generator = |modal_type: ModalType| {
            match modal_type {
                ModalType::ShowingDrawings => {
                    let online_tab = Container::new(Scrollable::new(Column::<Message, Theme, Renderer>::with_children(
                        if let Some(drawings) = self.drawings_online.clone() {
                            drawings
                                .clone()
                                .iter()
                                .map(|uuid| {
                                    Element::from(Button::new(Text::new(uuid.to_string())).on_press(Message::ChangeScene(
                                        Scenes::Drawing(Some(Box::new(DrawingOptions::new(
                                            Some(uuid.clone()),
                                            Some(SaveMode::Online),
                                        )))),
                                    )))
                                })
                                .collect()
                        } else {
                            vec![]
                        },
                    )))
                        .width(Length::Fixed(500.0))
                        .height(Length::Fixed(300.0))
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Top);

                    let offline_tab = Container::new(Scrollable::new(Column::<Message, Theme, Renderer>::with_children(
                        if let Some(drawings) = self.drawings_offline.clone() {
                            drawings
                                .clone()
                                .iter()
                                .map(|uuid| {
                                    Element::from(Button::new(Text::new(uuid.to_string())).on_press(Message::ChangeScene(
                                        Scenes::Drawing(Some(Box::new(DrawingOptions::new(
                                            Some(uuid.clone()),
                                            Some(SaveMode::Offline),
                                        )))),
                                    )))
                                })
                                .collect()
                        } else {
                            vec![]
                        },
                    )))
                        .width(Length::Fixed(500.0))
                        .height(Length::Fixed(300.0))
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Top);

                    Closeable::<Message, Theme, Renderer>::new(
                        Card::new(
                            Text::new("Your drawings")
                                .horizontal_alignment(Horizontal::Center)
                                .size(25),
                            Tabs::new_with_tabs(
                                vec![
                                    (
                                        MainTabIds::Offline,
                                        TabLabel::Text("Offline".into()),
                                        offline_tab.into()
                                    ),
                                    (
                                        MainTabIds::Online,
                                        TabLabel::Text("Online".into()),
                                        online_tab.into()
                                    )
                                ],
                                |tab| Message::DoAction(Box::new(MainAction::SelectTab(tab)))
                            )
                                .set_active_tab(&self.active_tab)
                                .width(Length::Fill)
                                .height(Length::Fixed(300.0))
                        )
                            .width(Length::Fixed(500.0))
                            .height(Length::Fixed(300.0))
                            .footer(Text::new(Icon::X.to_string()).font(ICON).size(30.0))
                    )
                        .style(crate::theme::closeable::Closeable::Transparent)
                        .on_close(
                            Message::DoAction(Box::new(MainAction::ToggleModal(ModalType::ShowingDrawings))),
                            32.0
                        )
                        .into()
                }
                ModalType::SelectingSaveMode => {
                    Container::<Message, Theme, Renderer>::new(
                        Card::new(
                            Text::new("Create new drawing"),
                            Column::with_children(vec![
                                Space::with_height(Length::Fill).into(),
                                Row::with_children(vec![
                                    Button::new("Offline")
                                        .padding(8)
                                        .width(Length::FillPortion(1))
                                        .on_press(Message::ChangeScene(Scenes::Drawing(Some(Box::new(DrawingOptions::new(None, Some(SaveMode::Offline)))))))
                                        .into(),
                                    Space::with_width(Length::FillPortion(2)).into(),
                                    if globals.get_db().is_some() && globals.get_user().is_some() {
                                        Button::new("Online")
                                            .padding(8)
                                            .width(Length::FillPortion(1))
                                            .on_press(Message::ChangeScene(Scenes::Drawing(Some(Box::new(DrawingOptions::new(None, Some(SaveMode::Online)))))))
                                            .into()
                                    } else {
                                        Button::new("Online")
                                            .padding(8)
                                            .width(Length::FillPortion(1))
                                            .into()
                                    }
                                ])
                                    .into()
                            ])
                                .height(Length::Fixed(150.0))
                        )
                            .width(Length::Fixed(300.0))
                            //.on_close(Message::DoAction(Box::new(MainAction::ToggleModal(ModalType::SelectingSaveMode))))
                    )
                        .into()
                }
            }
        };

        self.modals.get_modal(container_entrance.into(), modal_generator)
    }

    fn get_error_handler(&self, error: Error) -> Box<dyn Action> {
        Box::new(MainAction::ErrorHandler(error))
    }

    fn clear(&self) {}
}

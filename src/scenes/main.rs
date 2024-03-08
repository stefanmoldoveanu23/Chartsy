use std::any::Any;
use std::fs;
use directories::ProjectDirs;

use crate::errors::error::Error;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, horizontal_space, vertical_space, row, text, Column, Container, Scrollable};
use iced::{Alignment, Command, Element, Length, Renderer};
use iced::advanced::widget::Text;
use iced_aw::{Card, Tabs, TabLabel};
use mongodb::bson::{doc, Bson, Uuid, UuidRepresentation};
use crate::widgets::modal_stack::ModalStack;

use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::auth::{AuthOptions, AuthTabIds};
use crate::scenes::scenes::Scenes;

use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::scenes::drawing::{DrawingOptions, SaveMode};
use crate::theme::Theme;

#[derive(Clone, Eq, PartialEq)]
enum ModalType {
    ShowingDrawings,
    SelectingSaveMode,
}

/// The [Messages](Action) of the main [Scene]:
/// - [ToggleModal](MainAction::ToggleModal), which opens or closes the given overlay;
/// - [LoadedDrawings](MainAction::LoadedDrawings), which receives the list of drawings from
/// the [Database](mongodb::Database), or locally;
/// - [LogOut](MainAction::LogOut), which logs the user out of their account;
/// - [SelectTab](MainAction::SelectTab), for the drawings overlay tabs;
/// - [ErrorHandler(Error)](MainAction::ErrorHandler), which handles errors.
#[derive(Clone)]
enum MainAction {
    None,
    ToggleModal(ModalType),
    LoadedDrawings(Vec<Uuid>, MainTabIds),
    LogOut,
    SelectTab(MainTabIds),
    ErrorHandler(Error),
}

impl Action for MainAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            MainAction::None => String::from("None"),
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
///
/// Allows the user to create a new [Drawing](crate::scenes::drawing::Drawing) or to open an already
/// existing one.
#[derive(Clone)]
pub struct Main {
    modals: ModalStack<ModalType>,
    drawings_online: Option<Vec<Uuid>>,
    drawings_offline: Option<Vec<Uuid>>,
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
                self.modals.toggle_modal(modal.clone());

                if modal.clone() == ModalType::ShowingDrawings {
                    return self.update(globals, Box::new(MainAction::SelectTab(self.active_tab)));
                }
            }
            MainAction::LoadedDrawings(drawings, tab) => {
                match tab {
                    MainTabIds::Offline => {
                        self.drawings_offline = Some(drawings.clone());
                    }
                    MainTabIds::Online => {
                        self.drawings_online = Some(drawings.clone());
                    }
                }
            }
            MainAction::LogOut => {
                globals.set_user(None);
                self.drawings_online = None;

                let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
                let dir_path = proj_dirs.data_local_dir();
                let file_path = dir_path.join("./token");

                fs::remove_file(file_path).unwrap();
            }
            MainAction::SelectTab(tab_id) => {
                self.active_tab = tab_id.clone();

                match tab_id {
                    MainTabIds::Offline => {
                        if self.drawings_online.is_none() {
                            return Command::perform(
                                async {
                                    let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
                                    let dir_path = proj_dirs.data_local_dir();
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
                                },
                                |list| Message::DoAction(Box::new(MainAction::LoadedDrawings(list, MainTabIds::Offline)))
                            )
                        }
                    }
                    MainTabIds::Online => {
                        if self.drawings_online.is_none() {

                            if let (Some(db), Some(user)) = (globals.get_db(), globals.get_user()) {
                                let user_id = user.get_id();
                                
                                return Command::perform(
                                    async move {
                                        MongoRequest::send_requests(
                                            db,
                                            vec![MongoRequest::new(
                                                "canvases".into(),
                                                MongoRequestType::Get(doc! {"user_id": user_id}),
                                            )]
                                        ).await
                                    },
                                    |res| {
                                        match res {
                                            Ok(res) => {
                                                if let Some(MongoResponse::Get(cursor)) = res.get(0) {
                                                    let mut list = vec![];
                                                    for document in cursor {
                                                        if let Some(Bson::Binary(bin)) = document.get("id") {
                                                            if let Ok(uuid) =
                                                                bin.to_uuid_with_representation(UuidRepresentation::Standard)
                                                            {
                                                                list.push(uuid);
                                                            }
                                                        }
                                                    }
                                                    Message::DoAction(Box::new(MainAction::LoadedDrawings(list, MainTabIds::Online)))
                                                } else {
                                                    Message::DoAction(Box::new(MainAction::None))
                                                }
                                            }
                                            Err(message) => message
                                        }
                                    }
                                );
                            }
                        }
                    }
                }
            }
            MainAction::ErrorHandler(_) => {}
            MainAction::None => {}
        }

        Command::none()
    }

    fn view(&self, globals: &Globals) -> Element<Message, Renderer<Theme>> {
        let container_auth: Element<Message, Renderer<Theme>> =
            if let Some(user) = globals.get_user() {
                row![
                    horizontal_space(Length::Fill),
                    row![
                        text(format!("Welcome, {}!", user.get_username()))
                            .vertical_alignment(Vertical::Bottom),
                        button("Log Out")
                            .padding(8)
                            .on_press(Message::DoAction(Box::new(MainAction::LogOut))),
                    ]
                    .align_items(Alignment::Center)
                    .width(Length::Shrink)
                    .spacing(20)
                ]
                .into()
            } else {
                row![
                    horizontal_space(Length::Fill),
                    row![
                        button("Register")
                            .padding(8)
                            .on_press(Message::ChangeScene(Scenes::Auth(Some(Box::new(
                                AuthOptions::new(AuthTabIds::Register)
                            ))))),
                        button("Log In")
                            .padding(8)
                            .on_press(Message::ChangeScene(Scenes::Auth(Some(Box::new(
                                AuthOptions::new(AuthTabIds::LogIn)
                            ))))),
                    ]
                    .width(Length::Shrink)
                    .spacing(10),
                ]
                .into()
            };

        let container_entrance: Container<Message, Renderer<Theme>> = Container::new(
            column![
                container_auth,
                column![text("Chartsy").width(Length::Shrink).size(50)]
                    .height(Length::FillPortion(2))
                    .width(Length::Fill)
                    .align_items(Alignment::Center),
                column![
                    button("Start new Drawing")
                        .padding(8)
                        .on_press(Message::DoAction(Box::new(MainAction::ToggleModal(ModalType::SelectingSaveMode)))),
                    button("Continue drawing")
                        .padding(8)
                        .on_press(Message::DoAction(Box::new(MainAction::ToggleModal(ModalType::ShowingDrawings)))),
                    if globals.get_db().is_some() && globals.get_user().is_some() {
                        Element::<Message, Renderer<Theme>>::from(
                            button("Browse posts")
                                .padding(8)
                                .on_press(Message::ChangeScene(Scenes::Posts(None)))
                        )
                    } else {
                        vertical_space(Length::Shrink)
                            .into()
                    }
                ]
                .spacing(20)
                .height(Length::FillPortion(3))
                .width(Length::Fill)
                .align_items(Alignment::Center),
            ]
            .spacing(20)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center),
        );

        let modal_generator = |modal_type: ModalType| {
            match modal_type {
                ModalType::ShowingDrawings => {
                    let online_tab = Container::new(Scrollable::new(Column::<Message, Renderer<Theme>>::with_children(
                        if let Some(drawings) = self.drawings_online.clone() {
                            drawings
                                .clone()
                                .iter()
                                .map(|uuid| {
                                    Element::from(button(text(uuid)).on_press(Message::ChangeScene(
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

                    let offline_tab = Container::new(Scrollable::new(Column::<Message, Renderer<Theme>>::with_children(
                        if let Some(drawings) = self.drawings_offline.clone() {
                            drawings
                                .clone()
                                .iter()
                                .map(|uuid| {
                                    Element::from(button(text(uuid)).on_press(Message::ChangeScene(
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

                    Container::<Message, Renderer<Theme>>::new(
                        Card::new::<Text<Renderer<Theme>>, Element<Message, Renderer<Theme>>>(
                            text("Your drawings")
                                .horizontal_alignment(Horizontal::Center)
                                .size(25),
                            Tabs::with_tabs(
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
                                .into()
                        )
                            .width(Length::Fixed(500.0))
                            .height(Length::Fixed(300.0))
                            .on_close(Message::DoAction(Box::new(MainAction::ToggleModal(ModalType::ShowingDrawings)))),
                    )
                        .padding(10)
                        .height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center)
                        .into()
                }
                ModalType::SelectingSaveMode => {
                    Container::<Message, Renderer<Theme>>::new(
                        Card::new(
                            text("Create new drawing"),
                            column![
                                vertical_space(Length::Fill),
                                row![
                                    button("Offline")
                                        .padding(8)
                                        .width(Length::FillPortion(1))
                                        .on_press(Message::ChangeScene(Scenes::Drawing(Some(Box::new(DrawingOptions::new(None, Some(SaveMode::Offline))))))),
                                    horizontal_space(Length::FillPortion(2)),
                                    if globals.get_db().is_some() && globals.get_user().is_some() {
                                        button("Online")
                                            .padding(8)
                                            .width(Length::FillPortion(1))
                                            .on_press(Message::ChangeScene(Scenes::Drawing(Some(Box::new(DrawingOptions::new(None, Some(SaveMode::Online)))))))
                                    } else {
                                        button("Online")
                                            .padding(8)
                                            .width(Length::FillPortion(1))
                                    },
                                ]
                            ]
                                .height(Length::Fixed(150.0))
                        )
                            .width(Length::Fixed(300.0))
                            .on_close(Message::DoAction(Box::new(MainAction::ToggleModal(ModalType::SelectingSaveMode))))
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

/// The tabs for the drawing list overlay:
/// - [Offline](MainTabIds::Offline), for drawings stored locally;
/// - [Online](MainTabIds::Online), for drawings stored remotely in the mongo database.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MainTabIds {
    Offline,
    Online,
}

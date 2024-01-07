use std::any::Any;

use iced::{Alignment, Command, Element, Length, Renderer};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, text, column, row, Container, Column, Scrollable, horizontal_space};
use iced_aw::{Card, modal};
use mongodb::bson::{Uuid, doc, Document, Bson, UuidRepresentation};

use crate::scene::{Scene, Action, Message, SceneOptions, Globals};
use crate::scenes::auth::{AuthOptions, TabIds};
use crate::scenes::scenes::Scenes;

use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::scenes::drawing::DrawingOptions;
use crate::theme::Theme;

/// The [Messages](Action) of the main [Scene]:
/// - [None](MainAction::None) for when no action is required;
/// - [ShowDrawings](MainAction::ShowDrawings), which opens a [modal](modal::Modal)
/// with a list of the drawings;
/// - [LoadedDrawings](MainAction::LoadedDrawings), which receives the list of drawings from
/// the [Database](mongodb::Database).
#[derive(Clone)]
enum MainAction {
    None,
    ShowDrawings,
    LoadedDrawings(Vec<Document>),
    LogOut,
}

impl Action for MainAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            MainAction::None => String::from("None"),
            MainAction::ShowDrawings => String::from("Show drawings"),
            MainAction::LoadedDrawings(_) => String::from("Loaded drawings"),
            MainAction::LogOut => String::from("Logged out"),
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
    showing_drawings: bool,
    drawings: Option<Vec<Uuid>>,
    globals: Globals,
}

/// The [Main] scene has no options.
#[derive(Debug, Clone, Copy)]
pub struct MainOptions {}

impl SceneOptions<Main> for MainOptions {
    fn apply_options(&self, _scene: &mut Main) { }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Main>> {
        Box::new((*self).clone())
    }
}

impl Scene for Main {
    fn new(options: Option<Box<dyn SceneOptions<Main>>>, globals: Globals) -> (Self, Command<Message>) where Self: Sized {
        let mut main = Main { showing_drawings: false, drawings: None, globals };
        if let Some(options) = options {
            options.apply_options(&mut main);
        }

        (main, Command::none())
    }

    fn get_title(&self) -> String {
        String::from("Main")
    }

    fn update(&mut self, message: Box<dyn Action>) -> Command<Message> {
        let message: &MainAction = message.as_any().downcast_ref::<MainAction>().expect("Panic downcasting to MainAction");

        match message {
            MainAction::ShowDrawings => {
                self.showing_drawings ^= true;
                if self.drawings.is_none() {
                    return Command::perform(
                        async { },
                            move |_| {
                            Message::SendMongoRequests(
                                vec![
                                    MongoRequest::new(
                                        "canvases".into(),
                                        MongoRequestType::Get(doc!{}),
                                    )
                                ],
                                |res| {
                                    if let Some(MongoResponse::Get(cursor)) = res.get(0) {
                                        Box::new(MainAction::LoadedDrawings(cursor.clone()))
                                    } else {
                                        Box::new(MainAction::None)
                                    }
                                }
                            )
                        }
                    );
                }
            }
            MainAction::LoadedDrawings(drawings) => {
                let mut list :Vec<Uuid>= vec![];

                for drawing in drawings {
                    if let Some(Bson::Binary(bin)) = drawing.get("id") {
                        if let Ok(uuid) = bin.to_uuid_with_representation(UuidRepresentation::Standard) {
                            println!("{}", uuid);
                            list.push(uuid);
                        }
                    }
                }

                self.drawings = Some(list);
            }
            MainAction::LogOut => {
                self.globals.set_user(None);
                let globals = self.globals.clone();

                return Command::perform(
                    async { },
                    |_| {
                        Message::UpdateGlobals(globals)
                    }
                );
            }
            _ => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<Message, Renderer<Theme>> {
        let container_auth :Element<Message, Renderer<Theme>>= if let Some(user) = self.globals.get_user() {
            row![
                horizontal_space(Length::Fill),
                row![
                    text(format!("Welcome, {}!", user.get_username())).vertical_alignment(Vertical::Bottom),
                    button("Log Out").padding(8).on_press(Message::DoAction(Box::new(MainAction::LogOut))),
                ]
                    .align_items(Alignment::Center)
                    .width(Length::Shrink)
                    .spacing(20)
            ].into()
        } else {
            row![
                horizontal_space(Length::Fill),
                row![
                    button("Register").padding(8).on_press(Message::ChangeScene(Scenes::Auth(Some(Box::new(AuthOptions::new(TabIds::Register)))))),
                    button("Log In").padding(8).on_press(Message::ChangeScene(Scenes::Auth(Some(Box::new(AuthOptions::new(TabIds::LogIn)))))),
                ]
                    .width(Length::Shrink)
                    .spacing(10)
                ,
            ].into()
        };

        let container_entrance :Container<Message, Renderer<Theme>> = Container::new(column![
            container_auth,
            column![
                text("Chartsy").width(Length::Shrink).size(50)
                ]
                .height(Length::FillPortion(2))
                .width(Length::Fill)
                .align_items(Alignment::Center)
            ,
            column![
                button("Start new Drawing").padding(8).on_press(Message::ChangeScene(Scenes::Drawing(None))),
                button("Continue drawing").padding(8).on_press(Message::DoAction(Box::new(MainAction::ShowDrawings))),
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
            .align_items(Alignment::Center));

        let container_drawings =
        Container::<Message, Renderer<Theme>>::new(
            Card::new(
                text("Your drawings").horizontal_alignment(Horizontal::Center).size(25),
                Container::new(
                    Scrollable::new(
                        Column::with_children(
                            if let Some(drawings) = self.drawings.clone() {
                            drawings.clone().iter().map(|uuid| {
                                Element::from(button(text(uuid)).on_press(
                                    Message::ChangeScene(Scenes::Drawing(Some(Box::new(DrawingOptions::new(Some(uuid.clone()))))))
                                ))
                            }).collect()
                            } else {
                                vec![]
                            }
                        )
                    )
                )
                    .width(Length::Fixed(500.0))
                    .height(Length::Fixed(300.0))
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Top)
            )
                .width(Length::Fixed(500.0))
                .height(Length::Fixed(300.0))
                .on_close(Message::DoAction(Box::new(MainAction::ShowDrawings)))
        )
            .padding(10)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);

        modal::<Message, Renderer<Theme>>(container_entrance, if self.showing_drawings {Some(container_drawings)} else {None})
            .into()

    }

    fn update_globals(&mut self, globals: Globals) { self.globals = globals; }

    fn clear(&self) { }
}
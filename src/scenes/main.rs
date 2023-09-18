use std::any::Any;

use iced::{Alignment, Command, Element, Length};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, text, column, Container, Column, Scrollable};
use iced_aw::{Card, modal};
use mongodb::bson::{Uuid, doc, Document, Bson, UuidRepresentation};

use crate::scene::{Scene, Action, Message, SceneOptions, Globals};
use crate::scenes::scenes::Scenes;

//use crate::menu::menu;
use crate::mongo::{MongoRequest, MongoResponse};
use crate::scenes::drawing::DrawingOptions;

#[derive(Clone)]
enum MainAction {
    None,
    ShowDrawings,
    LoadedDrawings(Vec<Document>)
}

impl Action for MainAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        String::from("No actions in main!")
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

#[derive(Clone)]
pub struct Main {
    showing_drawings: bool,
    drawings: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Copy)]
pub struct MainOptions {}

impl SceneOptions<Main> for MainOptions {
    fn apply_options(&self, _scene: &mut Main) { }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Main>> {
        Box::new((*self).clone())
    }
}

impl Scene for Main {
    fn new(options: Option<Box<dyn SceneOptions<Main>>>, _globals: Globals) -> (Self, Command<Message>) where Self: Sized {
        let mut main = Main { showing_drawings: false, drawings: None };
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
                            Message::SendMongoRequest((
                                "canvases".into(),
                                MongoRequest::Get(doc!{}),
                                |res| {
                                    if let MongoResponse::Get(cursor) = res {
                                        Box::new(MainAction::LoadedDrawings(cursor))
                                    } else {
                                        Box::new(MainAction::None)
                                    }
                                }
                                ))
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
            _ => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let container_entrance :Container<Message> = Container::new(column![
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
        Container::new(
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

        modal(container_entrance, if self.showing_drawings {Some(container_drawings)} else {None})
            .into()

    }

    fn update_globals(&mut self, _globals: Globals) { }

    fn clear(&self) { }
}
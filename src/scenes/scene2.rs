use std::any::Any;

use iced::{Alignment, Element, Length};
use iced::widget::{button, text, column, row};

use crate::scene::{Scene, Action, Message};
use crate::scenes::scenes::Scenes;

#[derive(Clone)]
pub enum Scene2Action {
    Uno,
    Dos,
    Tres,
}

impl Action for Scene2Action {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            Scene2Action::Uno => String::from("Uno"),
            Scene2Action::Dos => String::from("Dos"),
            Scene2Action::Tres => String::from("Tres"),
        }
    }
}

#[derive(Clone)]
pub struct Scene2 {
    pub action: Scene2Action,
}

impl Scene2 {
    pub fn new() -> Self {
        Scene2 {action: Scene2Action::Uno}
    }
}

impl Scene for Scene2 {

    fn get_title(&self) -> String {
        String::from("Scene 2")
    }

    fn update(&mut self, message: &dyn Action) {
        let message :&Scene2Action= message.as_any().downcast_ref::<Scene2Action>().expect("Panic downcasting to Scene2Action");

        match message {
            Scene2Action::Uno => {self.action = Scene2Action::Uno}
            Scene2Action::Dos => {self.action = Scene2Action::Dos}
            Scene2Action::Tres => {self.action = Scene2Action::Tres}
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text(format!("{}:{}", self.get_title(), self.action.get_name())).width(Length::Shrink).size(50),
            row![
                button("Uno").padding(8).on_press(Message::DoAction(&Scene2Action::Uno)),
                button("Dos").padding(8).on_press(Message::DoAction(&Scene2Action::Dos)),
                button("Tres").padding(8).on_press(Message::DoAction(&Scene2Action::Tres)),
            ],
            button("Switch").padding(8).on_press(Message::ChangeScene(Scenes::Scene1)),
        ]
            .padding(20)
            .spacing(20)
            .align_items(Alignment::Center)
            .into()
    }
}
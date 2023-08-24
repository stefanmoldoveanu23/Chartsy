use std::any::Any;

use iced::{Alignment, Element, Length};
use iced::widget::{button, text, column, row};

use crate::scene::{Scene, Action, Message};
use crate::scenes::scenes::Scenes;

#[derive(Clone)]
pub enum Scene1Action {
    One,
    Two,
}

impl Action for Scene1Action {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            Scene1Action::One => String::from("One"),
            Scene1Action::Two => String::from("Two"),
        }
    }
}

#[derive(Clone)]
pub struct Scene1 {
    pub action: Scene1Action,
}

impl Scene1 {
    pub fn new() -> Self {
        Scene1 {action: Scene1Action::One}
    }
}

impl Scene for Scene1 {

    fn get_title(&self) -> String {
        String::from("Scene 1")
    }

    fn update(&mut self, message: &dyn Action) {
        let message :&Scene1Action= message.as_any().downcast_ref::<Scene1Action>().expect("Panic downcasting to Scene1Action");

        match message {
            Scene1Action::One => {self.action = Scene1Action::One}
            Scene1Action::Two => {self.action = Scene1Action::Two}
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text(format!("{}:{}", self.get_title(), self.action.get_name())).width(Length::Shrink).size(50),
            row![
                button("One").padding(8).on_press(Message::DoAction(&Scene1Action::One)),
                button("Two").padding(8).on_press(Message::DoAction(&Scene1Action::Two)),
            ],
            button("Switch").padding(8).on_press(Message::ChangeScene(Scenes::Scene2)),
        ]
            .padding(20)
            .spacing(20)
            .align_items(Alignment::Center)
            .into()
    }
}
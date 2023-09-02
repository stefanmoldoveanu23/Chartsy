use std::any::Any;

use iced::{Alignment, Element, Length};
use iced::widget::{button, text, column};

use crate::scene::{Scene, Action, Message};
use crate::scenes::scenes::Scenes;

#[derive(Clone)]
enum MainAction {}

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
pub struct Main;

impl Main {
    pub fn new() -> Self {
        Main
    }
}

impl Scene for Main {
    fn get_title(&self) -> String {
        String::from("Main")
    }

    fn update(&mut self, message: Box<dyn Action>) {
        let _message: &MainAction = message.as_any().downcast_ref::<MainAction>().expect("Panic downcasting to MainAction");

    }

    fn view(&self) -> Element<'_, Message> {
        column![
            column![
                text("Chartsy").width(Length::Shrink).size(50)
                ]
                .height(Length::FillPortion(2))
                .align_items(Alignment::Center)
            ,
            column![
                button("Start Drawing").padding(8).on_press(Message::ChangeScene(Scenes::Drawing))
                ]
                .height(Length::FillPortion(3))
                .align_items(Alignment::Center)
            ,
        ]
            .spacing(20)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }
}
use std::any::Any;
use std::fmt::{Debug, Formatter};
use iced::Element;
use crate::scenes::scenes::Scenes;

pub trait Scene: Send+Sync {
    fn get_title(&self) -> String;
    fn update(&mut self, message: &dyn Action);
    fn view(&self) -> Element<'_, Message>;
}

impl Debug for dyn Scene {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scene {{{}}}.", self.get_title())
    }
}

pub trait Action: Send+Sync {
    fn as_any(&self) -> &dyn Any;
    fn get_name(&self) -> String;
}

impl Debug for dyn Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Action{{{}}}.", self.get_name())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ChangeScene(Scenes),
    DoAction(&'static dyn Action),
}

use std::any::Any;
use std::fmt::{Debug, Formatter};
use iced::{Command, Element, Event, Renderer, Size};
use mongodb::{Database};
use crate::mongo::{MongoRequest, MongoResponse};
use crate::scenes::scenes::Scenes;
use crate::theme::Theme;

pub trait Scene: Send+Sync {
    fn new(options: Option<Box<dyn SceneOptions<Self>>>, globals: Globals) -> (Self, Command<Message>) where Self:Sized;
    fn get_title(&self) -> String;
    fn update(&mut self, message: Box<dyn Action>) -> Command<Message>;
    fn view(&self) -> Element<'_, Message, Renderer<Theme>>;
    fn update_globals(&mut self, globals: Globals);
    fn clear(&self);
}

impl Debug for dyn Scene {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scene {{{}}}.", self.get_title())
    }
}

pub trait SceneOptions<SceneType:Scene>: Debug+Send+Sync {
    fn apply_options(&self, scene: &mut SceneType);
    fn boxed_clone(&self) -> Box<dyn SceneOptions<SceneType>>;
}

impl<SceneType:Scene> Clone for Box<dyn SceneOptions<SceneType>> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

pub trait Action: Send+Sync {
    fn as_any(&self) -> &dyn Any;
    fn get_name(&self) -> String;
    fn boxed_clone(&self) -> Box<dyn Action + 'static>;
}

impl Clone for Box<dyn Action + 'static> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

impl Debug for dyn Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Action{{{}}}.", self.get_name())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Error(String),
    ChangeScene(Scenes),
    DoAction(Box<dyn Action>),
    DoneDatabaseInit(Result<Database, mongodb::error::Error>),
    SendMongoRequests(Vec<MongoRequest>, fn(Vec<MongoResponse>) -> Box<dyn Action>),
    Event(Event)
}

#[derive(Debug, Copy, Clone)]
pub struct Globals {
    window_size: Size,
}

impl Globals {
    pub(crate) fn set_window_size(&mut self, size: Size) {
        self.window_size = size;
    }

    pub(crate) fn get_window_height(&self) -> f32 {
        self.window_size.height
    }

    pub(crate) fn get_window_width(&self) -> f32 {
        self.window_size.width
    }

    pub(crate) fn get_window_size(&self) -> Size {
        self.window_size
    }
}

impl Default for Globals {
    fn default() -> Self {
        Globals {
            window_size: Size::new(0.0, 0.0)
        }
    }
}
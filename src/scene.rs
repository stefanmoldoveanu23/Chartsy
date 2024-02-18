use std::any::Any;
use std::fmt::{Debug, Formatter};
use iced::{Command, Element, Event, Renderer, Size};
use mongodb::{Database};
use crate::errors::error::Error;
use crate::mongo::{MongoRequest, MongoResponse};
use crate::scenes::auth::User;
use crate::scenes::scenes::Scenes;
use crate::theme::Theme;

/// An individual scene that handles its actions internally.
pub trait Scene: Send+Sync {
    /// Returns a [Scene] by initializing it with its [options](SceneOptions) and giving it access to
    /// the [global](Globals) values.
    fn new(options: Option<Box<dyn SceneOptions<Self>>>, globals: Globals) -> (Self, Command<Message>) where Self:Sized;
    /// Returns the name of the [Scene].
    fn get_title(&self) -> String;
    /// Updates the [Scene] using the given [message](Action); to be called in the
    /// [update](iced::Application::update) function of the [Application](crate::Chartsy).
    fn update(&mut self, message: Box<dyn Action>) -> Command<Message>;
    /// Returns a view of the [Scene]; to be called in the [view](iced::Application::view)
    /// function of the [Application](crate::Chartsy).
    fn view(&self) -> Element<'_, Message, Renderer<Theme>>;
    /// Returns the [scenes](Scene) own error handler action.
    fn get_error_handler(&self, error: Error) -> Box<dyn Action>;
    /// Updates the [global values](Globals) when they change externally.
    fn update_globals(&mut self, globals: Globals);
    /// Handles closing the [Scene].
    fn clear(&self);
}

impl Debug for dyn Scene {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scene {{{}}}.", self.get_title())
    }
}

/// Options that help initialize a [Scene].
pub trait SceneOptions<SceneType:Scene>: Debug+Send+Sync {
    /// This function applies the options to the given [Scene].
    fn apply_options(&self, scene: &mut SceneType);
    /// Returns a clone of the reference to the [options](SceneOptions) enclosed in a [Box].
    fn boxed_clone(&self) -> Box<dyn SceneOptions<SceneType>>;
}

impl<SceneType:Scene> Clone for Box<dyn SceneOptions<SceneType>> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

/// The individual messages for a [Scene].
pub trait Action: Send+Sync {
    /// Returns an upcasted reference of the [Action] as [Any].
    fn as_any(&self) -> &dyn Any;
    /// Returns the name of the [Action].
    fn get_name(&self) -> String;
    /// Returns a reference to a clone of the [Action] enclosed in a [Box].
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

/// The Messages used in the [Application](crate::Chartsy):
/// - [Error](Message::Error), for error handling;
/// - [ChangeScene](Message::ChangeScene), for handling transitions between [Scenes](Scene);
/// - [DoAction](Message::DoAction), which is passed to the [update](Scene::update) function
/// of the current [Scene];
/// - [UpdateGlobals](Message::UpdateGlobals), to update the global values in the current [Scene](Scene);
/// - [DoneDatabaseInit](Message::DoneDatabaseInit), which signals that the mongo [Database]
/// connection was completed successfully;
/// - [SendMongoRequests](Message::SendMongoRequests), for sending [MongoRequests](MongoRequest)
/// to the [Database];
/// - [SendSmtpMail](Message::SendSmtpMail), to send an e-mail using the official email address;
/// - [Event](Message::Event), for handling [Events](Event).
#[derive(Debug, Clone)]
pub enum Message {
    None,
    Error(Error),
    ChangeScene(Scenes),
    DoAction(Box<dyn Action>),
    UpdateGlobals(Globals),
    DoneDatabaseInit(Result<Database, Error>),
    SendMongoRequests(Vec<MongoRequest>, fn(Vec<MongoResponse>) -> Box<dyn Action>),
    SendSmtpMail(lettre::Message),
    Event(Event)
}

/// The [Applications](crate::Chartsy) global values.
#[derive(Debug, Clone)]
pub struct Globals {
    user: Option<User>,
    window_size: Size,
}

impl Globals {
    /// Updates the value of the user.
    pub(crate) fn set_user (&mut self, user: Option<User>) { self.user = user; }

    /// Returns the user data.
    pub(crate) fn get_user (&self) -> Option<User> { self.user.clone() }

    /// Updates the value of the window_size.
    pub(crate) fn set_window_size(&mut self, size: Size) {
        self.window_size = size;
    }

    /// Returns the height of the window.
    pub(crate) fn get_window_height(&self) -> f32 {
        self.window_size.height
    }

    /// Returns the width of the window.
    pub(crate) fn get_window_width(&self) -> f32 {
        self.window_size.width
    }

    /// Returns the size of the window.
    pub(crate) fn get_window_size(&self) -> Size {
        self.window_size
    }
}

impl Default for Globals {
    fn default() -> Self {
        Globals {
            user: None,
            window_size: Size::new(0.0, 0.0)
        }
    }
}
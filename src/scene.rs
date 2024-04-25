use crate::errors::error::Error;
use crate::scenes::data::auth::User;
use crate::scenes::scenes::Scenes;
use crate::theme::Theme;
use iced::{Command, Element, Renderer};
use mongodb::{Client, ClientSession, Database};
use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;
use moka::future::Cache;
use mongodb::bson::Uuid;

/// An individual scene that handles its actions internally.
pub trait Scene: Send + Sync {
    /// Returns a [Scene] by initializing it with its [options](SceneOptions) and giving it access to
    /// the [global](Globals) values.
    fn new(
        options: Option<Box<dyn SceneOptions<Self>>>,
        globals: &mut Globals
    ) -> (Self, Command<Message>)
    where
        Self: Sized;

    /// Returns the name of the [Scene].
    fn get_title(&self) -> String;

    /// Updates the [Scene] using the given [message](Action); to be called in the
    /// [update](iced::Application::update) function of the [Application](crate::Chartsy).
    fn update(&mut self, globals: &mut Globals, message: Box<dyn Action>) -> Command<Message>;

    /// Returns a view of the [Scene]; to be called in the [view](iced::Application::view)
    /// function of the [Application](crate::Chartsy).
    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer>;

    /// Returns the [scenes](Scene) own error handler action.
    fn get_error_handler(&self, error: Error) -> Box<dyn Action>;

    /// Handles closing the [Scene].
    fn clear(&self, globals: &mut Globals);
}

impl Debug for dyn Scene {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scene {{{}}}.", self.get_title())
    }
}

/// Options that help initialize a [Scene].
pub trait SceneOptions<SceneType: Scene>: Debug + Send + Sync {
    /// This function applies the options to the given [Scene].
    fn apply_options(&self, scene: &mut SceneType);

    /// Returns a clone of the reference to the [options](SceneOptions) enclosed in a [Box].
    fn boxed_clone(&self) -> Box<dyn SceneOptions<SceneType>>;
}

impl<SceneType: Scene> Clone for Box<dyn SceneOptions<SceneType>> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

/// The individual messages for a [Scene].
pub trait Action: Send + Sync {
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

/// The Messages used in the [Application](crate::Chartsy).
#[derive(Debug, Clone)]
pub enum Message {
    None,
    /// Handles errors.
    Error(Error),
    /// Changes the scene to the given [Scene](Scenes).
    ChangeScene(Scenes),
    /// Performs an [Action], which should correspond to the current [scenes](Scene) enum of messages.
    DoAction(Box<dyn Action>),
    /// Triggers when a database connection has been established.
    DoneDatabaseInit(Result<Client, Error>),
    /// Triggers when a user has been logged in using a token stored locally from a previous login.
    AutoLoggedIn(User),
    /// Sends en e-mail.
    SendSmtpMail(lettre::Message),
    /// Quits the application.
    Quit
}

/// The [Applications](crate::Chartsy) global values.
#[derive(Debug, Clone)]
pub struct Globals {
    /// The data corresponding the authenticated [User]. Is None is no user is authenticated.
    user: Option<User>,

    /// The database the program is connected to.
    mongo_client: Option<Client>,

    /// The caching system.
    cache: Cache<Uuid, Arc<Vec<u8>>>
}

impl Globals {
    /// Updates the value of the user.
    pub fn set_user(&mut self, user: Option<User>) {
        self.user = user;
    }

    /// Returns the user data.
    pub fn get_user(&self) -> Option<&User> { self.user.as_ref() }

    /// Returns the user data as mutable.
    pub fn get_user_mut(&mut self) -> Option<&mut User> { self.user.as_mut() }

    /// Updates the client object.
    pub fn set_client(&mut self, client: Client) { self.mongo_client = Some(client); }

    /// Returns the database from the client.
    pub fn get_db(&self) -> Option<Database> {
        match &self.mongo_client {
            Some(client) => Some(client.database("chartsy")),
            None => None
        }
    }

    /// Starts a mongo session and returns it.
    pub async fn start_session(&self) -> Option<Result<ClientSession, Error>> {
        match &self.mongo_client {
            Some(client) => Some(client.start_session(None).await.map_err(|err| err.into())),
            None => None
        }
    }

    /// Returns the cache.
    pub fn get_cache(&self) -> &Cache<Uuid, Arc<Vec<u8>>> {
        &self.cache
    }
}

impl Default for Globals {
    fn default() -> Self {
        Globals {
            user: None,
            mongo_client: None,
            cache: Cache::builder()
                .time_to_idle(Duration::from_secs(60 * 60))
                .max_capacity(500 * 1024 * 1024)
                .build()
        }
    }
}

use crate::debug_message;
use crate::scenes::data::auth::User;
use crate::scenes::scenes::Scenes;
use crate::utils::cache::Cache;
use crate::utils::errors::Error;
use crate::utils::icons::{Icon, ICON};
use iced::advanced::widget::Text;
use iced::widget::{Button, Row};
use iced::{Command, Element, Renderer};
use iced::{Length, Theme};
use mongodb::{Client, ClientSession, Database};
use std::any::Any;
use std::fmt::{Debug, Formatter};

/// An individual scene that handles its actions internally.
pub trait Scene: {
    type Message: SceneMessage;
    type Options: Debug + Send + Sync;

    /// Returns a [Scene] by initializing it with its [options](SceneOptions) and giving it access to
    /// the [global](Globals) values.
    fn new(
        options: Option<Self::Options>,
        globals: &mut Globals,
    ) -> (Self, Command<impl Into<Message>>)
    where
        Self: Sized;

    /// Returns the name of the [Scene].
    fn get_title(&self) -> String;

    /// Returns the name in an element that changes to the main [Scene].
    fn title_element(&self) -> Element<'_, Message, Theme, Renderer> {
        Row::with_children(vec![
            Button::new(Text::new(Icon::Leave.to_string()).font(ICON).size(30.0))
                .padding(0.0)
                .style(iced::widget::button::text)
                .on_press(Message::ChangeScene(Scenes::Main(None)))
                .into(),
            Text::new(self.get_title()).size(30.0).into(),
        ])
        .width(Length::Fill)
        .padding(10.0)
        .spacing(10.0)
        .into()
    }

    /// Applies the [options](Self::Options) to self.
    fn apply_options(&mut self, options: Self::Options);

    /// Gets a dynamic [message](SceneMessage) and returns it in the associated type.
    fn unwrap_message<'a>(&self, message: &'a dyn SceneMessage) -> Result<&'a Self::Message, Error>
    where
        Self::Message: 'static,
    {
        message.as_any().downcast_ref().ok_or(
            debug_message!(
                "Failed to downcast message \"{}\" to scene \"{}\".",
                message.get_name(),
                self.get_title()
            )
            .into(),
        )
    }

    /// Updates the [Scene] using the given [message](Action); to be called in the
    /// [update](iced::Application::update) function of the [Application](crate::Chartsy).
    fn update(&mut self, globals: &mut Globals, message: &Self::Message) -> Command<Message>;

    /// Returns a view of the [Scene]; to be called in the [view](iced::Application::view)
    /// function of the [Application](crate::Chartsy).
    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer>;

    /// Handles an [Error].
    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message>;

    /// Handles closing the [Scene].
    fn clear(&self, globals: &mut Globals);
}

impl<Message, Options> Debug for dyn Scene<Message = Message, Options = Options>
where
    Message: SceneMessage,
    Options: Debug + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scene {{{}}}.", self.get_title())
    }
}

/// The individual messages for a [Scene].
pub trait SceneMessage: Send + Sync {
    /// Returns an upcasted reference of the [message](SceneMessage) as [Any].
    fn as_any(&self) -> &dyn Any;

    /// Returns the name of the [message](SceneMessage).
    fn get_name(&self) -> String;

    /// Returns a reference to a clone of the [message](SceneMessage) enclosed in a [Box].
    fn boxed_clone(&self) -> Box<dyn SceneMessage + 'static>;
}

impl Clone for Box<dyn SceneMessage + 'static> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

impl Debug for dyn SceneMessage {
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
    DoAction(Box<dyn SceneMessage>),
    /// Triggers when a database connection has been established.
    DoneDatabaseInit(Result<Client, Error>),
    /// Triggers when a user has been logged in using a token stored locally from a previous login.
    AutoLoggedIn(User),
    /// Sends en e-mail.
    SendSmtpMail(lettre::Message),
    /// Quits the application.
    Quit,
}

/// The [Applications](crate::Chartsy) global values.
#[derive(Debug, Clone)]
pub struct Globals {
    /// The data corresponding the authenticated [User]. Is None is no user is authenticated.
    user: Option<User>,

    /// The database the program is connected to.
    mongo_client: Option<Client>,

    /// The caching system.
    cache: Cache,
}

impl Globals {
    /// Updates the value of the user.
    pub fn set_user(&mut self, user: Option<User>) {
        self.user = user;
    }

    /// Returns the user data.
    pub fn get_user(&self) -> Option<&User> {
        self.user.as_ref()
    }

    /// Returns the user data as mutable.
    pub fn get_user_mut(&mut self) -> Option<&mut User> {
        self.user.as_mut()
    }

    /// Updates the client object.
    pub fn set_client(&mut self, client: Client) {
        self.mongo_client = Some(client);
    }

    /// Returns the database from the client.
    pub fn get_db(&self) -> Option<Database> {
        match &self.mongo_client {
            Some(client) => Some(client.database("chartsy")),
            None => None,
        }
    }

    /// Starts a mongo session and returns it.
    pub async fn start_session(&self) -> Option<Result<ClientSession, Error>> {
        match &self.mongo_client {
            Some(client) => Some(
                client
                    .start_session(None)
                    .await
                    .map_err(|err| debug_message!("{}", err).into()),
            ),
            None => None,
        }
    }

    /// Returns a clone of the cache.
    pub fn get_cache(&self) -> Cache {
        self.cache.clone()
    }
}

impl Default for Globals {
    fn default() -> Self {
        Globals {
            user: None,
            mongo_client: None,
            cache: Cache::new(),
        }
    }
}

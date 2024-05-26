use std::ops::Deref;
use crate::scene::{SceneMessage, Globals, Message, Scene};
use crate::scenes::auth::{Auth, AuthOptions};
use crate::scenes::drawing::{Drawing, DrawingOptions};
use crate::scenes::main::{Main, MainOptions};
use iced::{Command, Element, Renderer};
use crate::debug_message;
use crate::errors::error::Error;
use crate::scenes::posts::{Posts, PostsOptions};
use crate::scenes::settings::{Settings, SettingsOptions};
use crate::utils::theme::Theme;

/// The list of [Scenes](Scene) in the [Application](crate::Chartsy).
#[derive(Debug, Clone)]
pub enum Scenes {
    Main(Option<MainOptions>),
    Drawing(Option<DrawingOptions>),
    Auth(Option<AuthOptions>),
    Posts(Option<PostsOptions>),
    Settings(Option<SettingsOptions>)
}

/// The [Scene] transition manager.
///
/// Holds the current [Scene](Scenes) and an instance of each [Scene].
pub struct SceneManager {
    current_scene: Scenes,
    main: Option<Main>,
    drawing: Option<Drawing>,
    auth: Option<Auth>,
    posts: Option<Posts>,
    settings: Option<Settings>,
}

impl SceneManager {
    pub fn new(globals: &mut Globals) -> Self
    {
        SceneManager {
            current_scene: Scenes::Main(None),
            main: Some(Main::new(None, globals).0),
            drawing: None,
            auth: None,
            posts: None,
            settings: None,
        }
    }
    
    /// Closes the current [Scene] and opens the requested [Scene].
    pub fn load(&mut self, scene: Scenes, globals: &mut Globals) -> Command<Message> {
        match self.current_scene {
            Scenes::Main(_) => {
                if let Some(main) = &self.main {
                    main.clear(globals);
                }
                self.main = None
            }
            Scenes::Drawing(_) => {
                if let Some(drawing) = &self.drawing {
                    drawing.clear(globals);
                }
                self.drawing = None
            }
            Scenes::Auth(_) => {
                if let Some(auth) = &self.auth {
                    auth.clear(globals);
                }
                self.auth = None;
            }
            Scenes::Posts(_) => {
                if let Some(posts) = &self.posts {
                    posts.clear(globals);
                }
                self.posts = None;
            },
            Scenes::Settings(_) => {
                if let Some(settings) = &self.settings {
                    settings.clear(globals);
                }
                self.settings = None;
            }
        }

        self.current_scene = scene;

        match &self.current_scene {
            Scenes::Main(options) => {
                let (main, command) = Scene::new(options.clone(), globals);
                self.main = Some(main);
                Command::batch(vec![command])
            }
            Scenes::Drawing(options) => {
                let (drawing, command) = Scene::new(options.clone(), globals);
                self.drawing = Some(drawing);
                Command::batch(vec![command])
            }
            Scenes::Auth(options) => {
                let (auth, command) = Scene::new(options.clone(), globals);
                self.auth = Some(auth);
                Command::batch(vec![command])
            }
            Scenes::Posts(options) => {
                let (posts, command) = Scene::new(options.clone(), globals);
                self.posts = Some(posts);
                Command::batch(vec![command])
            }
            Scenes::Settings(options) => {
                let (settings, command) = Scene::new(options.clone(), globals);
                self.settings = Some(settings);
                Command::batch(vec![command])
            }
        }
    }

    /// Returns the current [Scene] as a mutable variable.
    pub fn update(&mut self, globals: &mut Globals, message: Box<dyn SceneMessage>)
        -> Result<Command<Message>, Error> {
        match self.current_scene {
            Scenes::Main(_) => match self.main {
                None => Err(debug_message!("Main scene missing.").into()),
                Some(ref mut main) => main.unwrap_message(message.deref()).map(
                    |message| main.update(globals, message)
                )
            }
            Scenes::Drawing(_) => match self.drawing {
                None => Err(debug_message!("Drawing scene missing.").into()),
                Some(ref mut drawing) => drawing.unwrap_message(message.deref()).map(
                    |message| drawing.update(globals, message)
                )
            },
            Scenes::Auth(_) => match self.auth {
                None => Err(debug_message!("Auth scene missing.").into()),
                Some(ref mut auth) => auth.unwrap_message(message.deref()).map(
                    |message| auth.update(globals, message)
                )
            },
            Scenes::Posts(_) => match self.posts {
                None => Err(debug_message!("Posts scene missing.").into()),
                Some(ref mut posts) => posts.unwrap_message(message.deref()).map(
                    |message| posts.update(globals, message)
                )
            },
            Scenes::Settings(_) => match self.settings {
                None => Err(debug_message!("Settings scene missing.").into()),
                Some(ref mut settings) => settings.unwrap_message(message.deref()).map(
                    |message| settings.update(globals, message)
                )
            }
        }
    }

    /// Returns the current [Scene].
    pub fn view(&self, globals: &Globals) -> Result<Element<Message, Theme, Renderer>, Error> {
        match self.current_scene {
            Scenes::Main(_) => match self.main {
                None => Err(debug_message!("Main scene missing.").into()),
                Some(ref main) => Ok(main.view(globals))
            }
            Scenes::Drawing(_) => match self.drawing {
                None => Err(debug_message!("Drawing scene missing.").into()),
                Some(ref drawing) => Ok(drawing.view(globals))
            },
            Scenes::Auth(_) => match self.auth {
                None => Err(debug_message!("Auth scene missing.").into()),
                Some(ref auth) => Ok(auth.view(globals)),
            },
            Scenes::Posts(_) => match self.posts {
                None => Err(debug_message!("Posts scene missing.").into()),
                Some(ref posts) => Ok(posts.view(globals)),
            },
            Scenes::Settings(_) => match self.settings {
                None => Err(debug_message!("Settings scene missing.").into()),
                Some(ref settings) => Ok(settings.view(globals))
            }
        }
    }

    /// Handles an error.
    pub fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Result<Command<Message>, Error>
    {
        match self.current_scene {
            Scenes::Main(_) => match self.main {
                None => Err(debug_message!("Main scene missing.").into()),
                Some(ref mut main) => Ok(main.handle_error(globals, error))
            },
            Scenes::Drawing(_) => match self.drawing {
                None => Err(debug_message!("Drawing scene missing.").into()),
                Some(ref mut drawing) => Ok(drawing.handle_error(globals, error))
            },
            Scenes::Auth(_) => match self.auth {
                None => Err(debug_message!("Auth scene missing.").into()),
                Some(ref mut auth) => Ok(auth.handle_error(globals, error))
            },
            Scenes::Posts(_) => match self.posts {
                None => Err(debug_message!("Posts scene missing.").into()),
                Some(ref mut posts) => Ok(posts.handle_error(globals, error))
            },
            Scenes::Settings(_) => match self.settings {
                None => Err(debug_message!("Settings scene missing.").into()),
                Some(ref mut settings) => Ok(settings.handle_error(globals, error))
            }
        }
    }
}

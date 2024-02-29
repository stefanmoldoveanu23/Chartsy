use crate::scene::{Globals, Message, Scene, SceneOptions};
use crate::scenes::auth::Auth;
use crate::scenes::drawing::Drawing;
use crate::scenes::main::Main;
use iced::Command;

/// The list of [Scenes](Scene) in the [Application](crate::Chartsy).
#[derive(Debug, Clone)]
pub enum Scenes {
    Main(Option<Box<dyn SceneOptions<Main>>>),
    Drawing(Option<Box<dyn SceneOptions<Box<Drawing>>>>),
    Auth(Option<Box<dyn SceneOptions<Auth>>>),
}

/// An enum that is returned when an unusual behaviour occurs during the handling of [Scenes](Scene).
#[derive(Debug)]
pub enum SceneErr {
    Error,
}

/// The [Scene] transition manager.
///
/// Holds the current [Scene](Scenes) and an instance of each [Scene].
pub struct SceneLoader {
    current_scene: Scenes,
    main: Option<Main>,
    drawing: Option<Box<Drawing>>,
    auth: Option<Auth>,
}

impl SceneLoader {
    /// Closes the current [Scene] and opens the requested [Scene].
    pub fn load(&mut self, scene: Scenes, globals: Globals) -> Command<Message> {
        match self.current_scene {
            Scenes::Main(_) => {
                if let Some(main) = &self.main {
                    main.clear();
                }
                self.main = None
            }
            Scenes::Drawing(_) => {
                if let Some(drawing) = &self.drawing {
                    drawing.clear();
                }
                self.drawing = None
            }
            Scenes::Auth(_) => {
                if let Some(auth) = &self.auth {
                    auth.clear();
                }
                self.auth = None;
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
        }
    }

    /// Returns the current [Scene] as a mutable variable.
    pub fn get_mut(&mut self) -> Result<&mut dyn Scene, SceneErr> {
        match self.current_scene {
            Scenes::Main(_) => match self.main {
                None => Err(SceneErr::Error),
                Some(ref mut scene) => Ok(scene),
            },
            Scenes::Drawing(_) => match self.drawing {
                None => Err(SceneErr::Error),
                Some(ref mut scene) => Ok(scene),
            },
            Scenes::Auth(_) => match self.auth {
                None => Err(SceneErr::Error),
                Some(ref mut scene) => Ok(scene),
            },
        }
    }

    /// Returns the current [Scene].
    pub fn get(&self) -> Result<&dyn Scene, SceneErr> {
        match self.current_scene {
            Scenes::Main(_) => match self.main {
                None => Err(SceneErr::Error),
                Some(ref scene) => Ok(scene),
            },
            Scenes::Drawing(_) => match self.drawing {
                None => Err(SceneErr::Error),
                Some(ref scene) => Ok(scene),
            },
            Scenes::Auth(_) => match self.auth {
                None => Err(SceneErr::Error),
                Some(ref scene) => Ok(scene),
            },
        }
    }
}

impl Default for SceneLoader {
    fn default() -> Self {
        SceneLoader {
            current_scene: Scenes::Main(None),
            main: Some(Main::new(None, Globals::default()).0),
            drawing: None,
            auth: None,
        }
    }
}

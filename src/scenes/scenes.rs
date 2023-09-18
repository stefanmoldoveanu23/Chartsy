use iced::{Command};
use crate::scene::{Globals, Message, Scene, SceneOptions};
use crate::scenes::drawing::Drawing;
use crate::scenes::main::Main;

#[derive(Debug, Clone)]
pub enum Scenes {
    Main(Option<Box<dyn SceneOptions<Main>>>),
    Drawing(Option<Box<dyn SceneOptions<Box<Drawing>>>>),
}

#[derive(Debug)]
pub enum SceneErr {
    Error,
}

pub struct SceneLoader {
    current_scene: Scenes,
    main: Option<Main>,
    drawing: Option<Box<Drawing>>,
}

impl SceneLoader {
    pub fn load(&mut self, scene: Scenes, globals: Globals) -> Command<Message> {
        match self.current_scene {
            Scenes::Main(_) => {
                self.main = None
            }
            Scenes::Drawing(_) => {
                self.drawing = None
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
        }
    }

    pub fn get_mut(&mut self) -> Result<& mut dyn Scene, SceneErr> {
        match self.current_scene {
            Scenes::Main(_) => {
                match self.main {
                    None => Err(SceneErr::Error),
                    Some(ref mut scene) => Ok(scene)
                }
            }
            Scenes::Drawing(_) => {
                match self.drawing {
                    None => Err(SceneErr::Error),
                    Some(ref mut scene) => Ok(scene)
                }
            }
        }
    }

    pub fn get(&self) -> Result<& dyn Scene, SceneErr> {
        match self.current_scene {
            Scenes::Main(_) => {
                match self.main {
                    None => Err(SceneErr::Error),
                    Some(ref scene) => Ok(scene)
                }
            }
            Scenes::Drawing(_) => {
                match self.drawing {
                    None => Err(SceneErr::Error),
                    Some(ref scene) => Ok(scene)
                }
            }
        }
    }
}

impl Default for SceneLoader {
    fn default() -> Self {
        SceneLoader {
            current_scene: Scenes::Main(None),
            main: Some(Main::new(None, Globals::default()).0),
            drawing: None,
        }
    }
}
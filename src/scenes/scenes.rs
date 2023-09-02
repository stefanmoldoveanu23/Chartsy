use crate::scene::Scene;
use crate::scenes::drawing::Drawing;
use crate::scenes::main::Main;

#[derive(Debug, Clone, Copy)]
pub enum Scenes {
    Main,
    Drawing,
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
    pub fn load(&mut self, scene: Scenes) {
        match self.current_scene {
            Scenes::Main => {
                self.main = None
            }
            Scenes::Drawing => {
                self.drawing = None
            }
        }

        self.current_scene = scene;

        match self.current_scene {
            Scenes::Main => {
                self.main = Some(Main::new());
            }
            Scenes::Drawing => {
                self.drawing = Some(Drawing::new());
            }
        }
    }

    pub fn get_mut(&mut self) -> Result<& mut dyn Scene, SceneErr> {
        match self.current_scene {
            Scenes::Main => {
                match self.main {
                    None => Err(SceneErr::Error),
                    Some(ref mut scene) => Ok(scene)
                }
            }
            Scenes::Drawing => {
                match self.drawing {
                    None => Err(SceneErr::Error),
                    Some(ref mut scene) => Ok(scene)
                }
            }
        }
    }

    pub fn get(&self) -> Result<& dyn Scene, SceneErr> {
        match self.current_scene {
            Scenes::Main => {
                match self.main {
                    None => Err(SceneErr::Error),
                    Some(ref scene) => Ok(scene)
                }
            }
            Scenes::Drawing => {
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
            current_scene: Scenes::Main,
            main: Some(Main::new()),
            drawing: None,
        }
    }
}
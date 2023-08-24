use crate::scene::Scene;
use crate::scenes::scene1::{Scene1};
use crate::scenes::scene2::{Scene2};

#[derive(Debug, Clone, Copy)]
pub enum Scenes {
    Scene1,
    Scene2,
}

#[derive(Debug)]
pub enum SceneErr {
    Error,
}

pub struct SceneLoader {
    current_scene: Scenes,
    scene1: Option<Scene1>,
    scene2: Option<Scene2>,
}

impl SceneLoader {
    pub fn load(&mut self, scene: Scenes) {
        match self.current_scene {
            Scenes::Scene1 => {
                self.scene1 = None;
            }
            Scenes::Scene2 => {
                self.scene2 = None;
            }
        }

        self.current_scene = scene;

        match self.current_scene {
            Scenes::Scene1 => {
                self.scene1 = Some(Scene1::new());
            }
            Scenes::Scene2 => {
                self.scene2 = Some(Scene2::new());
            }
        }
    }

    pub fn get_mut(&mut self) -> Result<& mut dyn Scene, SceneErr> {
        match self.current_scene {
            Scenes::Scene1 => {
                match self.scene1 {
                    None => Err(SceneErr::Error),
                    Some(ref mut scene) => Ok(scene)
                }
            }
            Scenes::Scene2 => {
                match self.scene2 {
                    None => Err(SceneErr::Error),
                    Some(ref mut scene) => Ok(scene)
                }
            }
        }
    }

    pub fn get(&self) -> Result<& dyn Scene, SceneErr> {
        match self.current_scene {
            Scenes::Scene1 => {
                match self.scene1 {
                    None => Err(SceneErr::Error),
                    Some(ref scene) => Ok(scene)
                }
            }
            Scenes::Scene2 => {
                match self.scene2 {
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
            current_scene: Scenes::Scene1,
            scene1: Some(Scene1::new()),
            scene2: None,
        }
    }
}
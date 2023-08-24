mod scene;
mod scenes;

use scene::{Message};
use scenes::scenes::SceneLoader;

use iced::{Element, Sandbox, Settings};

pub fn main() -> iced::Result {
    Chartsy::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

struct Chartsy {
    scene_loader: SceneLoader,
}

impl Sandbox for Chartsy {
    type Message = Message;

    fn new() -> Self {
        Chartsy{scene_loader: SceneLoader::default()}
    }

    fn title(&self) -> String {
        String::from("Title")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::ChangeScene(scene) => {
                self.scene_loader.load(scene);
            }
            Message::DoAction(action) => {
                let scene = self.scene_loader.get_mut().expect("Error getting scene.");
                scene.update(action);
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let scene = self.scene_loader.get().expect("Error getting scene.");
        scene.view()
    }
}
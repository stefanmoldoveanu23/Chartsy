#![windows_subsystem = "windows"]

mod scene;
mod scenes;
mod tool;
mod tools;
mod menu;

use scene::{Message};
use scenes::scenes::SceneLoader;

use iced::{Application, Command, Element, executor, Settings, Theme};

pub fn main() -> iced::Result {
    Chartsy::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

struct Chartsy {
    scene_loader: SceneLoader,
}

impl Application for Chartsy {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Chartsy, Command<Self::Message>) {
        (Chartsy{scene_loader: SceneLoader::default()}, Command::none())
    }

    fn title(&self) -> String {
        String::from("Title")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {

        match message {
            Message::ChangeScene(scene) => {
                self.scene_loader.load(scene);
                Command::none()
            }
            Message::DoAction(action) => {
                let scene = self.scene_loader.get_mut().expect("Error getting scene.");
                scene.update(action);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let scene = self.scene_loader.get().expect("Error getting scene.");
        scene.view()
    }
}
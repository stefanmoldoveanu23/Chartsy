#![windows_subsystem = "windows"]

mod scene;
mod scenes;
mod tool;
mod tools;
mod menu;
mod mongo;
mod config;
mod serde;

use scene::{Message};
use scenes::scenes::SceneLoader;

use iced::{Application, Command, Element, executor, Settings, Theme};
use mongodb::Database;

pub fn main() -> iced::Result {
    Chartsy::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

struct Chartsy {
    scene_loader: SceneLoader,
    mongo_db: Option<Database>,
}

impl Application for Chartsy {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Chartsy, Command<Self::Message>) {
        (Chartsy{scene_loader: SceneLoader::default(), mongo_db: None}, Command::perform(mongo::connect_to_mongodb(), Message::DoneDatabaseInit))
    }

    fn title(&self) -> String {
        String::from("Title")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {

        match message {
            Message::ChangeScene(scene) => {
                self.scene_loader.load(scene)
            }
            Message::DoAction(action) => {
                let scene = self.scene_loader.get_mut().expect("Error getting scene.");
                scene.update(action)
            }
            Message::DoneDatabaseInit(result) => {
                self.mongo_db = Some(result.expect("Error connecting to database."));

                println!("Successfully connected to database.");
                Command::none()
            }
            Message::SendMongoRequest(request) => {
                match &self.mongo_db {
                    None => Command::none(),
                    Some(db) => mongo::MongoRequest::send_request(db.clone(), request)
                }
            }
            Message::Error(message) => {
                eprintln!("{}", message);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let scene = self.scene_loader.get().expect("Error getting scene.");
        scene.view()
    }
}
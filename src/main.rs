#![windows_subsystem = "windows"]

mod scene;
mod scenes;
mod tool;
mod tools;
mod mongo;
mod config;
mod serde;
mod theme;
mod canvas;

use scene::{Message, Globals};
use scenes::scenes::SceneLoader;
use theme::Theme;

use iced::{Application, Command, Element, executor, Renderer, Settings, Size, Subscription, window};
use iced::subscription::events;
use iced_runtime::command::Action;
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
    globals: Globals,
}

impl Application for Chartsy {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Chartsy, Command<Self::Message>) {
        (
            Chartsy{scene_loader: SceneLoader::default(), mongo_db: None, globals: Globals::default()},
            Command::batch(
                vec![
                    Command::single(Action::Window(window::Action::Maximize(true))),
                    Command::perform(mongo::connect_to_mongodb(), Message::DoneDatabaseInit)
                ]
            )
        )
    }

    fn title(&self) -> String {
        String::from("Title")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {

        match message {
            Message::ChangeScene(scene) => {
                self.scene_loader.load(scene, self.globals)
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
            Message::SendMongoRequests(requests, response_handler) => {
                match &self.mongo_db {
                    None => Command::none(),
                    Some(db) => mongo::MongoRequest::send_requests(db.clone(), (requests, response_handler))
                }
            }
            Message::Event(event) => {
                match event {
                    iced::Event::Window(window::Event::Resized {width, height}) => {
                        self.globals.set_window_size(Size::new(width as f32, height as f32));
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::Error(message) => {
                eprintln!("{}", message);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        let scene = self.scene_loader.get().expect("Error getting scene.");
        scene.view()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        events().map(Message::Event)
    }
}
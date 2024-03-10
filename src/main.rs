#![cfg_attr(
    all(
        target_os = "windows",
        not(debug_assertions),
    ),
    windows_subsystem = "windows"
)]

mod scene;
mod scenes;
mod mongo;
mod config;
mod serde;
mod theme;
mod canvas;
mod errors;
mod widgets;

use scene::{Message, Globals};
use scenes::scenes::SceneLoader;
use theme::Theme;

use crate::config::{EMAIL_PASS, EMAIL_USERNAME};
use iced::event;
use iced::{
    executor, window, Application, Command, Element, Renderer, Settings, Size, Subscription,
};
use iced_runtime::command::Action;
use lettre::{AsyncSmtpTransport, AsyncStd1Executor, AsyncTransport};
use mongodb::Database;

pub fn main() -> iced::Result {
    Chartsy::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

/// The model for the [Application].
///
/// Its purpose is to manage the basic aspects of the drawing app:
/// - transitioning between different scenes, including the handling of
/// closing and opening a scene using a [SceneLoader];
/// - communication with a [Database];
/// - holding and passing of global values, using the [Globals] structure.
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
            Chartsy{
                scene_loader: SceneLoader::default(),
                mongo_db: None,
                globals: Globals::default()
            },
            Command::batch(vec![
                Command::single(Action::Window(window::Action::Maximize(window::Id::MAIN, true))),
                Command::perform(mongo::connect_to_mongodb(), Message::DoneDatabaseInit),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("Chartsy")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        return Command::none();

        match message {
            Message::None => {
                Command::none()
            }
            Message::ChangeScene(scene) => {
                self.scene_loader.load(scene, self.globals.clone())
            }
            Message::DoAction(action) => {
                let scene = self.scene_loader.get_mut().expect("Error getting scene.");
                scene.update(action)
            }
            Message::UpdateGlobals(globals) => {
                self.globals = globals;

                let scene = self.scene_loader.get_mut().expect("Error getting scene.");
                scene.update_globals(self.globals.clone());

                Command::none()
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
            Message::SendSmtpMail(mail) => {
                Command::perform(
                    async {
                        let connection = AsyncSmtpTransport::<AsyncStd1Executor>::from_url(
                            &*format!("smtps://{}:{}@smtp.gmail.com:465/", EMAIL_USERNAME, EMAIL_PASS)
                        ).unwrap().build();

                        let result = connection.send(mail).await;
                        if let Err(ref err) = result {
                            println!("Error sending mail! {}", err);
                        } else {
                            println!("Mail sent successfully!");
                        }

                        result
                    },
                    |_result| Message::None
                )
            }
            Message::Event(event) => {
                match event {
                    iced::Event::Window(window::Id::MAIN, window::Event::Resized { width, height }) => {
                        self.globals
                            .set_window_size(Size::new(width as f32, height as f32));
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::Error(error) => {
                if error.is_debug() {
                    eprintln!("{}", error);
                    Command::none()
                } else {
                    let scene = self.scene_loader.get_mut().expect("Error getting scene.");
                    scene.update(scene.get_error_handler(error))
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        let scene = self.scene_loader.get().expect("Error getting scene.");
        scene.view(&self.globals)
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        event::listen().map(Message::Event)
    }
}

#![cfg_attr(
    all(
        target_os = "windows",
        not(debug_assertions),
    ),
    windows_subsystem = "windows"
)]

#[macro_use]
extern crate litcrypt2;

use_litcrypt!();

mod canvas;
mod config;
mod errors;
mod mongo;
mod scene;
mod scenes;
mod serde;
mod theme;
mod widgets;
mod icons;

use scene::{Globals, Message};
use scenes::scenes::SceneLoader;
use theme::Theme;

use iced::{executor, window, Application, Command, Element, Renderer, Settings, Subscription};
use lettre::{AsyncSmtpTransport, AsyncStd1Executor, AsyncTransport};

pub fn main() -> iced::Result {
    Chartsy::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

/// The model for the [Application].
struct Chartsy {
    /// Handles transitions between scenes.
    scene_loader: SceneLoader,

    /// Holds the global data.
    globals: Globals,
}

impl Application for Chartsy {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Chartsy, Command<Self::Message>) {
        let mut globals = Globals::default();
        let scene_loader = SceneLoader::new(&mut globals);

        (
            Chartsy {
                scene_loader,
                globals,
            },
            Command::batch(vec![
                window::change_mode(window::Id::MAIN, window::Mode::Fullscreen),
                iced::font::load(icons::ICON_BYTES).map(|_| Message::None),
                Command::perform(mongo::base::connect_to_mongodb(), Message::DoneDatabaseInit),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("Chartsy")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::None => Command::none(),
            Message::ChangeScene(scene) => self.scene_loader.load(scene, &mut self.globals),
            Message::DoAction(action) => {
                let scene = self.scene_loader.get_mut().expect("Error getting scene.");
                scene.update(&mut self.globals, action)
            }
            Message::DoneDatabaseInit(result) => {
                match result {
                    Ok(db) => {
                        self.globals.set_db(db.clone());

                        println!("Successfully connected to database.");
                        Command::perform(
                            async move {
                                let result = mongo::auth::get_user_from_token(&db).await;

                                if let Ok(user) = &result {
                                    let user_id = user.get_id();

                                    mongo::auth::update_user_token(&db, user_id).await;
                                }

                                result
                            },
                            |result| {
                                match result {
                                    Ok(user) => Message::AutoLoggedIn(user),
                                    Err(err) => Message::Error(err)
                                }
                            }
                        )
                    }
                    Err(err) => {
                        println!("Error connecting to database: {}", err);
                        Command::perform(mongo::base::connect_to_mongodb(), Message::DoneDatabaseInit)
                    }
                }

            }
            Message::AutoLoggedIn(user) => {
                self.globals.set_user(Some(user));
                Command::none()
            }
            Message::SendSmtpMail(mail) => Command::perform(
                async {
                    let connection = AsyncSmtpTransport::<AsyncStd1Executor>::from_url(&*format!(
                        "smtps://{}:{}@smtp.gmail.com:465/",
                        config::email_username(), config::email_pass()
                    ))
                    .unwrap()
                    .build();

                    let result = connection.send(mail).await;
                    if let Err(ref err) = result {
                        println!("Error sending mail! {}", err);
                    } else {
                        println!("Mail sent successfully!");
                    }

                    result
                },
                |_result| Message::None,
            ),
            Message::Error(error) => {
                if error.is_debug() {
                    eprintln!("{}", error);
                    Command::none()
                } else {
                    let scene = self.scene_loader.get_mut().expect("Error getting scene.");
                    scene.update(&mut self.globals, scene.get_error_handler(error))
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        let scene = self.scene_loader.get().expect("Error getting scene.");
        scene.view(&self.globals)
    }

    fn subscription(&self) -> Subscription<Self::Message> { Subscription::none() }
}

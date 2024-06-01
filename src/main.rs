#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]

#[macro_use]
extern crate litcrypt2;

use_litcrypt!();

mod canvas;
mod config;
mod database;
mod errors;
mod scene;
mod scenes;
mod services;
mod utils;
mod widgets;

use errors::error::Error;
use iced::theme::palette::{self, Background, Danger, Primary, Secondary, Success};
use lettre::transport::smtp::response::Response;
use scene::{Globals, Message};
use scenes::scenes::SceneManager;
use utils::theme::SECONDARY;

use crate::widgets::wait_panel::WaitPanel;
use iced::font::{Family, Stretch, Style, Weight};
use iced::{
    advanced::Application, executor, window, Command, Element, Font, Renderer, Settings,
    Subscription,
};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

pub const LOADING_IMAGE: &[u8] = include_bytes!("images/loading.png");

pub const INCONSOLATA_BYTES: &[u8] = include_bytes!("images/Inconsolata-SemiBold.ttf");
pub const INCONSOLATA: Font = Font {
    family: Family::Name("Inconsolata"),
    weight: Weight::Semibold,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

pub fn main() -> iced::Result {
    Chartsy::run(Settings {
        antialiasing: true,
        default_font: INCONSOLATA,
        ..Settings::default()
    })
}

/// The model for the [Application].
struct Chartsy {
    /// Handles transitions between scenes.
    scene_loader: SceneManager,

    /// Holds the global data.
    globals: Globals,
}

impl Application for Chartsy {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();
    type Renderer = iced::Renderer;

    fn new(_flags: Self::Flags) -> (Chartsy, Command<Self::Message>) {
        let mut globals = Globals::default();
        let scene_loader = SceneManager::new(&mut globals);

        (
            Chartsy {
                scene_loader,
                globals,
            },
            Command::batch(vec![
                window::maximize(window::Id::MAIN, true),
                iced::font::load(utils::icons::ICON_BYTES).map(|_| Message::None),
                iced::font::load(INCONSOLATA_BYTES).map(|_| Message::None),
                Command::perform(
                    database::base::connect_to_mongodb(),
                    Message::DoneDatabaseInit,
                ),
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
                match self.scene_loader.update(&mut self.globals, action) {
                    Ok(command) => command,
                    Err(err) => self.update(Message::Error(err)),
                }
            }
            Message::DoneDatabaseInit(result) => match result {
                Ok(client) => {
                    self.globals.set_client(client);
                    let db = self.globals.get_db().unwrap();

                    println!("Successfully connected to database.");
                    Command::perform(
                        async move {
                            let user = database::auth::get_user_from_token(&db).await?;

                            let user_id = user.get_id();
                            database::auth::update_user_token(&db, user_id).await?;

                            Ok(user)
                        },
                        |result| match result {
                            Ok(user) => Message::AutoLoggedIn(user),
                            Err(err) => Message::Error(err),
                        },
                    )
                }
                Err(err) => {
                    println!("Error connecting to database: {}", err);
                    Command::perform(
                        database::base::connect_to_mongodb(),
                        Message::DoneDatabaseInit,
                    )
                }
            },
            Message::AutoLoggedIn(user) => {
                self.globals.set_user(Some(user));
                Command::none()
            }
            Message::SendSmtpMail(mail) => Command::perform(
                async {
                    let connection =
                        match AsyncSmtpTransport::<Tokio1Executor>::from_url(&*format!(
                            "smtps://{}:{}@smtp.gmail.com:465/",
                            config::email_username(),
                            config::email_pass()
                        )) {
                            Ok(connection) => connection.build(),
                            Err(err) => {
                                return Err(debug_message!("{}", err).into());
                            }
                        };

                    let result = connection.send(mail).await;

                    result.map_err(|err| debug_message!("{}", err).into())
                },
                |result: Result<Response, Error>| match result {
                    Ok(response) => Message::Error(
                        format!("Mail sent with response {}.", response.code()).into(),
                    ),
                    Err(err) => Message::Error(err),
                },
            ),
            Message::Error(error) => {
                if error.is_debug() {
                    eprintln!("{}", error);
                    Command::none()
                } else {
                    match self.scene_loader.handle_error(&mut self.globals, &error) {
                        Ok(command) => command,
                        Err(err) => self.update(Message::Error(err)),
                    }
                }
            }
            Message::Quit => window::close(window::Id::MAIN),
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        match self.scene_loader.view(&self.globals) {
            Ok(element) => element,
            Err(err) => {
                if err.is_debug() {
                    eprintln!("{}", err);
                }

                WaitPanel::new("Trouble loading scene...").into()
            }
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    fn theme(&self) -> Self::Theme {
        utils::theme::Theme::custom_with_fn(
            String::from("Chartsy"),
            utils::theme::PALETTE,
            |palette| palette::Extended {
                background: Background::new(palette.background, palette.text),
                primary: Primary::generate(palette.primary, palette.background, palette.background),
                secondary: Secondary::generate(SECONDARY, palette.background),
                success: Success::generate(palette.success, palette.background, palette.text),
                danger: Danger::generate(palette.danger, palette.background, palette.text),
                is_dark: true,
            },
        )
    }
}

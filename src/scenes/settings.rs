use std::any::Any;
use iced::{Alignment, Command, Element, Length, Renderer};
use iced::advanced::image::Handle;
use iced::widget::{Button, Column};
use rfd::AsyncFileDialog;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::theme::Theme;

pub struct Settings {
    image: Handle
}

#[derive(Debug, Clone)]
pub struct SettingsOptions { }

impl SceneOptions<Settings> for SettingsOptions {
    fn apply_options(&self, _scene: &mut Settings) { }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Settings>> {
        Box::new((*self).clone())
    }
}

#[derive(Clone)]
pub enum SettingsAction {
    None,
    SelectImage,
    SetImage(String)
}

impl Action for SettingsAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            SettingsAction::None => String::from("None"),
            SettingsAction::SelectImage => String::from("Select image"),
            SettingsAction::SetImage(_) => String::from("Set image"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn Action + 'static> {
        Box::new((*self).clone())
    }
}

impl Scene for Settings {
    fn new(options: Option<Box<dyn SceneOptions<Self>>>, _globals: &mut Globals)
        -> (Self, Command<Message>) where Self: Sized {

        let mut settings = Self {
            image: Handle::from_path("./src/images/loading.png")
        };

        if let Some(options) = options {
            options.apply_options(&mut settings);
        }

        (
            settings,
            Command::none()
            )
    }

    fn get_title(&self) -> String {
        "Settings".into()
    }

    fn update(&mut self, _globals: &mut Globals, message: Box<dyn Action>) -> Command<Message> {
        let message = message
            .as_any()
            .downcast_ref::<SettingsAction>()
            .expect("Panic downcasting to SettingsAction");

        match message {
            SettingsAction::SelectImage => {
                Command::perform(
                    async {
                        let file = AsyncFileDialog::new()
                            .add_filter("image", &["png", "jpg", "jpeg", "webp"])
                            .set_directory("~")
                            .pick_file()
                            .await;

                        match file {
                            Some(file) => Ok(String::from(file.path().to_str().unwrap())),
                            None => {
                                Err(Error::DebugError(DebugError::new(
                                    "Error getting file path."
                                )))
                            }
                        }
                    },
                    |result| {
                        match result {
                            Ok(path) => Message::DoAction(Box::new(SettingsAction::SetImage(path))),
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            },
            SettingsAction::SetImage(path) => {
                println!("{}", path);
                self.image = Handle::from_path(path);

                Command::none()
            }
            SettingsAction::None => Command::none()
        }
    }

    fn view(&self, _globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        Column::from_vec(vec![
            iced::widget::image::Image::new(self.image.clone())
                .width(500.0)
                .height(500.0)
                .into(),
            Button::new("Select image")
                .on_press(Message::DoAction(Box::new(SettingsAction::SelectImage)))
                .into()
        ])
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(20.0)
            .into()
    }

    fn get_error_handler(&self, _error: Error) -> Box<dyn Action> { Box::new(SettingsAction::None) }

    fn clear(&self) { }
}
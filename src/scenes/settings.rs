use crate::database;
use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::scenes::data::auth::User;
use crate::scenes::scenes::Scenes;
use crate::utils::errors::{AuthError, Error};
use crate::utils::theme::{self, Theme};
use crate::widgets::{ModalStack, WaitPanel};
use iced::advanced::image::Handle;
use iced::widget::{Button, Column, Row, Scrollable, Space, Text};
use iced::{Alignment, Command, Element, Length, Renderer};
use mongodb::bson::doc;
use std::any::Any;
use std::sync::Arc;

use super::services;

/// The struct for the settings [Scene].
pub struct Settings {
    /// The current user input in the username TextInput.
    username_input: String,

    /// The current user input in the user tag TextInput.
    user_tag_input: String,

    /// The current user input in the password TextInput.
    password_input: String,

    /// The current user input in the password repeat TextInput.
    password_repeat: String,

    /// The current profile picture of the user.
    profile_picture_input: Handle,

    /// The last error that an update request has created.
    input_error: Option<Error>,

    /// This is checked when the user has deleted their account.
    deleted_account: bool,

    /// Tells whether the loading panel is activated.
    modal_stack: ModalStack<()>,
}

/// This scene has no options.
#[derive(Debug, Clone)]
pub struct SettingsOptions {}

/// The possible [messages](SceneMessage) this [Scene] can trigger.
#[derive(Clone)]
pub enum SettingsMessage {
    /// When the username TextInput field is modified.
    UpdateUsernameField(String),

    /// Username update request.
    UpdateUsername,

    /// When the user tag TextInput field is modified.
    UpdateUserTagField(String),

    /// User tag update request.
    UpdateUserTag,

    /// When the password TextInput field is modified.
    UpdatePasswordField(String),

    /// When the password repeat TextInput is modified.
    UpdatePasswordRepeatField(String),

    /// Password update request.
    UpdatePassword,

    /// Triggers when the users profile picture has been loaded into the scene.
    LoadedProfilePicture(Vec<u8>),

    /// Opens the file dialog so that the user can select a new profile picture.
    SelectImage,

    /// Sets the users profile picture to the image selected in the file dialog.
    SetImage(Vec<u8>),

    /// Deletes the current users account.
    DeleteAccount,

    /// Triggered upon successful update.
    /// After securing that the database has been updated, the data will be set in the program as well.
    DoneUpdate(Arc<dyn Fn(&mut Settings, &mut Globals) + Send + Sync + 'static>),

    /// Handles errors.
    Error(Error),
}

impl SceneMessage for SettingsMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            Self::UpdateUsernameField(_) => String::from("Update username field"),
            Self::UpdateUsername => String::from("Update username"),
            Self::UpdateUserTagField(_) => String::from("Update user tag field"),
            Self::UpdateUserTag => String::from("Update user tag"),
            Self::UpdatePasswordField(_) => String::from("Update password field"),
            Self::UpdatePasswordRepeatField(_) => String::from("Update password repeat field"),
            Self::UpdatePassword => String::from("Update password"),
            Self::LoadedProfilePicture(_) => String::from("Loaded profile picture"),
            Self::SelectImage => String::from("Select image"),
            Self::SetImage(_) => String::from("Set image"),
            Self::DeleteAccount => String::from("Delete account"),
            Self::DoneUpdate(_) => String::from("Done update"),
            Self::Error(_) => String::from("Error"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneMessage + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Message> for SettingsMessage {
    fn into(self) -> Message {
        Message::DoAction(Box::new(self))
    }
}

impl Settings {
    fn update_username(&mut self, globals: &mut Globals) -> Command<Message> {
        if !User::check_username(&self.username_input) {
            self.input_error = Some(Error::AuthError(AuthError::RegisterBadCredentials {
                email: false,
                username: true,
                password: false,
            }));

            return Command::none();
        }

        let username = self.username_input.clone();
        let db = globals.get_db().unwrap();
        let user_id = globals.get_user().unwrap().get_id();
        self.input_error = None;

        Command::perform(
            async move {
                database::settings::update_user(&db, user_id, doc! { "username": username.clone() })
                    .await
                    .map(|()| username)
            },
            move |result| match result {
                Ok(username) => SettingsMessage::DoneUpdate(Arc::new(move |_settings, globals| {
                    globals
                        .get_user_mut()
                        .unwrap()
                        .set_username(username.clone())
                }))
                .into(),
                Err(err) => Message::Error(err),
            },
        )
    }

    fn update_user_tag(&mut self, globals: &mut Globals) -> Command<Message> {
        if !User::check_user_tag(&self.user_tag_input) {
            self.input_error = Some(Error::AuthError(AuthError::BadUserTag));

            Command::none()
        } else {
            let tag = self.user_tag_input.clone();
            let globals = globals.clone();
            self.input_error = None;

            Command::perform(
                async move {
                    database::settings::find_user_by_tag(&globals, tag.clone())
                        .await
                        .map(|()| tag)
                },
                |result| match result {
                    Ok(tag) => SettingsMessage::DoneUpdate(Arc::new(move |_settings, globals| {
                        globals.get_user_mut().unwrap().set_user_tag(tag.clone())
                    }))
                    .into(),
                    Err(err) => Message::Error(err),
                },
            )
        }
    }

    fn update_password(&mut self, globals: &mut Globals) -> Command<Message> {
        if !User::check_password(&self.password_input) {
            self.input_error = Some(Error::AuthError(AuthError::RegisterBadCredentials {
                email: false,
                username: false,
                password: true,
            }));

            return Command::none();
        }

        let password = self.password_input.clone();
        let db = globals.get_db().unwrap();
        let user_id = globals.get_user().unwrap().get_id();
        self.input_error = None;

        Command::perform(
            async move {
                database::settings::update_user(
                    &db,
                    user_id,
                    doc! {
                        "password": pwhash::bcrypt::hash(password.clone()).unwrap()
                    },
                )
                .await
            },
            |result| match result {
                Ok(_) => SettingsMessage::DoneUpdate(Arc::new(move |settings, _globals| {
                    settings.password_input = String::from("");
                    settings.password_repeat = String::from("");
                }))
                .into(),
                Err(err) => Message::Error(err),
            },
        )
    }

    fn update_profile_picture(
        &mut self,
        data: &Vec<u8>,
        globals: &mut Globals,
    ) -> Command<Message> {
        let data = data.clone();
        self.modal_stack.toggle_modal(());

        let need_mongo_update = !globals.get_user().unwrap().has_profile_picture();
        let db = globals.get_db().unwrap();
        let user_id = globals.get_user().unwrap().get_id();

        let data = data.clone();

        Command::perform(
            async move {
                services::settings::set_user_image(
                    data.clone(),
                    user_id,
                    if need_mongo_update { Some(&db) } else { None },
                )
                .await
                .map(|()| data)
            },
            |result| match result {
                Ok(data) => SettingsMessage::DoneUpdate(Arc::new(move |settings, globals| {
                    settings.profile_picture_input = Handle::from_bytes(data.clone());
                    globals.get_user_mut().unwrap().set_profile_picture();
                    settings.modal_stack.toggle_modal(());
                }))
                .into(),
                Err(err) => Message::Error(err),
            },
        )
    }
}

impl Scene for Settings {
    type Message = SettingsMessage;
    type Options = SettingsOptions;

    fn new(options: Option<Self::Options>, globals: &mut Globals) -> (Self, Command<Message>)
    where
        Self: Sized,
    {
        let user = globals.get_user().unwrap().clone();

        let mut settings = Self {
            username_input: user.get_username().clone(),
            user_tag_input: user.get_user_tag().clone(),
            password_input: String::from(""),
            password_repeat: String::from(""),
            profile_picture_input: Handle::from_path("./src/images/loading.png"),
            input_error: None,
            deleted_account: false,
            modal_stack: ModalStack::new(),
        };

        if let Some(options) = options {
            settings.apply_options(options);
        }

        (
            settings,
            Command::perform(
                async move { services::settings::get_profile_picture(&user).await },
                |result| match result {
                    Ok(data) => Into::<Message>::into(SettingsMessage::LoadedProfilePicture(data)),
                    Err(err) => Message::Error(err),
                },
            ),
        )
    }

    fn get_title(&self) -> String {
        "Settings".into()
    }

    fn apply_options(&mut self, _options: Self::Options) {}

    fn update(&mut self, globals: &mut Globals, message: &Self::Message) -> Command<Message> {
        match message {
            SettingsMessage::UpdateUsernameField(username) => {
                self.username_input = username.clone();
                Command::none()
            }
            SettingsMessage::UpdateUsername => self.update_username(globals),
            SettingsMessage::UpdateUserTagField(user_tag) => {
                self.user_tag_input = user_tag.clone();

                Command::none()
            }
            SettingsMessage::UpdateUserTag => self.update_user_tag(globals),
            SettingsMessage::UpdatePasswordField(password) => {
                self.password_input = password.clone();

                Command::none()
            }
            SettingsMessage::UpdatePasswordRepeatField(password) => {
                self.password_repeat = password.clone();

                Command::none()
            }
            SettingsMessage::UpdatePassword => self.update_password(globals),
            SettingsMessage::LoadedProfilePicture(data) => {
                self.profile_picture_input = Handle::from_bytes(data.clone());

                Command::none()
            }
            SettingsMessage::SelectImage => Command::perform(
                async { services::settings::select_image().await },
                |result| match result {
                    Ok(data) => SettingsMessage::SetImage(data).into(),
                    Err(err) => Message::Error(err),
                },
            ),
            SettingsMessage::SetImage(data) => self.update_profile_picture(data, globals),
            SettingsMessage::DeleteAccount => {
                let user_id = globals.get_user().unwrap().get_id();
                let db = globals.get_db().unwrap();
                self.deleted_account = true;

                Command::perform(
                    async move { database::settings::delete_account(&db, user_id).await },
                    |result| match result {
                        Ok(_) => Message::ChangeScene(Scenes::Main(None)),
                        Err(err) => Message::Error(err),
                    },
                )
            }
            SettingsMessage::DoneUpdate(update_function) => {
                update_function(self, globals);

                Command::none()
            }
            SettingsMessage::Error(err) => {
                self.input_error = Some(err.clone());

                Command::none()
            }
        }
    }

    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let (username_error, password_error) =
            if let Some(Error::AuthError(AuthError::RegisterBadCredentials {
                email: _,
                username,
                password,
            })) = self.input_error
            {
                (username, password)
            } else {
                (false, false)
            };

        let title = self.title_element();

        let user = globals.get_user().unwrap();

        let username = services::settings::username_input(
            user.get_username().clone(),
            self.username_input.clone(),
        );

        let username_error = if username_error {
            services::settings::username_error()
        } else {
            Space::with_width(Length::Fill).into()
        };

        let user_tag = services::settings::user_tag_input(
            user.get_user_tag().clone(),
            self.user_tag_input.clone(),
        );

        let user_tag_error = if Some(Error::AuthError(AuthError::UserTagAlreadyExists))
            == self.input_error
            || Some(Error::AuthError(AuthError::BadUserTag)) == self.input_error
        {
            let error = self.input_error.clone();
            Text::new(error.unwrap().to_string())
                .style(theme::text::danger)
                .size(15.0)
                .into()
        } else {
            Space::with_width(Length::Fill).into()
        };

        let password = services::settings::password_input(
            self.password_input.clone(),
            self.password_repeat.clone(),
        );

        let password_error = if password_error {
            services::settings::password_error()
        } else {
            Space::with_width(Length::Fill).into()
        };

        let profile_picture =
            services::settings::profile_picture_input(&self.profile_picture_input);

        let profile_picture_error =
            if self.input_error == Some(Error::AuthError(AuthError::ProfilePictureTooLarge)) {
                Text::new((&self.input_error).clone().unwrap().to_string())
                    .size(15.0)
                    .into()
            } else {
                Space::with_width(Length::Fill).into()
            };

        let delete_account = Button::new("Delete account")
            .style(iced::widget::button::danger)
            .on_press(SettingsMessage::DeleteAccount.into())
            .into();

        let underlay = Column::from_vec(vec![
            title,
            Scrollable::new(Row::with_children(vec![
                Space::with_width(Length::FillPortion(1)).into(),
                Column::with_children(vec![
                    Column::with_children(vec![username, username_error]).into(),
                    Column::with_children(vec![user_tag, user_tag_error]).into(),
                    Column::with_children(vec![password, password_error]).into(),
                    Column::with_children(vec![profile_picture, profile_picture_error]).into(),
                    delete_account,
                ])
                .spacing(20.0)
                .width(Length::FillPortion(1))
                .into(),
                Space::with_width(Length::FillPortion(1)).into(),
            ]))
            .into(),
        ])
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .spacing(20.0);

        let generate_modal = |()| WaitPanel::new("Saving image. Please wait...").into();

        self.modal_stack.get_modal(underlay, generate_modal)
    }

    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message> {
        self.update(globals, &SettingsMessage::Error(error.clone()))
    }

    fn clear(&self, globals: &mut Globals) {
        if self.deleted_account {
            globals.set_user(None);
        }
    }
}

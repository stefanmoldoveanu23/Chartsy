use std::any::Any;
use std::fs;
use std::io::Cursor;
use std::ops::Deref;
use iced::{Alignment, Command, Element, Length, Renderer};
use iced::advanced::image::Handle;
use iced::widget::{Button, Column, Row, Scrollable, Space, Text, TextInput};
use mongodb::bson::doc;
use rfd::AsyncFileDialog;
use crate::errors::auth::AuthError;
use crate::errors::debug::{debug_message, DebugError};
use crate::errors::error::Error;
use crate::icons::{Icon, ICON};
use crate::database;
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::data::auth::User;
use crate::scenes::scenes::Scenes;
use crate::theme::Theme;
use crate::widgets::modal_stack::ModalStack;
use crate::widgets::wait_panel::WaitPanel;

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
pub struct SettingsOptions { }

impl SceneOptions<Settings> for SettingsOptions {
    fn apply_options(&self, _scene: &mut Settings) { }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Settings>> {
        Box::new((*self).clone())
    }
}

/// The possible [actions](Action) this [Scene] can trigger.
#[derive(Clone)]
pub enum SettingsAction {
    /// Default [Action].
    None,

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

    /// Triggered when a new profile picture has been saved.
    SavedProfilePicture,

    /// Deletes the current users account.
    DeleteAccount,

    /// Handles errors.
    Error(Error)
}

impl Action for SettingsAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            SettingsAction::None => String::from("None"),
            SettingsAction::UpdateUsernameField(_) => String::from("Update username field"),
            SettingsAction::UpdateUsername => String::from("Update username"),
            SettingsAction::UpdateUserTagField(_) => String::from("Update user tag field"),
            SettingsAction::UpdateUserTag => String::from("Update user tag"),
            SettingsAction::UpdatePasswordField(_) => String::from("Update password field"),
            SettingsAction::UpdatePasswordRepeatField(_) => String::from("Update password repeat field"),
            SettingsAction::UpdatePassword => String::from("Update password"),
            SettingsAction::LoadedProfilePicture(_) => String::from("Loaded profile picture"),
            SettingsAction::SelectImage => String::from("Select image"),
            SettingsAction::SetImage(_) => String::from("Set image"),
            SettingsAction::SavedProfilePicture => String::from("Saved profile picture"),
            SettingsAction::DeleteAccount => String::from("Delete account"),
            SettingsAction::Error(_) => String::from("Error")
        }
    }

    fn boxed_clone(&self) -> Box<dyn Action + 'static> {
        Box::new((*self).clone())
    }
}

impl Scene for Settings {
    fn new(options: Option<Box<dyn SceneOptions<Self>>>, globals: &mut Globals)
        -> (Self, Command<Message>) where Self: Sized {

        let user = globals.get_user().unwrap().clone();

        let mut settings = Self {
            username_input: user.get_username().clone(),
            user_tag_input: user.get_user_tag().clone(),
            password_input: String::from(""),
            password_repeat: String::from(""),
            profile_picture_input: Handle::from_path("./src/images/loading.png"),
            input_error: None,
            deleted_account: false,
            modal_stack: ModalStack::new()
        };

        if let Some(options) = options {
            options.apply_options(&mut settings);
        }

        (
            settings,
            Command::perform(
                async move {
                    database::base::download_file(
                        if user.has_profile_picture() {
                            format!("/{}/profile_picture.webp", user.get_id())
                        } else {
                            String::from("/default_profile_picture.webp")
                        }
                    ).await
                },
                |result| {
                    match result {
                        Ok(data) => Message::DoAction(Box::new(
                            SettingsAction::LoadedProfilePicture(data)
                        )),
                        Err(err) => Message::Error(err)
                    }
                }
            )
        )
    }

    fn get_title(&self) -> String {
        "Settings".into()
    }

    fn update(&mut self, globals: &mut Globals, message: Box<dyn Action>) -> Command<Message> {
        let as_option: Option<&SettingsAction> = message
            .as_any()
            .downcast_ref::<SettingsAction>();
        let message = if let Some(message) = as_option {
            message
        } else {
            return Command::perform(async {}, move |()| Message::Error(
                Error::DebugError(DebugError::new(
                    debug_message!(format!("Message doesn't belong to settings scene: {}.", message.get_name()))
                ))
            ))
        };

        match message {
            SettingsAction::UpdateUsernameField(username) => {
                self.username_input = username.clone();
                Command::none()
            }
            SettingsAction::UpdateUsername => {
                if !User::check_username(&self.username_input) {
                    self.input_error = Some(Error::AuthError(AuthError::RegisterBadCredentials {
                        email: false,
                        username: true,
                        password: false
                    }));

                    return Command::none();
                }

                let username = self.username_input.clone();
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();
                globals.get_user_mut().as_mut().unwrap().set_username(username.clone());
                self.input_error = None;

                Command::perform(
                    async move {
                        database::settings::update_user(
                            &db,
                            user_id,
                            doc! {
                                "username": username
                            }
                        ).await
                    },
                    |result| {
                        match result {
                            Ok(_) => Message::None,
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            }
            SettingsAction::UpdateUserTagField(user_tag) => {
                self.user_tag_input = user_tag.clone();

                Command::none()
            }
            SettingsAction::UpdateUserTag => {
                if !User::check_user_tag(&self.user_tag_input) {
                    self.input_error = Some(Error::AuthError(AuthError::BadUserTag));

                    Command::none()
                } else {
                    let tag = self.user_tag_input.clone();
                    globals.get_user_mut().as_mut().unwrap().set_user_tag(tag.clone());
                    let globals = globals.clone();

                    Command::perform(
                        async move {
                            database::settings::find_user_by_tag(&globals, tag).await
                        },
                        |result| {
                            match result {
                                Ok(()) => Message::None,
                                Err(err) => Message::Error(err)
                            }
                        }
                    )
                }
            }
            SettingsAction::UpdatePasswordField(password) => {
                self.password_input = password.clone();

                Command::none()
            }
            SettingsAction::UpdatePasswordRepeatField(password) => {
                self.password_repeat = password.clone();

                Command::none()
            }
            SettingsAction::UpdatePassword => {
                if !User::check_password(&self.password_input) {
                    self.input_error = Some(Error::AuthError(AuthError::RegisterBadCredentials {
                        email: false,
                        username: false,
                        password: true
                    }));

                    return Command::none();
                }

                let password = self.password_input.clone();
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();
                self.input_error = None;
                self.password_input = String::from("");
                self.password_repeat = String::from("");

                Command::perform(
                    async move {
                        database::settings::update_user(
                            &db,
                            user_id,
                            doc! {
                                "password": pwhash::bcrypt::hash(password.clone()).unwrap()
                            }
                        ).await
                    },
                    |result| {
                        match result {
                            Ok(_) => Message::None,
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            }
            SettingsAction::LoadedProfilePicture(data) => {
                self.profile_picture_input = Handle::from_memory(data.clone());

                Command::none()
            }
            SettingsAction::SelectImage => {
                Command::perform(
                    async {
                        let file = AsyncFileDialog::new()
                            .add_filter("image", &["png", "jpg", "jpeg", "webp"])
                            .set_directory("~")
                            .pick_file()
                            .await;

                        match file {
                            Some(file) => {
                                if fs::metadata(file.path()).unwrap().len() > 5000000 {
                                    Err(Error::AuthError(AuthError::ProfilePictureTooLarge))
                                } else {
                                    Ok(file.read().await)
                                }
                            }
                            None => Err(Error::DebugError(
                                DebugError::new(debug_message!("Error getting file path."))
                            ))
                        }
                    },
                    |result| {
                        match result {
                            Ok(data) => Message::DoAction(Box::new(SettingsAction::SetImage(data))),
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            },
            SettingsAction::SetImage(data) => {
                self.profile_picture_input = Handle::from_memory(data.clone());
                self.modal_stack.toggle_modal(());

                let need_mongo_update = !globals.get_user().unwrap().has_profile_picture();
                globals.get_user_mut().as_mut().unwrap().set_profile_picture();
                let db = globals.get_db().unwrap();
                let user_id = globals.get_user().unwrap().get_id();

                let data = data.clone();

                Command::perform(
                    async move {
                        let data = match tokio::task::spawn_blocking(
                            move || {
                                let dyn_image = match image::load_from_memory(data.as_slice()) {
                                    Ok(image) => image,
                                    Err(err) => {
                                        return Err(Error::DebugError(DebugError::new(
                                            debug_message!(err.to_string())
                                        )));
                                    }
                                };

                                match webp::Encoder::from_image(&dyn_image) {
                                    Ok(encoder) => {
                                        Ok(encoder.encode(20.0).deref().to_vec())
                                    },
                                    Err(err) => {
                                        Err(Error::DebugError(DebugError::new(debug_message!(err))))
                                    }
                                }
                            }
                        ).await {
                            Ok(Ok(data)) => data,
                            Ok(Err(err)) => {
                                return Err(err);
                            }
                            Err(err) => {
                                return Err(Error::DebugError(DebugError::new(
                                    debug_message!(err.to_string())
                                )))
                            }
                        };

                        match database::base::upload_file(
                            format!("/{}/profile_picture.webp", user_id),
                            data,
                        ).await {
                            Ok(_) => { },
                            Err(err) => {
                                return Err(err);
                            }
                        };

                        if need_mongo_update {
                            database::settings::update_user(
                                &db,
                                user_id,
                                doc! {
                                    "profile_picture": true
                                }
                            ).await
                        } else {
                            Ok(())
                        }
                    },
                    |result| {
                        match result {
                            Ok(_) => Message::DoAction(Box::new(SettingsAction::SavedProfilePicture)),
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            }
            SettingsAction::SavedProfilePicture => {
                self.modal_stack.toggle_modal(());

                Command::none()
            }
            SettingsAction::DeleteAccount => {
                let user_id = globals.get_user().unwrap().get_id();
                let db = globals.get_db().unwrap();
                self.deleted_account = true;

                Command::perform(
                    async move {
                        database::settings::delete_account(&db, user_id).await
                    },
                    |result| {
                        match result {
                            Ok(_) => Message::ChangeScene(Scenes::Main(None)),
                            Err(err) => Message::Error(err)
                        }
                    }
                )
            }
            SettingsAction::None => Command::none(),
            SettingsAction::Error(err) => {
                self.input_error = Some(err.clone());

                Command::none()
            }
        }
    }

    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let (username_error, password_error) =
            if let Some(Error::AuthError(AuthError::RegisterBadCredentials { email: _, username, password }))
                = self.input_error {
                (username, password)
            } else {
                (false, false)
            };

        let title = Row::with_children(vec![
            Button::new(
                Text::new(Icon::Leave.to_string()).font(ICON).size(30.0)
            )
                .padding(0.0)
                .style(crate::theme::button::Button::Transparent)
                .on_press(Message::ChangeScene(Scenes::Main(None)))
                .into(),
            Text::new(self.get_title()).size(30.0).into()
        ])
            .width(Length::Fill)
            .padding(10.0)
            .spacing(10.0);

        let user = globals.get_user().unwrap();

        let username = Column::with_children(vec![
            Text::new("Username").size(20.0).into(),
            Row::with_children(vec![
                TextInput::new(
                    "Input username...",
                    &*self.username_input.clone()
                )
                    .on_input(|value| Message::DoAction(Box::new(
                        SettingsAction::UpdateUsernameField(value.clone())
                    )))
                    .size(15.0)
                    .into(),
                Space::with_width(Length::Fill)
                    .into(),
                if self.username_input.clone() == user.get_username().clone() {
                    Button::new(Text::new("Update").size(15.0))
                } else {
                    Button::new(Text::new("Update").size(15.0))
                        .on_press(Message::DoAction(Box::new(SettingsAction::UpdateUsername)))
                }
                    .into()
            ])
                .spacing(5.0)
                .into()
        ])
            .width(Length::Fill)
            .spacing(5.0)
            .into();

        let username_error = if username_error {
            Text::new(
                Error::AuthError(AuthError::RegisterBadCredentials {
                    email: false,
                    username: true,
                    password: false,
                })
                    .to_string()
            )
                .style(crate::theme::text::Text::Error)
                .size(15.0)
                .into()
        } else {
            Space::with_width(Length::Fill).into()
        };

        let user_tag = Column::with_children(vec![
            Text::new("User Tag").size(20.0).into(),
            Row::with_children(vec![
                TextInput::new(
                    "Input user tag...",
                    &*self.user_tag_input.clone()
                )
                    .on_input(|value| Message::DoAction(Box::new(
                        SettingsAction::UpdateUserTagField(value.clone())
                    )))
                    .size(15.0)
                    .into(),
                Space::with_width(Length::Fill)
                    .into(),
                if self.user_tag_input.clone() == user.get_user_tag().clone() {
                    Button::new(Text::new("Update").size(15.0))
                } else {
                    Button::new(Text::new("Update").size(15.0))
                        .on_press(Message::DoAction(Box::new(SettingsAction::UpdateUserTag)))
                }
                    .into()
            ])
                .spacing(5.0)
                .into()
        ])
            .width(Length::Fill)
            .spacing(5.0)
            .into();

        let user_tag_error =
            if Some(Error::AuthError(AuthError::UserTagAlreadyExists)) == self.input_error ||
                Some(Error::AuthError(AuthError::BadUserTag)) == self.input_error {
                let error = self.input_error.clone();
                Text::new(error.unwrap().to_string())
                    .style(crate::theme::text::Text::Error)
                    .size(15.0)
                    .into()
            } else {
                Space::with_width(Length::Fill).into()
            };

        let password = Row::with_children(vec![
            Column::with_children(vec![
                Text::new("Password").size(20.0).into(),
                TextInput::new(
                    "Input password...",
                    &*self.password_input.clone()
                )
                    .size(15.0)
                    .on_input(|value| Message::DoAction(Box::new(
                        SettingsAction::UpdatePasswordField(value.clone())
                    )))
                    .secure(true)
                    .into(),
                TextInput::new(
                    "Repeat password...",
                    &*self.password_repeat.clone()
                )
                    .size(15.0)
                    .on_input(|value| Message::DoAction(Box::new(
                        SettingsAction::UpdatePasswordRepeatField(value.clone())
                    )))
                    .secure(true)
                    .into()
            ])
                .spacing(5.0)
                .into(),
            Space::with_width(Length::Fill)
                .into(),
            if self.password_input == self.password_repeat {
                Button::new(Text::new("Update").size(15.0))
                    .on_press(Message::DoAction(Box::new(SettingsAction::UpdatePassword)))
            } else {
                Button::new(Text::new("Update").size(15.0))
            }
                .into()
        ])
            .align_items(Alignment::End)
            .width(Length::Fill)
            .spacing(5.0)
            .into();

        let password_error = if password_error {
            Text::new(
                Error::AuthError(AuthError::RegisterBadCredentials {
                    email: false,
                    username: false,
                    password: true
                })
                    .to_string()
            )
                .style(crate::theme::text::Text::Error)
                .size(15.0)
                .into()
        } else {
            Space::with_width(Length::Fill).into()
        };

        let profile_picture = Row::with_children(vec![
            Text::new("Profile picture").size(20.0)
                .into(),
            Space::with_width(Length::Fill)
                .into(),
            Column::with_children(vec![
                iced::widget::image::Image::new(self.profile_picture_input.clone())
                    .height(200.0)
                    .width(200.0)
                    .into(),
                Button::new("Select image")
                    .on_press(Message::DoAction(Box::new(SettingsAction::SelectImage)))
                    .into()
            ])
                .align_items(Alignment::Center)
                .spacing(10.0)
                .into()
        ])
            .align_items(Alignment::Center)
            .into();

        let profile_picture_error =
            if self.input_error == Some(Error::AuthError(AuthError::ProfilePictureTooLarge)) {
                Text::new((&self.input_error).clone().unwrap().to_string())
                    .size(15.0)
                    .into()
            } else {
                Space::with_width(Length::Fill).into()
            };

        let delete_account =
            Button::new("Delete account")
                .style(crate::theme::button::Button::Danger)
                .on_press(Message::DoAction(Box::new(SettingsAction::DeleteAccount)))
                .into();

        let underlay = Column::from_vec(vec![
            title.into(),
            Scrollable::new(
                Row::with_children(vec![
                    Space::with_width(Length::FillPortion(1))
                        .into(),
                    Column::with_children(vec![
                        Column::with_children(vec![
                            username,
                            username_error,
                        ])
                            .into(),
                        Column::with_children(vec![
                            user_tag,
                            user_tag_error
                        ])
                            .into(),
                        Column::with_children(vec![
                            password,
                            password_error
                        ])
                            .into(),
                        Column::with_children(vec![
                            profile_picture,
                            profile_picture_error
                        ])
                            .into(),
                        delete_account
                    ])
                        .spacing(20.0)
                        .width(Length::FillPortion(1))
                        .into(),
                    Space::with_width(Length::FillPortion(1))
                        .into()
                ])
            )
                .into(),
        ])
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(20.0);

        let generate_modal = |()| {
            WaitPanel::new("Saving image. Please wait...").into()
        };

        self.modal_stack.get_modal(underlay, generate_modal)
    }

    fn get_error_handler(&self, error: Error) -> Box<dyn Action> { Box::new(SettingsAction::Error(error)) }

    fn clear(&self, globals: &mut Globals) {
        if self.deleted_account {
            globals.set_user(None);
        }
    }
}
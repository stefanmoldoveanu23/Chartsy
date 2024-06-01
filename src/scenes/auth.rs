use crate::database;
use crate::errors::auth::AuthError;
use crate::errors::error::Error;
use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::scenes::data::auth::*;
use crate::scenes::scenes::Scenes;
use crate::utils::serde::Serialize;
use crate::utils::theme::Theme;
use crate::widgets::tabs::Tabs;
use iced::widget::{Button, Column, Container, Row, Space, Text, TextInput};
use iced::{Command, Element, Length, Renderer};
use std::any::Any;

/// Possible messages for the authentication page.
#[derive(Clone)]
pub enum AuthMessage {
    /// Triggered when a field in the registration form has been updated.
    RegisterTextFieldUpdate(RegisterField),

    /// Triggered when a field in the login form has been updated.
    LogInTextFieldUpdate(LogInField),

    /// Sends a registration request.
    /// If the boolean is false, then it will add the user in the database, and it will trigger the
    ///  same message with the boolean set to true, which will send the validation e-mail.
    SendRegister(bool),

    /// Checks whether the validation code that the user added is correct.
    ValidateEmail,

    /// Resets the email validation code.
    ResetRegisterCode,

    /// Triggered when the registration process is complete.
    DoneRegistration,

    /// Sends a login request.
    SendLogIn,

    /// Triggered when the user has been successfully logged in. Holds the user data.
    LoggedIn(User),

    /// Used for switching between the registration/login tabs.
    TabSelection(AuthTabIds),

    /// Handles errors.
    HandleError(Error),
}

impl SceneMessage for AuthMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            Self::RegisterTextFieldUpdate(_) => String::from("Modified register text input field"),
            Self::LogInTextFieldUpdate(_) => String::from("Modified log in text input field"),
            Self::SendRegister(_) => String::from("Register attempt"),
            Self::ValidateEmail => String::from("Validate email address"),
            Self::ResetRegisterCode => String::from("Reset email validation code"),
            Self::DoneRegistration => String::from("Successful registration"),
            Self::SendLogIn => String::from("Log In attempt"),
            Self::LoggedIn(_) => String::from("Logged in successfully"),
            Self::TabSelection(_) => String::from("Select tab"),
            Self::HandleError(_) => String::from("Handle an error"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneMessage + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Message> for AuthMessage {
    fn into(self) -> Message {
        Message::DoAction(Box::new(self))
    }
}

impl Into<Box<dyn SceneMessage + 'static>> for Box<AuthMessage> {
    fn into(self) -> Box<dyn SceneMessage + 'static> {
        Box::new(*self)
    }
}

/// A structure that represents the authentication scene.
#[derive(Clone)]
pub struct Auth {
    /// The currently active tab.
    active_tab: AuthTabIds,

    /// The data from the register form.
    register_form: RegisterForm,

    /// The data from the login form.
    log_in_form: LogInForm,

    /// The value of the e-mail validation code field.
    register_code: Option<String>,

    /// Holds possible errors with the user input.
    code_error: Option<AuthError>,
}

/// The options for the authentication page. Holds the initial [tab id](TabIds).
#[derive(Debug, Clone, Copy)]
pub struct AuthOptions {
    /// Holds the tab that should be open when the scene activates.
    active_tab: Option<AuthTabIds>,
}

impl AuthOptions {
    pub fn new(active_tab: AuthTabIds) -> Self {
        AuthOptions {
            active_tab: Some(active_tab),
        }
    }
}

impl Scene for Auth {
    type Message = AuthMessage;

    type Options = AuthOptions;

    fn new(options: Option<Self::Options>, _: &mut Globals) -> (Self, Command<Message>)
    where
        Self: Sized,
    {
        let mut auth = Auth {
            active_tab: AuthTabIds::LogIn,
            register_form: RegisterForm::default(),
            log_in_form: LogInForm::default(),
            register_code: None,
            code_error: None,
        };
        if let Some(options) = options {
            auth.apply_options(options);
        }

        (auth, Command::none())
    }

    fn get_title(&self) -> String {
        String::from("Authentication")
    }

    fn apply_options(&mut self, options: Self::Options) {
        if let Some(active_tab) = options.active_tab {
            self.active_tab = active_tab;
        }
    }

    fn update(&mut self, globals: &mut Globals, message: &Self::Message) -> Command<Message> {
        match message {
            AuthMessage::RegisterTextFieldUpdate(field) => match field {
                RegisterField::Email(email) => {
                    self.register_form.set_email(email.clone());
                }
                RegisterField::Username(username) => {
                    self.register_form.set_username(username.clone());
                }
                RegisterField::Password(password) => {
                    self.register_form.set_password(password.clone());
                }
                RegisterField::Code(code) => {
                    self.register_code = Some(code.clone());
                }
            },
            AuthMessage::LogInTextFieldUpdate(field) => match field {
                LogInField::Email(email) => {
                    self.log_in_form.set_email(email.clone());
                }
                LogInField::Password(password) => {
                    self.log_in_form.set_password(password.clone());
                }
            },
            AuthMessage::SendRegister(added_to_db) => {
                self.register_form.set_error(None);

                return if *added_to_db {
                    let mail = self.register_form.gen_register_email();
                    self.register_code = Some("".into());

                    Command::perform(async {}, |_| Message::SendSmtpMail(mail))
                } else {
                    let error = User::check_credentials(
                        self.register_form.get_username(),
                        self.register_form.get_email(),
                        self.register_form.get_password(),
                    );

                    if let Some(error) = error {
                        return self.update(globals, &AuthMessage::HandleError(error));
                    }

                    self.register_form.set_code(User::gen_register_code());

                    let mut register_form = self.register_form.clone();
                    register_form
                        .set_password(pwhash::bcrypt::hash(register_form.get_password()).unwrap());

                    match globals.get_db() {
                        Some(db) => Command::perform(
                            async move {
                                database::auth::create_user(
                                    &db,
                                    register_form.get_email().clone(),
                                    register_form.serialize(),
                                )
                                .await
                            },
                            move |res| match res {
                                Ok(_) => AuthMessage::SendRegister(true).into(),
                                Err(err) => Message::Error(err),
                            },
                        ),
                        None => Command::none(),
                    }
                };
            }
            AuthMessage::ValidateEmail => {
                let register_code = self.register_code.clone();
                let register_form = self.register_form.clone();
                self.register_code = Some("".into());
                self.code_error = None;

                if let Some(db) = globals.get_db() {
                    return Command::perform(
                        async move {
                            database::auth::validate_email(
                                &db,
                                register_form.get_email().clone(),
                                register_code.unwrap_or_default(),
                            )
                            .await
                        },
                        move |res| match res {
                            Ok(_) => AuthMessage::DoneRegistration.into(),
                            Err(err) => Message::Error(err),
                        },
                    );
                }
            }
            AuthMessage::ResetRegisterCode => {
                let db = globals.get_db().unwrap();
                let email = self.register_form.get_email().clone();
                let code = User::gen_register_code();

                self.register_form.set_code(code.clone());

                return Command::perform(
                    async move { database::auth::reset_register_code(&db, email, code).await },
                    |result| match result {
                        Ok(()) => AuthMessage::SendRegister(true).into(),
                        Err(err) => Message::Error(err),
                    },
                );
            }
            AuthMessage::DoneRegistration => {
                self.register_code = None;
                self.active_tab = AuthTabIds::LogIn;
            }
            AuthMessage::SendLogIn => {
                self.log_in_form.set_error(None);
                let log_in_form = self.log_in_form.clone();

                if let Some(db) = globals.get_db() {
                    return Command::perform(
                        async move { database::auth::login(&db, log_in_form.serialize()).await },
                        move |res| match res {
                            Ok(user) => AuthMessage::LoggedIn(user).into(),
                            Err(err) => Message::Error(err),
                        },
                    );
                }
            }
            AuthMessage::LoggedIn(user) => {
                if !user.test_password(self.log_in_form.get_password()) {
                    return self.update(
                        globals,
                        &AuthMessage::HandleError(Error::AuthError(
                            AuthError::LogInUserDoesntExist,
                        )),
                    );
                }

                if !user.is_validated() {
                    let email = user.get_email().clone();
                    let username = user.get_username().clone();

                    self.register_form.set_email(email);
                    self.register_form.set_username(username);
                    self.register_code = Some("".into());

                    let _ = self.update(globals, &AuthMessage::TabSelection(AuthTabIds::Register));

                    return self.update(globals, &AuthMessage::ResetRegisterCode);
                }

                globals.set_user(Some(user.clone()));
                let db = globals.get_db().unwrap();
                let id = user.get_id();

                return Command::perform(
                    async move { database::auth::update_user_token(&db, id).await },
                    |_| Message::ChangeScene(Scenes::Main(None)),
                );
            }
            AuthMessage::TabSelection(tab_id) => {
                self.active_tab = *tab_id;
            }
            AuthMessage::HandleError(error) => {
                if let Error::AuthError(error) = error {
                    match error {
                        AuthError::RegisterBadCode => {
                            self.code_error = Some(error.clone());
                        }
                        AuthError::LogInUserDoesntExist => {
                            self.log_in_form.set_error(error.clone());
                        }
                        AuthError::RegisterBadCredentials { .. } => {
                            self.register_form.set_error(error.clone());
                        }
                        AuthError::RegisterUserAlreadyExists => {
                            self.register_form.set_error(error.clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let register_error_text = Text::new(
            if let Some(error) = self.register_form.get_error().clone() {
                error.to_string()
            } else {
                String::from("")
            },
        );

        let log_in_error_text =
            Text::new(if let Some(error) = self.log_in_form.get_error().clone() {
                error.to_string()
            } else {
                String::from("")
            });

        let code_error_text = Text::new(if let Some(error) = self.code_error.clone() {
            error.to_string()
        } else {
            String::from("")
        });

        Container::new(Column::with_children(vec![
            Space::with_height(Length::FillPortion(1)).into(),
            Row::with_children(vec![
                Space::with_width(Length::FillPortion(1)).into(),
                Tabs::new_with_tabs(
                    vec![
                        (
                            AuthTabIds::Register,
                            Text::new("Register").into(),
                            if let Some(code) = &self.register_code {
                                Column::with_children([
                                    Text::new("A code has been sent to your email address:").into(),
                                    code_error_text.into(),
                                    TextInput::new("Input register code...", code)
                                        .on_input(|value| {
                                            AuthMessage::RegisterTextFieldUpdate(
                                                RegisterField::Code(value),
                                            )
                                            .into()
                                        })
                                        .into(),
                                    Button::new("Reset code")
                                        .on_press(AuthMessage::ResetRegisterCode.into())
                                        .into(),
                                    Button::new("Validate")
                                        .on_press(AuthMessage::ValidateEmail.into())
                                        .into(),
                                ])
                                .spacing(10.0)
                                .into()
                            } else {
                                Column::with_children([
                                    register_error_text.into(),
                                    Text::new("Email:").into(),
                                    TextInput::new(
                                        "Input email...",
                                        &*self.register_form.get_email(),
                                    )
                                    .on_input(|value| {
                                        AuthMessage::RegisterTextFieldUpdate(RegisterField::Email(
                                            value,
                                        ))
                                        .into()
                                    })
                                    .into(),
                                    Text::new("Username:").into(),
                                    TextInput::new(
                                        "Input username...",
                                        &*self.register_form.get_username(),
                                    )
                                    .on_input(|value| {
                                        AuthMessage::RegisterTextFieldUpdate(
                                            RegisterField::Username(value),
                                        )
                                        .into()
                                    })
                                    .into(),
                                    Text::new("Password:").into(),
                                    TextInput::new(
                                        "Input password...",
                                        &*self.register_form.get_password(),
                                    )
                                    .on_input(|value| {
                                        AuthMessage::RegisterTextFieldUpdate(
                                            RegisterField::Password(value),
                                        )
                                        .into()
                                    })
                                    .secure(true)
                                    .into(),
                                    if globals.get_db().is_some() {
                                        Button::new("Register")
                                            .on_press(AuthMessage::SendRegister(false).into())
                                            .into()
                                    } else {
                                        Button::new("Register").into()
                                    },
                                ])
                                .into()
                            },
                        ),
                        (
                            AuthTabIds::LogIn,
                            Text::new("Login").into(),
                            Column::with_children([
                                log_in_error_text.into(),
                                Text::new("Email:").into(),
                                TextInput::new("Input email...", &*self.log_in_form.get_email())
                                    .on_input(|value| {
                                        AuthMessage::LogInTextFieldUpdate(LogInField::Email(value))
                                            .into()
                                    })
                                    .into(),
                                Text::new("Password:").into(),
                                TextInput::new(
                                    "Input password...",
                                    &*self.log_in_form.get_password(),
                                )
                                .on_input(|value| {
                                    AuthMessage::LogInTextFieldUpdate(LogInField::Password(value))
                                        .into()
                                })
                                .secure(true)
                                .into(),
                                if globals.get_db().is_some() {
                                    Button::new("Log In")
                                        .on_press(AuthMessage::SendLogIn.into())
                                        .into()
                                } else {
                                    Button::new("Log In").into()
                                },
                            ])
                            .into(),
                        ),
                    ],
                    |tab_id| AuthMessage::TabSelection(tab_id).into(),
                )
                .width(Length::FillPortion(2))
                .selected(self.active_tab)
                .into(),
                Space::with_width(Length::FillPortion(1)).into(),
            ])
            .height(Length::FillPortion(2))
            .into(),
            Space::with_height(Length::FillPortion(1)).into(),
        ]))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message> {
        self.update(globals, &AuthMessage::HandleError(error.clone()))
    }

    fn clear(&self, _globals: &mut Globals) {}
}

use crate::errors::auth::AuthError;
use crate::errors::error::Error;
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::scenes::Scenes;
use crate::theme::Theme;
use iced::widget::{Button, Column, Container, Row, Space, Text, TextInput};
use iced::{Element, Length, Renderer, Command};
use iced_aw::{TabLabel, Tabs};
use std::any::Any;
use crate::database;
use crate::scenes::data::auth::*;
use crate::serde::Serialize;

/// Possible messages for the authentication page.
#[derive(Clone)]
enum AuthAction {
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

impl Action for AuthAction {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_name(&self) -> String {
        match self {
            AuthAction::RegisterTextFieldUpdate(_) => {
                String::from("Modified register text input field")
            }
            AuthAction::LogInTextFieldUpdate(_) => String::from("Modified log in text input field"),
            AuthAction::SendRegister(_) => String::from("Register attempt"),
            AuthAction::ValidateEmail => String::from("Validate email address"),
            AuthAction::ResetRegisterCode => String::from("Reset email validation code"),
            AuthAction::DoneRegistration => String::from("Successful registration"),
            AuthAction::SendLogIn => String::from("Log In attempt"),
            AuthAction::LoggedIn(_) => String::from("Logged in successfully"),
            AuthAction::TabSelection(_) => String::from("Select tab"),
            AuthAction::HandleError(_) => String::from("Handle an error"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn Action + 'static> {
        Box::new((*self).clone())
    }
}

impl Into<Box<dyn Action + 'static>> for Box<AuthAction> {
    fn into(self) -> Box<dyn Action + 'static> {
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

impl SceneOptions<Auth> for AuthOptions {
    fn apply_options(&self, scene: &mut Auth) {
        if let Some(active_tab) = self.active_tab {
            scene.active_tab = active_tab;
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Auth>> {
        Box::new(*self)
    }
}

impl Scene for Auth {
    fn new(
        options: Option<Box<dyn SceneOptions<Self>>>,
        _: &mut Globals,
    ) -> (Self, Command<Message>)
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
            options.apply_options(&mut auth);
        }

        (auth, Command::none())
    }

    fn get_title(&self) -> String {
        String::from("Authentication")
    }

    fn update(&mut self, globals: &mut Globals, message: Box<dyn Action>) -> Command<Message> {
        let message: &AuthAction = message
            .as_any()
            .downcast_ref::<AuthAction>()
            .expect("Panic downcasting to AuthAction");

        match message {
            AuthAction::RegisterTextFieldUpdate(field) => match field {
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
            AuthAction::LogInTextFieldUpdate(field) => match field {
                LogInField::Email(email) => {
                    self.log_in_form.set_email(email.clone());
                }
                LogInField::Password(password) => {
                    self.log_in_form.set_password(password.clone());
                }
            },
            AuthAction::SendRegister(added_to_db) => {
                self.register_form.set_error(None);

                return if *added_to_db {
                    let mail = self.register_form.gen_register_email();
                    self.register_code = Some("".into());

                    Command::perform(async {}, |_| Message::SendSmtpMail(mail))
                } else {
                    let error = User::check_credentials(
                        self.register_form.get_username(),
                        self.register_form.get_email(),
                        self.register_form.get_password()
                    );

                    if let Some(error) = error {
                        return self.update(globals, Box::new(AuthAction::HandleError(error)));
                    }

                    self.register_form.set_code(User::gen_register_code());

                    let mut register_form = self.register_form.clone();
                    register_form.set_password(pwhash::bcrypt::hash(register_form.get_password()).unwrap());

                    match globals.get_db() {
                        Some(db) => {
                            Command::perform(
                                async move {
                                    database::auth::create_user(
                                        &db,
                                        register_form.get_email().clone(),
                                        register_form.serialize()
                                    ).await
                                },
                                move |res| {
                                    match res {
                                        Ok(_) => Message::DoAction(Box::new(AuthAction::SendRegister(true))),
                                        Err(err) => Message::Error(err)
                                    }

                                })
                        }
                        None => Command::none()
                    }
                };
            }
            AuthAction::ValidateEmail => {
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
                                register_code.unwrap_or_default()
                            ).await
                        },
                        move |res| {
                            match res {
                                Ok(_) => Message::DoAction(Box::new(AuthAction::DoneRegistration)),
                                Err(err) => Message::Error(err)
                            }
                        }
                    );
                }
            }
            AuthAction::ResetRegisterCode => {
                let db = globals.get_db().unwrap();
                let email = self.register_form.get_email().clone();
                let code = User::gen_register_code();

                self.register_form.set_code(code.clone());

                return Command::perform(
                    async move {
                        database::auth::reset_register_code(
                            &db,
                            email,
                            code
                        ).await
                    },
                    |result| {
                        match result {
                            Ok(()) => Message::DoAction(Box::new(AuthAction::SendRegister(true))),
                            Err(err) => Message::Error(err)
                        }
                    }
                );
            }
            AuthAction::DoneRegistration => {
                self.register_code = None;
                self.active_tab = AuthTabIds::LogIn;
            }
            AuthAction::SendLogIn => {
                self.log_in_form.set_error(None);
                let log_in_form = self.log_in_form.clone();

                if let Some(db) = globals.get_db() {
                    return Command::perform(
                        async move {
                            database::auth::login(&db, log_in_form.serialize()).await
                        },
                        move |res| {
                            match res {
                                Ok(user) => {
                                    Message::DoAction(Box::new(AuthAction::LoggedIn(user)))
                                }
                                Err(err) => Message::Error(err)
                            }
                        }
                    );
                }
            }
            AuthAction::LoggedIn(user) => {
                if !user.test_password(self.log_in_form.get_password()) {
                    return self.update(globals, Box::new(AuthAction::HandleError(Error::AuthError(
                        AuthError::LogInUserDoesntExist,
                    ))));
                }

                if !user.is_validated() {
                    let email = user.get_email().clone();
                    let username = user.get_username().clone();

                    self.register_form.set_email(email);
                    self.register_form.set_username(username);
                    self.register_code = Some("".into());

                    let _ = self.update(globals, Box::new(AuthAction::TabSelection(
                        AuthTabIds::Register
                    )));

                    return self.update(globals, Box::new(AuthAction::ResetRegisterCode));
                }

                globals.set_user(Some(user.clone()));
                let db = globals.get_db().unwrap();
                let id = user.get_id();

                return Command::perform(
                    async move {
                        database::auth::update_user_token(&db, id).await
                    },
                    |_| Message::ChangeScene(Scenes::Main(None))
                );
            }
            AuthAction::TabSelection(tab_id) => {
                self.active_tab = *tab_id;
            }
            AuthAction::HandleError(error) => {
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
                        _ => { }
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let register_error_text = Text::new(if let Some(error) = self.register_form.get_error().clone() {
            error.to_string()
        } else {
            String::from("")
        });

        let log_in_error_text = Text::new(if let Some(error) = self.log_in_form.get_error().clone() {
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
                            TabLabel::Text("Register".into()),
                            if let Some(code) = &self.register_code {
                                Column::with_children([
                                    Text::new("A code has been sent to your email address:").into(),
                                    code_error_text.into(),
                                    TextInput::new("Input register code...", code).on_input(|value| {
                                        Message::DoAction(Box::new(
                                            AuthAction::RegisterTextFieldUpdate(
                                                RegisterField::Code(value),
                                            ),
                                        ))
                                    }).into(),
                                    Button::new("Reset code").on_press(Message::DoAction(Box::new(
                                        AuthAction::ResetRegisterCode
                                    ))).into(),
                                    Button::new("Validate").on_press(Message::DoAction(Box::new(
                                        AuthAction::ValidateEmail
                                    ))).into()
                                ])
                                    .spacing(10.0)
                                    .into()
                            } else {
                                Column::with_children([
                                    register_error_text.into(),
                                    Text::new("Email:").into(),
                                    TextInput::new("Input email...", &*self.register_form.get_email())
                                        .on_input(|value| {
                                            Message::DoAction(Box::new(
                                                AuthAction::RegisterTextFieldUpdate(
                                                    RegisterField::Email(value),
                                                ),
                                            ))
                                        }).into(),
                                    Text::new("Username:").into(),
                                    TextInput::new("Input username...", &*self.register_form.get_username())
                                        .on_input(|value| {
                                            Message::DoAction(Box::new(
                                                AuthAction::RegisterTextFieldUpdate(
                                                    RegisterField::Username(value),
                                                ),
                                            ))
                                        }).into(),
                                    Text::new("Password:").into(),
                                    TextInput::new("Input password...", &*self.register_form.get_password())
                                        .on_input(|value| {
                                            Message::DoAction(Box::new(
                                                AuthAction::RegisterTextFieldUpdate(
                                                    RegisterField::Password(value),
                                                ),
                                            ))
                                        })
                                        .secure(true)
                                        .into(),
                                    if globals.get_db().is_some() {
                                        Button::new("Register").on_press(Message::DoAction(Box::new(
                                            AuthAction::SendRegister(false)
                                        ))).into()
                                    } else {
                                        Button::new("Register").into()
                                    }
                                ])
                                    .into()
                            }
                        ),
                        (
                            AuthTabIds::LogIn,
                            TabLabel::Text("Login".into()),
                            Column::with_children([
                                log_in_error_text.into(),
                                Text::new("Email:").into(),
                                TextInput::new("Input email...", &*self.log_in_form.get_email()).on_input(
                                    |value| {
                                        Message::DoAction(Box::new(
                                            AuthAction::LogInTextFieldUpdate(LogInField::Email(
                                                value,
                                            )),
                                        ))
                                    }
                                ).into(),
                                Text::new("Password:").into(),
                                TextInput::new("Input password...", &*self.log_in_form.get_password())
                                    .on_input(|value| {
                                        Message::DoAction(Box::new(
                                            AuthAction::LogInTextFieldUpdate(LogInField::Password(
                                                value,
                                            )),
                                        ))
                                    })
                                    .secure(true)
                                    .into(),
                                if globals.get_db().is_some() {
                                    Button::new("Log In")
                                        .on_press(Message::DoAction(Box::new(AuthAction::SendLogIn)))
                                        .into()
                                } else {
                                    Button::new("Log In").into()
                                }
                            ])
                                .into()
                        )
                    ],
                    |tab_id| Message::DoAction(Box::new(AuthAction::TabSelection(tab_id)))
                )
                    .width(Length::FillPortion(2))
                    .set_active_tab(&self.active_tab)
                    .into(),
                Space::with_width(Length::FillPortion(1)).into()
            ])
                .height(Length::FillPortion(2))
                .into(),
            Space::with_height(Length::FillPortion(1)).into()
        ]))
            .center_x()
            .center_y()
            .into()
    }

    fn get_error_handler(&self, error: Error) -> Box<dyn Action> {
        Box::new(AuthAction::HandleError(error))
    }

    fn clear(&self) {}
}

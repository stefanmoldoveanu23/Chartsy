use crate::database;
use crate::scene::{Globals, Message, Scene, SceneMessage};
use crate::scenes::data::auth::*;
use crate::scenes::scenes::Scenes;
use crate::utils::errors::{AuthError, Error};
use crate::utils::serde::Serialize;
use crate::utils::theme::Theme;
use iced::widget::Column;
use iced::{Command, Element, Renderer};
use std::any::Any;

use super::services;

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

impl Auth {
    fn send_register(&mut self, globals: &mut Globals) -> Command<Message> {
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
        register_form.set_password(pwhash::bcrypt::hash(register_form.get_password()).unwrap());

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
    }

    fn validate_email(&mut self, globals: &mut Globals) -> Command<Message> {
        let register_code = self.register_code.clone();
        let register_form = self.register_form.clone();
        self.register_code = Some("".into());
        self.code_error = None;

        if let Some(db) = globals.get_db() {
            Command::perform(
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
            )
        } else {
            Command::none()
        }
    }

    pub fn logged_in(&mut self, user: &User, globals: &mut Globals) -> Command<Message> {
        if !user.test_password(self.log_in_form.get_password()) {
            return self.update(
                globals,
                &AuthMessage::HandleError(Error::AuthError(AuthError::LogInUserDoesntExist)),
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
                    self.send_register(globals)
                };
            }
            AuthMessage::ValidateEmail => {
                return self.validate_email(globals);
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
                return self.logged_in(user, globals);
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
        let register_tab = services::auth::register_tab(
            &self.register_form,
            &self.register_code,
            &self.code_error,
            globals,
        );

        let log_in_tab = services::auth::log_in_tab(&self.log_in_form, globals);

        let tabs = services::auth::tabs(register_tab, log_in_tab, self.active_tab);

        Column::with_children(vec![self.title_element(), tabs]).into()
    }

    fn handle_error(&mut self, globals: &mut Globals, error: &Error) -> Command<Message> {
        self.update(globals, &AuthMessage::HandleError(error.clone()))
    }

    fn clear(&self, _globals: &mut Globals) {}
}

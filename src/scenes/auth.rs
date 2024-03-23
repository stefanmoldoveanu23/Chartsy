use crate::config;
use crate::errors::auth::AuthError;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::scenes::Scenes;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;
use iced::widget::{Button, Column, Container, Row, Space, Text, TextInput};
use iced::{Element, Length, Renderer, Command};
use iced_aw::{TabLabel, Tabs};
use lettre::message::MultiPart;
use mongodb::bson::{Bson, doc, Document, Uuid, UuidRepresentation};
use rand::{Rng};
use regex::Regex;
use std::any::Any;
use crate::mongo;

/// User account registration fields.
#[derive(Clone)]
enum RegisterField {
    Email(String),
    Username(String),
    Password(String),
    Code(String),
}

/// User account authentication fields.
#[derive(Clone)]
enum LogInField {
    Email(String),
    Password(String),
}

/// Possible messages for the authentication page.
#[derive(Clone)]
enum AuthAction {
    None,

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
            AuthAction::None => String::from("None"),
            AuthAction::RegisterTextFieldUpdate(_) => {
                String::from("Modified register text input field")
            }
            AuthAction::LogInTextFieldUpdate(_) => String::from("Modified log in text input field"),
            AuthAction::SendRegister(_) => String::from("Register attempt"),
            AuthAction::ValidateEmail => String::from("Validate email address"),
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

/// Structure for the user data.
#[derive(Default, Debug, Clone)]
pub struct User {
    /// The database id of the [User].
    id: Uuid,

    /// The e-mail address of the [User].
    email: String,

    /// The username of the [User].
    username: String,

    /// The hashed password of the [User].
    password_hash: String,
}

impl User {
    /// Returns the id of the [user](User).
    pub fn get_id(&self) -> Uuid {
        self.id.clone()
    }

    /// Returns the email of the [user](User).
    pub fn get_email(&self) -> String {
        self.email.clone()
    }

    /// Returns the username of the [user](User).
    pub fn get_username(&self) -> String {
        self.username.clone()
    }

    /// Tests whether the given password is the same as the [users](User).
    pub fn test_password(&self, password: &String) -> bool {
        pwhash::bcrypt::verify(password, &*self.password_hash)
    }
}

impl Deserialize<Document> for User {
    fn deserialize(document: Document) -> Self
    where
        Self: Sized,
    {
        let mut user: User = User::default();

        if let Some(Bson::Binary(bin)) = document.get("id") {
            if let Ok(uuid) = bin.to_uuid_with_representation(UuidRepresentation::Standard) {
                user.id = uuid;
            }
        }
        if let Ok(email) = document.get_str("email") {
            user.email = email.into();
        }
        if let Ok(username) = document.get_str("username") {
            user.username = username.into();
        }
        if let Ok(password) = document.get_str("password") {
            user.password_hash = password.into();
        }

        user
    }
}

/// The fields of a registration form.
#[derive(Default, Clone)]
pub struct RegisterForm {
    /// The value of the e-mail field.
    email: String,

    /// The value of the username field.
    username: String,

    /// The value of the password field.
    password: String,

    /// The value of the e-mail validation code.
    code: String,

    /// Holds possible errors with the user input.
    error: Option<AuthError>,
}

impl Serialize<Document> for RegisterForm {
    fn serialize(&self) -> Document {
        doc! {
            "id": Uuid::new(),
            "email": self.email.clone(),
            "username": self.username.clone(),
            "password": self.password.clone(),
            "code": self.code.clone(),
            "validated": false,
        }
    }
}

/// The fields of an authentication form.
#[derive(Default, Clone)]
struct LogInForm {
    /// The e-mail field of the login form.
    email: String,

    /// The password field of the login form.
    password: String,

    /// Holds possible errors with the user input.
    error: Option<AuthError>,
}

impl Serialize<Document> for LogInForm {
    fn serialize(&self) -> Document {
        doc! {
            "email": self.email.clone(),
            "validated": true,
        }
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

impl Auth {
    /// [Mongo request chain function](MongoRequestType::Chain) that checks whether a user already exists.
    fn check_user_exists(res: MongoResponse, document: Document) -> Result<MongoRequest, Error> {
        match res {
            MongoResponse::Get(res) => {
                if res.len() > 0 {
                    Err(Error::AuthError(AuthError::RegisterUserAlreadyExists))
                } else {
                    Ok(MongoRequest::new(
                        "users".into(),
                        MongoRequestType::Insert{
                            documents: vec![document],
                            options: None
                        },
                    ))
                }
            }
            _ => Err(Error::DebugError(DebugError::new(
                "Error in chain request typing when registering user!".into(),
            ))),
        }
    }

    /// [Mongo request chain function](MongoRequestType::Chain) that sets the validated field as true for a given user.
    fn set_email_validated(res: MongoResponse, document: Document) -> Result<MongoRequest, Error> {
        let email = document.get_str("email");
        if email.is_err() {
            return Err(Error::DebugError(DebugError::new(
                "Error in chain setting email validated; no email provided!".into(),
            )));
        }

        match res {
            MongoResponse::Get(res) => {
                if res.len() > 0 {
                    Ok(MongoRequest::new(
                        "users".into(),
                        MongoRequestType::Update {
                            filter: doc! {
                                "email": email.unwrap(),
                            },
                            update: doc! {
                                "$set": {
                                "validated": true,
                                },
                            },
                            options: None,
                        },
                    ))
                } else {
                    Err(Error::AuthError(AuthError::RegisterBadCode))
                }
            }
            _ => Err(Error::DebugError(DebugError::new(
                "Error in chain request typing when registering user!".into(),
            ))),
        }
    }

    /// Checks the provided credentials in the registration form; if there is an issue, then it will return the error;
    /// otherwise, it will return [None].
    fn check_credentials(&self) -> Option<Error> {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        let username_regex = Regex::new(r"^[a-zA-Z0-9]+$").unwrap();

        let mut password_good = true;
        if self.register_form.password.len() < 8 {
            password_good = false;
        }

        let lowercase_regex = Regex::new(r"[a-z]").unwrap();
        let uppercase_regex = Regex::new(r"[A-Z]").unwrap();
        let digit_regex = Regex::new(r"\d").unwrap();
        let symbol_regex = Regex::new(r"[^\w\s]").unwrap();
        if !lowercase_regex.is_match(&*self.register_form.password.clone())
            | !uppercase_regex.is_match(&*self.register_form.password.clone())
            | !digit_regex.is_match(&*self.register_form.password.clone())
            | !symbol_regex.is_match(&*self.register_form.password.clone())
        {
            password_good = false;
        }

        let email_good = email_regex.is_match(&*self.register_form.email.clone());
        let username_good = username_regex.is_match(&*self.register_form.username.clone());

        if !email_good | !username_good | !password_good {
            Some(Error::AuthError(AuthError::RegisterBadCredentials {
                email: !email_good,
                username: !username_good,
                password: !password_good,
            }))
        } else {
            None
        }
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
                    self.register_form.email = email.clone();
                }
                RegisterField::Username(username) => {
                    self.register_form.username = username.clone();
                }
                RegisterField::Password(password) => {
                    self.register_form.password = password.clone();
                }
                RegisterField::Code(code) => {
                    self.register_code = Some(code.clone());
                }
            },
            AuthAction::LogInTextFieldUpdate(field) => match field {
                LogInField::Email(email) => {
                    self.log_in_form.email = email.clone();
                }
                LogInField::Password(password) => {
                    self.log_in_form.password = password.clone();
                }
            },
            AuthAction::SendRegister(added_to_db) => {
                self.register_form.error = None;

                return if *added_to_db {
                    let mail = lettre::Message::builder()
                        .from(format!("Chartsy <{}>", config::email_address()).parse().unwrap())
                        .to(format!("{} <{}>", self.register_form.username.clone(), self.register_form.email.clone()).parse().unwrap())
                        .subject("Code validation for Chartsy account")
                        .multipart(MultiPart::alternative_plain_html(
                            String::from(format!("Use the following code to validate your email address:\n{}", self.register_form.code)),
                            String::from(format!("<p>Use the following code to validate your email address:</p><h1>{}</h1>", self.register_form.code))
                        )).unwrap();

                    self.register_code = Some("".into());

                    Command::perform(async {}, |_| Message::SendSmtpMail(mail))
                } else {
                    let error = self.check_credentials();

                    if let Some(error) = error {
                        return self.update(globals, Box::new(AuthAction::HandleError(error)));
                    }

                    let mut rng = rand::thread_rng();
                    self.register_form.code =
                        (0..6).map(|_| rng.gen_range(0..=9).to_string()).collect();

                    let mut register_form = self.register_form.clone();
                    register_form.password = pwhash::bcrypt::hash(register_form.password).unwrap();

                    match globals.get_db() {
                        Some(db) => {
                            Command::perform(
                                async move {
                                    MongoRequest::send_requests(
                                        db,
                                        vec![MongoRequest::new(
                                            "users".into(),
                                            MongoRequestType::Chain(
                                                Box::new(MongoRequestType::Get{
                                                    filter: doc! {
                                                        "email": register_form.email.clone(),
                                                    },
                                                    options: None
                                                }),
                                                vec![(register_form.serialize(), Auth::check_user_exists)],
                                            ),
                                        )]
                                    ).await
                                },
                                move |res| {
                                    match res {
                                        Ok(res) => {
                                            if let Some(MongoResponse::Insert(_)) = res.get(0) {
                                                Message::DoAction(Box::new(AuthAction::SendRegister(true)))
                                            } else {
                                                Message::DoAction(Box::new(AuthAction::HandleError(Error::DebugError(
                                                    DebugError::new("Wrong chain final typing!".into()),
                                                ))))
                                            }
                                        }
                                        Err(message) => message
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
                        async {
                            MongoRequest::send_requests(
                                db,
                                vec![MongoRequest::new(
                                    "users".into(),
                                    MongoRequestType::Chain(
                                        Box::new(MongoRequestType::Get{
                                            filter: doc! {
                                                "email": register_form.email.clone(),
                                                "code": register_code,
                                            },
                                            options: None
                                        }),
                                        vec![(
                                            doc! {"email": register_form.email},
                                            Auth::set_email_validated,
                                        )],
                                    ),
                                )]
                            ).await
                        },
                        move |res| {
                            match res {
                                Ok(res) => {
                                    if let Some(MongoResponse::Update(_)) = res.get(0) {
                                        Message::DoAction(Box::new(AuthAction::DoneRegistration))
                                    } else {
                                        Message::DoAction(Box::new(AuthAction::HandleError(Error::DebugError(
                                            DebugError::new("Wrong chain final typing!".into()),
                                        ))))
                                    }
                                }
                                Err(message) => message
                            }
                        }
                    );
                }
            }
            AuthAction::DoneRegistration => {
                self.register_code = None;
                self.active_tab = AuthTabIds::LogIn;
            }
            AuthAction::SendLogIn => {
                self.log_in_form.error = None;
                let log_in_form = self.log_in_form.clone();

                if let Some(db) = globals.get_db() {
                    return Command::perform(
                        async move {
                            MongoRequest::send_requests(
                                db,
                                vec![MongoRequest::new(
                                    "users".into(),
                                    MongoRequestType::Get{
                                        filter: log_in_form.serialize(),
                                        options: None
                                    },
                                )]
                            ).await
                        },
                        move |res| {
                            match res {
                                Ok(res) => {
                                    if let Some(MongoResponse::Get(cursor)) = res.get(0) {
                                        if let Some(document) = cursor.get(0) {
                                            Message::DoAction(Box::new(AuthAction::LoggedIn(User::deserialize(
                                                document.clone(),
                                            ))))
                                        } else {
                                            Message::DoAction(Box::new(AuthAction::HandleError(Error::AuthError(
                                                AuthError::LogInUserDoesntExist,
                                            ))))
                                        }
                                    } else {
                                        Message::DoAction(Box::new(AuthAction::None))
                                    }
                                }
                                Err(message) => message
                            }
                        }
                    );
                }
            }
            AuthAction::LoggedIn(user) => {
                if !user.test_password(&self.log_in_form.password) {
                    return self.update(globals, Box::new(AuthAction::HandleError(Error::AuthError(
                        AuthError::LogInUserDoesntExist,
                    ))));
                }

                globals.set_user(Some(user.clone()));
                let db = globals.get_db().unwrap();
                let id = user.id;

                return Command::perform(
                    async move {
                        mongo::update_user_token(db, id).await
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
                            self.log_in_form.error = Some(error.clone());
                        }
                        AuthError::RegisterBadCredentials { .. } => {
                            self.register_form.error = Some(error.clone());
                        }
                        AuthError::RegisterUserAlreadyExists => {
                            self.register_form.error = Some(error.clone());
                        }
                    }
                }
            }
            AuthAction::None => {}
        }

        Command::none()
    }

    fn view(&self, globals: &Globals) -> Element<'_, Message, Theme, Renderer> {
        let register_error_text = Text::new(if let Some(error) = self.register_form.error.clone() {
            error.to_string()
        } else {
            String::from("")
        });

        let log_in_error_text = Text::new(if let Some(error) = self.log_in_form.error.clone() {
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
                                    Button::new("Validate").on_press(Message::DoAction(Box::new(
                                        AuthAction::ValidateEmail
                                    ))).into()
                                ])
                                    .into()
                            } else {
                                Column::with_children([
                                    register_error_text.into(),
                                    Text::new("Email:").into(),
                                    TextInput::new("Input email...", &*self.register_form.email)
                                        .on_input(|value| {
                                            Message::DoAction(Box::new(
                                                AuthAction::RegisterTextFieldUpdate(
                                                    RegisterField::Email(value),
                                                ),
                                            ))
                                        }).into(),
                                    Text::new("Username:").into(),
                                    TextInput::new("Input username...", &*self.register_form.username)
                                        .on_input(|value| {
                                            Message::DoAction(Box::new(
                                                AuthAction::RegisterTextFieldUpdate(
                                                    RegisterField::Username(value),
                                                ),
                                            ))
                                        }).into(),
                                    Text::new("Password:").into(),
                                    TextInput::new("Input password...", &*self.register_form.password)
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
                                TextInput::new("Input email...", &*self.log_in_form.email).on_input(
                                    |value| {
                                        Message::DoAction(Box::new(
                                            AuthAction::LogInTextFieldUpdate(LogInField::Email(
                                                value,
                                            )),
                                        ))
                                    }
                                ).into(),
                                Text::new("Password:").into(),
                                TextInput::new("Input password...", &*self.log_in_form.password)
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AuthTabIds {
    Register,
    LogIn,
}

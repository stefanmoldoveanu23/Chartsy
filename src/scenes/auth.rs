use crate::config::EMAIL_ADDRESS;
use crate::errors::auth::AuthError;
use crate::errors::debug::DebugError;
use crate::errors::error::Error;
use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::scenes::Scenes;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;
use iced::widget::{
    button, column, container, horizontal_space, row, text, text_input, vertical_space,
};
use iced::{Element, Length, Renderer};
use iced_aw::{TabLabel, Tabs};
use iced_runtime::Command;
use lettre::message::MultiPart;
use mongodb::bson::{Bson, doc, Document, Uuid, UuidRepresentation};
use rand::Rng;
use regex::Regex;
use std::any::Any;

/// User account registration fields; possible values are:
/// - [Email(String)](RegisterField::Email);
/// - [Username(String)](RegisterField::Username);
/// - [Password(String)](RegisterField::Password);
/// - [Code(String)](RegisterField::Code), which refers to the email validation code.
#[derive(Clone)]
enum RegisterField {
    Email(String),
    Username(String),
    Password(String),
    Code(String),
}

/// User account authentication fields; possible values are:
/// - [Email(String)](LogInField::Email);
/// - [Password(String)](LoginField::Password).
#[derive(Clone)]
enum LogInField {
    Email(String),
    Password(String),
}

/// Possible messages for the authentication page:
/// - [RegisterTextFieldUpdate(RegisterField)](AuthAction::RegisterTextFieldUpdate), for when a field from the registration
/// form has been modified;
/// - [LogInTextFieldUpdate(LogInField)](AuthAction::LogInTextFieldUpdate), for when a field from the authentication
/// form has been modified;
/// - [SendRegister(bool)](AuthAction::SendRegister), that handles registration; when the parameter is false, it adds
/// the user data to the database; when it is true, it sends an email with a verification code;
/// - [ValidateEmail](AuthAction::ValidateEmail), that handles email validation attempts;
/// - [DoneRegistration](AuthAction::DoneRegistration), that signals when a new user has been successfully registered;
/// - [SendLogIn](AuthAction::SendLogIn), that attempts to authenticate a user;
/// - [LoggedIn(User)](AuthAction::LoggedIn), that signals when a user has been successfully authenticated;
/// - [TabSelection(TabIds)](AuthAction::TabSelection), that changes the currently selected tab;
/// - [HandleError(Error)](AuthAction::HandleError), which handles errors.
#[derive(Clone)]
enum AuthAction {
    None,
    RegisterTextFieldUpdate(RegisterField),
    LogInTextFieldUpdate(LogInField),
    SendRegister(bool),
    ValidateEmail,
    DoneRegistration,
    SendLogIn,
    LoggedIn(User),
    TabSelection(AuthTabIds),
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

/// Structure for the user data: the user id, email, username, and the hash of the password.
#[derive(Default, Debug, Clone)]
pub struct User {
    id: Uuid,
    email: String,
    username: String,
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

/// The fields of a registration form: email, username, password, email verification code, and optional errors.
#[derive(Default, Clone)]
pub struct RegisterForm {
    email: String,
    username: String,
    password: String,
    code: String,
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

/// The fields of an authentication form: email, password and optional errors.
#[derive(Default, Clone)]
struct LogInForm {
    email: String,
    password: String,
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

/// Model for authentication scene. Holds the [id](TabIds) of the currently active tab, the data for the [registration form](RegisterForm),
/// the data for the [authentication form](LogInForm), and the user input value of the email verification code with optional errors.
#[derive(Clone)]
pub struct Auth {
    active_tab: AuthTabIds,
    register_form: RegisterForm,
    log_in_form: LogInForm,
    register_code: Option<String>,
    code_error: Option<AuthError>,
}

/// The options for the authentication page. Holds the initial [tab id](TabIds).
#[derive(Debug, Clone, Copy)]
pub struct AuthOptions {
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
                        MongoRequestType::Insert(vec![document]),
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
                        MongoRequestType::Update(
                            doc! {
                                "email": email.unwrap(),
                            },
                            doc! {
                                "$set": {
                                    "validated": true,
                                },
                            },
                        ),
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
                        .from(format!("Chartsy <{}>", EMAIL_ADDRESS).parse().unwrap())
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
                                                Box::new(MongoRequestType::Get(
                                                    doc! {
                                                        "email": register_form.email.clone(),
                                                })),
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
                                        Box::new(MongoRequestType::Get(doc! {
                                    "email": register_form.email.clone(),
                                    "code": register_code,
                                })),
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
                                    MongoRequestType::Get(log_in_form.serialize()),
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

                return Command::perform(async {}, |_| Message::ChangeScene(Scenes::Main(None)));
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

    fn view(&self, globals: &Globals) -> Element<'_, Message, Renderer<Theme>> {
        let register_error_text = text(if let Some(error) = self.register_form.error.clone() {
            error.to_string()
        } else {
            String::from("")
        });

        let log_in_error_text = text(if let Some(error) = self.log_in_form.error.clone() {
            error.to_string()
        } else {
            String::from("")
        });

        let code_error_text = text(if let Some(error) = self.code_error.clone() {
            error.to_string()
        } else {
            String::from("")
        });

        container(column![
            vertical_space(Length::FillPortion(1)),
            row![
                horizontal_space(Length::FillPortion(1)),
                Tabs::with_tabs(
                    vec![
                        (
                            AuthTabIds::Register,
                            TabLabel::Text("Register".into()),
                            if let Some(code) = &self.register_code {
                                column![
                                    text("A code has been sent to your email address:"),
                                    code_error_text,
                                    text_input("Input register code...", code).on_input(|value| {
                                        Message::DoAction(Box::new(
                                            AuthAction::RegisterTextFieldUpdate(
                                                RegisterField::Code(value),
                                            ),
                                        ))
                                    }),
                                    button("Validate").on_press(Message::DoAction(Box::new(
                                        AuthAction::ValidateEmail
                                    )))
                                ]
                                .into()
                            } else {
                                column![
                                    register_error_text,
                                    text("Email:"),
                                    text_input("Input email...", &*self.register_form.email)
                                        .on_input(|value| {
                                            Message::DoAction(Box::new(
                                                AuthAction::RegisterTextFieldUpdate(
                                                    RegisterField::Email(value),
                                                ),
                                            ))
                                        }),
                                    text("Username:"),
                                    text_input("Input username...", &*self.register_form.username)
                                        .on_input(|value| {
                                            Message::DoAction(Box::new(
                                                AuthAction::RegisterTextFieldUpdate(
                                                    RegisterField::Username(value),
                                                ),
                                            ))
                                        }),
                                    text("Password:"),
                                    text_input("Input password...", &*self.register_form.password)
                                        .on_input(|value| {
                                            Message::DoAction(Box::new(
                                                AuthAction::RegisterTextFieldUpdate(
                                                    RegisterField::Password(value),
                                                ),
                                            ))
                                        })
                                        .password(),
                                    if globals.get_db().is_some() {
                                        button("Register").on_press(Message::DoAction(Box::new(
                                            AuthAction::SendRegister(false)
                                        )))
                                    } else {
                                        button("Register")
                                    }
                                ]
                                .into()
                            }
                        ),
                        (
                            AuthTabIds::LogIn,
                            TabLabel::Text("Login".into()),
                            column![
                                log_in_error_text,
                                text("Email:"),
                                text_input("Input email...", &*self.log_in_form.email).on_input(
                                    |value| {
                                        Message::DoAction(Box::new(
                                            AuthAction::LogInTextFieldUpdate(LogInField::Email(
                                                value,
                                            )),
                                        ))
                                    }
                                ),
                                text("Password:"),
                                text_input("Input password...", &*self.log_in_form.password)
                                    .on_input(|value| {
                                        Message::DoAction(Box::new(
                                            AuthAction::LogInTextFieldUpdate(LogInField::Password(
                                                value,
                                            )),
                                        ))
                                    })
                                    .password(),
                                if globals.get_db().is_some() {
                                    button("Log In")
                                        .on_press(Message::DoAction(Box::new(AuthAction::SendLogIn)))
                                } else {
                                    button("Log In")
                                }
                            ]
                            .into()
                        )
                    ],
                    |tab_id| Message::DoAction(Box::new(AuthAction::TabSelection(tab_id)))
                )
                .width(Length::FillPortion(2))
                .set_active_tab(&self.active_tab),
                horizontal_space(Length::FillPortion(1))
            ]
            .height(Length::FillPortion(2)),
            vertical_space(Length::FillPortion(1))
        ])
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

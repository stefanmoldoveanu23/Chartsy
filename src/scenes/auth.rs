use std::any::Any;
use iced::{Element, Length, Renderer};
use iced::widget::{column, text, text_input, button, container, vertical_space, row, horizontal_space};
use iced_aw::{TabLabel, Tabs};
use iced_runtime::Command;
use mongodb::bson::{doc, Document};
use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::scene::{Action, Globals, Message, Scene, SceneOptions};
use crate::scenes::scenes::Scenes;
use crate::serde::{Deserialize, Serialize};
use crate::theme::Theme;

#[derive(Clone)]
enum RegisterField {
    Email(String),
    Username(String),
    Password(String),
}

#[derive(Clone)]
enum LogInField {
    Email(String),
    Password(String),
}

#[derive(Clone)]
enum AuthAction {
    None,
    RegisterTextFieldUpdate(RegisterField),
    LogInTextFieldUpdate(LogInField),
    SendRegister,
    SendLogIn,
    LoggedIn(User),
    TabSelection(TabIds),
}

impl Action for AuthAction {
    fn as_any(&self) -> &dyn Any { self }

    fn get_name(&self) -> String {
        match self {
            AuthAction::None => String::from("None"),
            AuthAction::RegisterTextFieldUpdate(_) => String::from("Modified register text input field"),
            AuthAction::LogInTextFieldUpdate(_) => String::from("Modified log in text input field"),
            AuthAction::SendRegister => String::from("Register attempt"),
            AuthAction::SendLogIn => String::from("Log In attempt"),
            AuthAction::LoggedIn(_) => String::from("Logged in successfully"),
            AuthAction::TabSelection(_) => String::from("Select tab"),
        }
    }

    fn boxed_clone(&self) -> Box<dyn Action + 'static> { Box::new((*self).clone()) }
}

impl Into<Box<dyn Action + 'static>> for Box<AuthAction> {
    fn into(self) -> Box<dyn Action + 'static> { Box::new(*self) }
}

#[derive(Default, Debug, Clone)]
pub struct User {
    email: String,
    username: String,
    password_hash: String,
}

impl User {
    pub fn get_email(&self) -> String { self.email.clone() }
    pub fn get_username(&self) -> String { self.username.clone() }

    pub fn test_password(&self, password: &String) -> bool {
        pwhash::bcrypt::verify(password, &*self.password_hash)
    }
}

impl Deserialize for User {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut user :User= User::default();

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

#[derive(Default, Clone)]
struct RegisterForm {
    email: String,
    username: String,
    password: String,
}

impl Serialize for RegisterForm {
    fn serialize(&self) -> Document {
        doc! {
            "email": self.email.clone(),
            "username": self.username.clone(),
            "password": self.password.clone(),
        }
    }
}

#[derive(Default, Clone)]
struct LogInForm {
    email: String,
    password: String,
}

impl Serialize for LogInForm {
    fn serialize(&self) -> Document {
        doc! {
            "email": self.email.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Auth {
    active_tab: TabIds,
    register_form: RegisterForm,
    log_in_form: LogInForm,
    globals: Globals,
}

#[derive(Debug, Clone, Copy)]
pub struct AuthOptions {
    active_tab: Option<TabIds>,
}

impl AuthOptions {
    pub fn new(active_tab: TabIds) -> Self {
        AuthOptions { active_tab: Some(active_tab) }
    }
}

impl SceneOptions<Auth> for AuthOptions {
    fn apply_options(&self, scene: &mut Auth) {
        if let Some(active_tab) = self.active_tab {
            scene.active_tab = active_tab;
        }
    }

    fn boxed_clone(&self) -> Box<dyn SceneOptions<Auth>> { Box::new(*self) }
}

impl Scene for Auth {
    fn new(options: Option<Box<dyn SceneOptions<Self>>>, globals: Globals) -> (Self, Command<Message>) where Self: Sized {
        let mut auth = Auth { active_tab: TabIds::LogIn, register_form: RegisterForm::default(), log_in_form: LogInForm::default(), globals };
        if let Some(options) = options {
            options.apply_options(&mut auth);
        }

        (auth, Command::none())
    }

    fn get_title(&self) -> String { String::from("Authentication") }

    fn update(&mut self, message: Box<dyn Action>) -> Command<Message> {
        let message :&AuthAction= message.as_any().downcast_ref::<AuthAction>().expect("Panic downcasting to AuthAction");

        match message {
            AuthAction::RegisterTextFieldUpdate(field) => {
                match field {
                    RegisterField::Email(email) => {
                        self.register_form.email = email.clone();
                    }
                    RegisterField::Username(username) => {
                        self.register_form.username = username.clone();
                    }
                    RegisterField::Password(password) => {
                        self.register_form.password = password.clone();
                    }
                }
            }
            AuthAction::LogInTextFieldUpdate(field) => {
                match field {
                    LogInField::Email(email) => {
                        self.log_in_form.email = email.clone();
                    }
                    LogInField::Password(password) => {
                        self.log_in_form.password = password.clone();
                    }
                }
            }
            AuthAction::SendRegister => {
                let mut register_form = self.register_form.clone();
                register_form.password = pwhash::bcrypt::hash(register_form.password).unwrap();

                return Command::perform(
                    async { },
                    move |_| {
                        Message::SendMongoRequests(
                            vec![
                                MongoRequest::new(
                                    "users".into(),
                                    MongoRequestType::Insert(
                                        vec![
                                            register_form.serialize()
                                        ]
                                    )
                                )
                            ],
                            |res| {
                                if let Some(MongoResponse::Insert(_insert_result)) = res.get(0) {
                                    Box::new(AuthAction::TabSelection(TabIds::LogIn))
                                } else {
                                    Box::new(AuthAction::None)
                                }
                            }
                        )
                    }
                );
            }
            AuthAction::SendLogIn => {
                let log_in_form = self.log_in_form.clone();

                return Command::perform(
                    async { },
                    move |_| {
                        Message::SendMongoRequests(
                            vec![
                                MongoRequest::new(
                                    "users".into(),
                                    MongoRequestType::Get(log_in_form.serialize())
                                )
                            ],
                            |res| {
                                if let Some(MongoResponse::Get(cursor)) = res.get(0) {
                                    if let Some(document) = cursor.get(0) {
                                        Box::new(AuthAction::LoggedIn(User::deserialize(document.clone())))
                                    } else {
                                        Box::new(AuthAction::None)
                                    }
                                } else {
                                    Box::new(AuthAction::None)
                                }
                            }
                        )
                    }
                )
            }
            AuthAction::LoggedIn(user) => {
                if !user.test_password(&self.log_in_form.password) {
                    return Command::none()
                }

                self.globals.set_user(Some(user.clone()));

                let globals = self.globals.clone();

                return Command::batch(
                    vec![
                        Command::perform(
                            async { },
                            |_| {
                                Message::UpdateGlobals(globals)
                            }
                        ),
                        Command::perform(
                            async { },
                            |_| {
                                Message::ChangeScene(Scenes::Main(None))
                            }
                        )
                    ]
                );
            }
            AuthAction::TabSelection(tab_id) => {
                self.active_tab = *tab_id;
            }
            _ => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Message, Renderer<Theme>> {
        container(
            column![
                vertical_space(Length::FillPortion(1)),
                row![
                    horizontal_space(Length::FillPortion(1)),
                    Tabs::with_tabs(
                        vec![
                            (
                                TabIds::Register,
                                TabLabel::Text("Register".into()),
                                column![
                                    text("Email:"),
                                    text_input(
                                        "Input email...",
                                        &*self.register_form.email
                                    ).on_input(|value| {Message::DoAction(Box::new(AuthAction::RegisterTextFieldUpdate(RegisterField::Email(value))))}),
                                    text("Username:"),
                                    text_input(
                                        "Input username...",
                                        &*self.register_form.username
                                    ).on_input(|value| {Message::DoAction(Box::new(AuthAction::RegisterTextFieldUpdate(RegisterField::Username(value))))}),
                                    text("Password:"),
                                    text_input(
                                        "Input password...",
                                        &*self.register_form.password
                                    ).on_input(|value| {Message::DoAction(Box::new(AuthAction::RegisterTextFieldUpdate(RegisterField::Password(value))))})
                                        .password(),
                                    button("Register").on_press(Message::DoAction(Box::new(AuthAction::SendRegister)))
                                ]
                                    .into()
                            ),
                            (
                                TabIds::LogIn,
                                TabLabel::Text("Login".into()),
                                column![
                                    text("Email:"),
                                    text_input(
                                        "Input email...",
                                        &*self.log_in_form.email
                                    ).on_input(|value| {Message::DoAction(Box::new(AuthAction::LogInTextFieldUpdate(LogInField::Email(value))))}),
                                    text("Password:"),
                                    text_input(
                                        "Input password...",
                                        &*self.log_in_form.password
                                    ).on_input(|value| {Message::DoAction(Box::new(AuthAction::LogInTextFieldUpdate(LogInField::Password(value))))})
                                        .password(),
                                    button("Log In").on_press(Message::DoAction(Box::new(AuthAction::SendLogIn)))
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
            ]
        )
            .center_x()
            .center_y()
            .into()
    }

    fn update_globals(&mut self, _globals: Globals) { }

    fn clear(&self) { }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TabIds {
    Register,
    LogIn,
}
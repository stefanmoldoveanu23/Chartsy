use crate::config;
use crate::errors::auth::AuthError;
use crate::errors::error::Error;
use crate::utils::serde::{Deserialize, Serialize};
use lettre::message::MultiPart;
use lettre::Message;
use mongodb::bson::spec::BinarySubtype;
use mongodb::bson::{doc, Binary, Bson, DateTime, Document, Uuid, UuidRepresentation};
use rand::{random, Rng};
use regex::Regex;
use sha2::{Digest, Sha256};

/// User account registration fields.
#[derive(Clone)]
pub enum RegisterField {
    Email(String),
    Username(String),
    Password(String),
    Code(String),
}

/// User account authentication fields.
#[derive(Clone)]
pub enum LogInField {
    Email(String),
    Password(String),
}

/// User roles.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Admin,
    #[default]
    User,
}

impl Into<i32> for Role {
    fn into(self) -> i32 {
        match self {
            Role::Admin => 0,
            Role::User => 1,
        }
    }
}

impl From<i32> for Role {
    fn from(value: i32) -> Self {
        match value {
            0 => Role::Admin,
            _ => Role::User,
        }
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

    /// The tag of the [User].
    user_tag: String,

    /// The role of the [User].
    role: Role,

    /// The hashed password of the [User].
    password_hash: String,

    /// Tells whether the e-mail address has been validated.
    validated: bool,

    /// Tells whether the user has a profile picture set.
    profile_picture: bool,
}

impl User {
    /// Returns the id of the [user](User).
    pub fn get_id(&self) -> Uuid {
        self.id
    }

    /// Returns the email of the [user](User).
    pub fn get_email(&self) -> &String {
        &self.email
    }

    /// Returns the username of the [user](User).
    pub fn get_username(&self) -> &String {
        &self.username
    }

    /// Returns the tag of the [user](User).
    pub fn get_user_tag(&self) -> &String {
        &self.user_tag
    }

    pub fn get_role(&self) -> &Role {
        &self.role
    }

    /// Sets the username of the [user](User).
    pub fn set_username(&mut self, username: impl Into<String>) {
        self.username = username.into();
    }

    /// Sets the tag of the [user](User).
    pub fn set_user_tag(&mut self, user_tag: impl Into<String>) {
        self.user_tag = user_tag.into();
    }

    /// Tests whether the given password is the same as the [users](User).
    pub fn test_password(&self, password: &String) -> bool {
        pwhash::bcrypt::verify(password, &*self.password_hash)
    }

    /// Generates a registration code.
    pub fn gen_register_code() -> String {
        let mut rng = rand::thread_rng();
        (0..6)
            .map(|_| rng.gen_range(0..=9).to_string())
            .collect::<String>()
    }

    /// Generates a random authentication token.
    pub fn gen_auth_token() -> ([u8; 32], Binary) {
        let code = random::<[u8; 32]>();
        let mut sha = Sha256::new();
        Digest::update(&mut sha, code);
        let hash = sha.finalize();

        (
            code,
            Binary {
                bytes: Vec::from(hash.iter().as_slice()),
                subtype: BinarySubtype::Generic,
            },
        )
    }

    /// Tells whether this users email address has been validated.
    pub fn is_validated(&self) -> bool {
        self.validated
    }

    /// Checks whether the provided username is valid.
    pub fn check_username(username: &String) -> bool {
        let regex = Regex::new(r"^[a-zA-Z0-9]+$").unwrap();

        regex.is_match(&*username.clone())
    }

    /// CHeck whether the provided user tag is valid.
    pub fn check_user_tag(user_tag: &String) -> bool {
        let regex = Regex::new(r"^[a-zA-Z0-9._]+$").unwrap();

        regex.is_match(&*user_tag.clone())
    }

    /// Checks whether the provided email is valid.
    pub fn check_email(email: &String) -> bool {
        let regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();

        regex.is_match(&*email.clone())
    }

    /// Checks whether the provided password is valid.
    pub fn check_password(password: &String) -> bool {
        if password.len() < 8 {
            return false;
        }

        let lowercase_regex = Regex::new(r"[a-z]").unwrap();
        let uppercase_regex = Regex::new(r"[A-Z]").unwrap();
        let digit_regex = Regex::new(r"\d").unwrap();
        let symbol_regex = Regex::new(r"[^\w\s]").unwrap();
        if !lowercase_regex.is_match(&*password.clone())
            | !uppercase_regex.is_match(&*password.clone())
            | !digit_regex.is_match(&*password.clone())
            | !symbol_regex.is_match(&*password.clone())
        {
            false
        } else {
            true
        }
    }

    /// Checks the provided credentials in the registration form; if there is an issue, then it will return the error;
    /// otherwise, it will return [None].
    pub fn check_credentials(
        username: &String,
        email: &String,
        password: &String,
    ) -> Option<Error> {
        let email_good = Self::check_email(email);
        let username_good = Self::check_username(username);
        let password_good = Self::check_password(password);

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

    /// Tells whether the user has set their own profile picture, or the default one should be used.
    pub fn has_profile_picture(&self) -> bool {
        self.profile_picture
    }

    /// Sets the profile picture argument as true when the user has selected a profile picture.
    pub fn set_profile_picture(&mut self) {
        self.profile_picture = true;
    }
}

impl Deserialize<Document> for User {
    fn deserialize(document: &Document) -> Self
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
        if let Ok(user_tag) = document.get_str("user_tag") {
            user.user_tag = user_tag.into();
        }
        if let Ok(role) = document.get_i32("role") {
            user.role = role.into();
        }
        if let Ok(password) = document.get_str("password") {
            user.password_hash = password.into();
        }
        if let Ok(validated) = document.get_bool("validated") {
            user.validated = validated;
        }
        if let Ok(profile_picture) = document.get_bool("profile_picture") {
            user.profile_picture = profile_picture;
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
            "user_tag": Uuid::new().to_string(),
            "role": Into::<i32>::into(Role::User),
            "password": self.password.clone(),
            "register_code": self.code.clone(),
            "auth_token": "",
            "validated": false,
            "token_expiration": Bson::DateTime(
                DateTime::from_millis(DateTime::now().timestamp_millis() + 30 * 24 * 60 * 60 * 1000)
            ),
            "code_expiration": Bson::DateTime(
                DateTime::from_millis(DateTime::now().timestamp_millis() + 5 * 60 * 1000)
            ),
            "profile_picture": false,
            "expiration_date": null
        }
    }
}

impl RegisterForm {
    pub fn get_email(&self) -> &String {
        &self.email
    }

    pub fn get_username(&self) -> &String {
        &self.username
    }

    pub fn get_password(&self) -> &String {
        &self.password
    }

    pub fn get_error(&self) -> &Option<AuthError> {
        &self.error
    }

    #[allow(dead_code)]
    pub fn get_code(&self) -> &String {
        &self.code
    }

    pub fn set_email(&mut self, email: impl Into<String>) {
        self.email = email.into();
    }

    pub fn set_username(&mut self, username: impl Into<String>) {
        self.username = username.into();
    }

    pub fn set_password(&mut self, password: impl Into<String>) {
        self.password = password.into();
    }

    pub fn set_error(&mut self, error: impl Into<Option<AuthError>>) {
        self.error = error.into();
    }

    pub fn set_code(&mut self, code: impl Into<String>) {
        self.code = code.into();
    }

    /// Generates a code verification email.
    pub fn gen_register_email(&self) -> Message {
        Message::builder()
            .from(
                format!("Chartsy <{}>", config::email_address())
                    .parse()
                    .unwrap(),
            )
            .to(format!("{} <{}>", self.username, self.email)
                .parse()
                .unwrap())
            .subject("Code validation for Chartsy account")
            .multipart(MultiPart::alternative_plain_html(
                String::from(format!(
                    "Use the following code to validate your email address:\n{}",
                    self.code
                )),
                String::from(format!(
                    "<p>Use the following code to validate your email address:</p><h1>{}</h1>",
                    self.code
                )),
            ))
            .unwrap()
    }
}

/// The fields of an authentication form.
#[derive(Default, Clone)]
pub struct LogInForm {
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
        }
    }
}

impl LogInForm {
    pub fn get_email(&self) -> &String {
        &self.email
    }

    pub fn get_password(&self) -> &String {
        &self.password
    }

    pub fn get_error(&self) -> &Option<AuthError> {
        &self.error
    }

    pub fn set_email(&mut self, email: impl Into<String>) {
        self.email = email.into();
    }

    pub fn set_password(&mut self, password: impl Into<String>) {
        self.password = password.into();
    }

    pub fn set_error(&mut self, error: impl Into<Option<AuthError>>) {
        self.error = error.into();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub enum AuthTabIds {
    #[default]
    Register,
    LogIn,
}

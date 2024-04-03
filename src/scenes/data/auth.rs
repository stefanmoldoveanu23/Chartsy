use mongodb::bson::{Bson, doc, Document, Uuid, UuidRepresentation};
use crate::errors::auth::AuthError;
use crate::serde::{Deserialize, Serialize};

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
            "validated": true,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AuthTabIds {
    Register,
    LogIn,
}
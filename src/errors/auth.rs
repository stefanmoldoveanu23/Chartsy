use std::fmt::{Debug, Display, Formatter};

/// Errors for the authentication page:
/// - [RegisterBadCredentials](AuthError::RegisterBadCredentials) for when the user has input incorrect credential formats;
/// - [RegisterBadCode](AuthError::RegisterBadCode) for when the user has input the incorrect email verification code;
/// - [RegisterUserAlreadyExists](AuthError::RegisterUserAlreadyExists) for when a user with the provided email already exists;
/// - [LogInUserDoesntExist](AuthError::LogInUserDoesntExist) for when a user with the provided email doesn't exist.
#[derive(Clone)]
pub enum AuthError {
    RegisterBadCredentials {
        email: bool,
        username: bool,
        password: bool,
    },
    RegisterBadCode,
    RegisterUserAlreadyExists,
    LogInUserDoesntExist,
}

impl Display for AuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut message = String::from("");
        f.write_str(
            match self {
                AuthError::RegisterBadCredentials { email, username, password } => {
                    if *email {
                        message = message + "\nThe provided email doesn't follow the proper format!";
                    }
                    if *username {
                        message = message + "\nThe username cannot be empty!";
                    }
                    if *password {
                        message = message + "\nThe password needs to have at least 8 characters, out of which one majuscule, one minuscule, one digit and one symbol!";
                    }

                    &*message
                }
                AuthError::RegisterBadCode => "The provided code is incorrect!",
                AuthError::RegisterUserAlreadyExists => "An account with this email already exists!",
                AuthError::LogInUserDoesntExist => "An account with this email and password doesn't exist!",
            }
        )
    }
}

impl Debug for AuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

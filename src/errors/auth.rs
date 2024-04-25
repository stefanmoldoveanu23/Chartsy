use std::fmt::{Debug, Display, Formatter};

/// Errors for the authentication page.
#[derive(Clone, Eq, PartialEq)]
pub enum AuthError {
    /// The user has input incorrect credential formats.
    RegisterBadCredentials {
        email: bool,
        username: bool,
        password: bool,
    },

    /// The user has input the incorrect email verification code.
    RegisterBadCode,

    /// A user with the provided email already exists.
    RegisterUserAlreadyExists,

    /// A user with the provided email doesn't exist.
    LogInUserDoesntExist,

    /// The provided profile picture is larger than 5MB.
    ProfilePictureTooLarge,

    /// The user tag has incorrect formatting.
    BadUserTag,

    /// The user tag provided is not unique.
    UserTagAlreadyExists
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
                AuthError::RegisterBadCode => "The provided code is incorrect or has expired!",
                AuthError::RegisterUserAlreadyExists => "An account with this email already exists!",
                AuthError::LogInUserDoesntExist => "An account with this email and password doesn't exist!",
                AuthError::ProfilePictureTooLarge => "Your new profile picture needs to be at most 5MB!",
                AuthError::BadUserTag => "The provided user tag cannot be empty!",
                AuthError::UserTagAlreadyExists => "Another account already uses this user tag!"
            }
        )
    }
}

impl Debug for AuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

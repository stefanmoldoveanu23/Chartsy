use crate::errors::auth::AuthError;
use crate::errors::debug::DebugError;
use std::fmt::{Debug, Display, Formatter};

/// Error types:
/// - [DebugError](Error::DebugError), which provides a [DebugError];
/// - [AuthError](Error::AuthError), which provides an [AuthError].
#[derive(Clone)]
pub enum Error {
    DebugError(DebugError),
    AuthError(AuthError),
}

impl Error {
    /// Tells whether the error is a [DebugError].
    pub fn is_debug(&self) -> bool {
        match self {
            Error::DebugError(_) => true,
            _ => false,
        }
    }

    /// Tells whether the error is an [AuthError].
    pub fn is_auth(&self) -> bool {
        match self {
            Error::AuthError(_) => true,
            _ => false,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DebugError(error) => Display::fmt(error, f),
            Error::AuthError(error) => Display::fmt(error, f),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DebugError(error) => Debug::fmt(error, f),
            Error::AuthError(error) => Debug::fmt(error, f),
        }
    }
}

impl From<mongodb::error::Error> for Error {
    fn from(value: mongodb::error::Error) -> Self {
        Error::DebugError(DebugError::new(
            String::from("Error accessing database:\n") + &*value.to_string(),
        ))
    }
}

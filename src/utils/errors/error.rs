use super::AuthError;
use super::DebugError;
use std::fmt::{Debug, Display, Formatter};

/// Error types.
#[derive(Clone, Eq, PartialEq)]
pub enum Error {
    /// An error to be displayed on console, for debug purposes.
    DebugError(DebugError),

    /// An error that the user has encountered while creating or updating their data.
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

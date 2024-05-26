use std::fmt::{Debug, Display, Formatter};

/// Debug error to be printed on the screen.
#[derive(Clone, Eq, PartialEq)]
pub struct DebugError {
    message: String,
}

impl DebugError {
    pub fn new(message: impl Into<String>) -> Self {
        DebugError { message: message.into() }
    }
}

impl Display for DebugError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*self.message)
    }
}

impl Debug for DebugError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Into<Error> for String {
    fn into(self) -> Error {
        Error::DebugError(DebugError::new(self))
    }
}

#[macro_export]
macro_rules! debug_message {
    ($($arg:tt)*) => {
        format!("{}:{} {} [ERROR] - {}", file!(), line!(), chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"), format!($($arg)*))
    };
}

pub use debug_message;
use crate::errors::error::Error;
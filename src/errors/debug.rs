use std::fmt::{Debug, Display, Formatter};

/// Debug error to be printed on the screen.
#[derive(Clone)]
pub struct DebugError {
    message: String,
}

impl DebugError {
    pub fn new(message: String) -> Self {
        DebugError { message }
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

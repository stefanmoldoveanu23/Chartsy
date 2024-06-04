pub mod auth;
pub mod debug;
pub mod error;

pub type Error = error::Error;

pub type DebugError = debug::DebugError;

pub type AuthError = auth::AuthError;

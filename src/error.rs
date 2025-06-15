use std::fmt::{Display, Formatter};
use std::error::Error;

pub type AnyErr = Box<dyn Error + Send + Sync>;

#[derive(Debug, Clone, Copy)]
pub enum ProtocolError {
    LoginRequired,
    InvalidCredentials,
    LoggedOut,
    InvalidDataType
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ProtocolError::LoginRequired => "Login required",
            ProtocolError::InvalidCredentials => "Invalid credentials",
            ProtocolError::LoggedOut => "Logged out",
            ProtocolError::InvalidDataType => "Invalid data type in request"
        })
    }
}

impl Error for ProtocolError {}
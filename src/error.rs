use std::fmt;

/// Custom Error and Result types to unify errors from all sources.
pub type BotResult<T> = Result<T, BotError>;

#[derive(Debug)]
pub enum BotError {
    Http(String),
    Parse,
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BotError::Http(s) => write!(f, "HTTP Error: {}", s),
            BotError::Parse => write!(f, "Parse Error"),
        }
    }
}

impl From<reqwest::Error> for BotError {
    fn from(error: reqwest::Error) -> Self {
        BotError::Http(error.to_string())
    }
}

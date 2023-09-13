use std::error::Error;
use std::fmt;
use tokio_cron_scheduler::JobSchedulerError;

/// Custom Error and Result types to unify errors from all sources.
pub type BotResult<T> = Result<T, BotError>;

#[derive(Debug)]
pub enum BotError {
    Http(String),
    Scheduler(String),
    AOC(String),
    Parse,
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BotError::Http(s) => write!(f, "HTTP Error: {}", s),
            BotError::Scheduler(s) => write!(f, "Scheduler Error: {}", s),
            BotError::AOC(s) => write!(f, "AOC Error: {}", s),
            BotError::Parse => write!(f, "Parse Error"),
        }
    }
}

impl Error for BotError {}

impl From<reqwest::Error> for BotError {
    fn from(error: reqwest::Error) -> Self {
        BotError::Http(error.to_string())
    }
}

impl From<JobSchedulerError> for BotError {
    fn from(error: JobSchedulerError) -> Self {
        BotError::Scheduler(error.to_string())
    }
}

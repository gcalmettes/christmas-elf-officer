use std::{error::Error, fmt};
use tokio_cron_scheduler::JobSchedulerError;

/// Custom Error and Result types to unify errors from all sources.
pub type BotResult<T> = Result<T, BotError>;

#[derive(Debug)]
pub enum BotError {
    Config(String),
    Http(String),
    IO(String),
    Scheduler(String),
    AOC(String),
    ChannelSend(String),
    Slack(String),
    Compute(String),
    Parse,
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BotError::Config(s) => write!(f, "Configuration Error: {}", s),
            BotError::Http(s) => write!(f, "HTTP Error: {}", s),
            BotError::IO(s) => write!(f, "IO Error: {}", s),
            BotError::Scheduler(s) => write!(f, "Scheduler Error: {}", s),
            BotError::AOC(s) => write!(f, "AOC Error: {}", s),
            BotError::ChannelSend(s) => write!(f, "MPSC Error: {}", s),
            BotError::Slack(s) => write!(f, "Slack Communication Error: {}", s),
            BotError::Compute(s) => write!(f, "Computation Error: {}", s),
            BotError::Parse => write!(f, "Parsing Error"),
        }
    }
}

impl Error for BotError {}

impl From<reqwest::Error> for BotError {
    fn from(error: reqwest::Error) -> Self {
        BotError::Http(error.to_string())
    }
}

impl From<std::io::Error> for BotError {
    fn from(error: std::io::Error) -> Self {
        BotError::IO(error.to_string())
    }
}

impl From<JobSchedulerError> for BotError {
    fn from(error: JobSchedulerError) -> Self {
        BotError::Scheduler(error.to_string())
    }
}

pub fn convert_err(e: reqwest::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e)
}

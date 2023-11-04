use clap::Parser;
use serde::{Serialize};

fn is_false(b: &bool) -> bool { !b }

#[derive(Debug, Parser, Serialize)]
pub struct Cli {
    /// Whether to load the private leaderboard for all the previous AOC events
    #[arg(long)]
    #[serde(skip_serializing_if = "is_false")]
    pub all_years: bool,
}

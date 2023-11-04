use crate::cli::Cli;
use clap::Parser;
use figment::{
    providers::{Env, Format, Serialized, Yaml},
    Figment,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::path::Path;
use tracing::Level;

const TRACE_LEVELS: [&'static str; 5] = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];

pub static SETTINGS: Lazy<Settings> = Lazy::new(|| Settings::new());

// Settings are a singleton generated at runtime. All settings may be
// configured via environment variables. Example:
// SLACK_TOKEN="xxx" would set slack_token to the xxx value.
// Some settings are derived from other settings
#[derive(Deserialize, Debug)]
pub struct Settings {
    #[serde(default = "default_trace_level")]
    trace_level: String,
    pub slack_token: String,
    pub slack_app_token: String,
    pub slack_default_channel: String,
    // Channel to reveive heartbeats and monitoring events
    pub slack_monitoring_channel: Option<String>,
    #[serde(default = "default_global_leaderboard_polling_interval_sec")]
    pub global_leaderboard_polling_interval_sec: u64,
    #[serde(default = "default_aoc_base_url")]
    pub aoc_base_url: String,
    #[serde(default = "default_aoc_api_timeout_sec")]
    pub aoc_api_timeout_sec: u64,
    pub aoc_private_leaderboard_id: u64,
    pub aoc_session_cookie: String,
    // Whether to load the private leaderboard for all the previous AOC events
    #[serde(default = "default_all_years")]
    pub all_years: bool,
}

impl Settings {
    pub fn new() -> Self {
        let local_settings_yaml_file = ".env.local.yaml";
        let settings: Settings = match Path::new(local_settings_yaml_file).exists() {
            true => {
                println!(
                    "\n######################################\n\
                       ##   Found '.env.local.yaml' file,  ##\n\
                       ##   loading local configuration.   ##\n\
                       ######################################\n\
                    "
                );
                Figment::new()
                    .merge(Yaml::file(local_settings_yaml_file))
                    .merge(Env::raw())
                    .merge(Serialized::defaults(Cli::parse()))
                    .extract()
                    .unwrap()
            }
            false => Figment::new().merge(Env::raw()).extract().unwrap(),
        };

        settings
    }

    pub fn get_trace_level(&self) -> Level {
        get_trace_level(&self.trace_level)
    }
}

fn get_trace_level(level_str: &str) -> Level {
    match level_str {
        level if level == TRACE_LEVELS[0] => Level::TRACE,
        level if level == TRACE_LEVELS[1] => Level::DEBUG,
        level if level == TRACE_LEVELS[2] => Level::INFO,
        level if level == TRACE_LEVELS[3] => Level::WARN,
        level if level == TRACE_LEVELS[4] => Level::ERROR,
        // Default trace level
        _ => Level::INFO,
    }
}

fn default_trace_level() -> String {
    "INFO".to_string()
}

fn default_global_leaderboard_polling_interval_sec() -> u64 {
    300
}

fn default_aoc_api_timeout_sec() -> u64 {
    5
}

fn default_aoc_base_url() -> String {
    "https://adventofcode.com".to_string()
}

fn default_all_years() -> bool {
    false
}

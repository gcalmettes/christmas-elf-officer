use crate::aoc::leaderboard::ScrapedPrivateLeaderboard;
use itertools::Itertools;
use std::fmt;
use std::iter::Iterator;

use chrono::{DateTime, Timelike, Utc};
use tracing::error;

// use itertools::Itertools;
use slack_morphism::{SlackChannelId, SlackTs};

#[derive(Debug)]
pub enum Event {
    GlobalLeaderboardComplete,
    GlobalLeaderboardHeroFound(String),
    PrivateLeaderboardUpdated,
    CommandReceived(SlackChannelId, SlackTs, Command),
}

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    GetPrivateStandingByLocalScore(Vec<(String, String)>, DateTime<Utc>),
}

const COMMANDS: [&'static str; 2] = ["!help", "!ranking"];

impl Command {
    pub fn is_command(input: &str) -> bool {
        let start_with = input.trim().split(" ").next().unwrap();
        COMMANDS.contains(&start_with)
    }

    pub fn build_from(input: String, leaderboard: &ScrapedPrivateLeaderboard) -> Command {
        let start_with = input.trim().split(" ").next().unwrap();
        match start_with {
            cmd if cmd == COMMANDS[0] => Command::Help,
            cmd if cmd == COMMANDS[1] => {
                let data = leaderboard
                    .leaderboard
                    .standings_by_local_score()
                    .into_iter()
                    .map(|(m, s)| (m, s.to_string()))
                    .collect::<Vec<(String, String)>>();
                Command::GetPrivateStandingByLocalScore(data, leaderboard.timestamp)
            }
            _ => unreachable!(),
        }
    }

    pub fn get_prefix(&self) -> &str {
        match self {
            Command::Help => &COMMANDS[0],
            Command::GetPrivateStandingByLocalScore(..) => &COMMANDS[1],
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Event::GlobalLeaderboardComplete => {
                write!(f, ":tada: Global Leaderboard complete")
            }
            Event::GlobalLeaderboardHeroFound(hero) => {
                write!(
                    f,
                    ":tada: Our very own {} made it to the global leaderboard !",
                    hero
                )
            }
            Event::PrivateLeaderboardUpdated => {
                write!(f, ":repeat: Private Leaderboard updated")
            }
            Event::CommandReceived(_channel_id, ts, cmd) => match cmd {
                // \n\ at each code line end creates a line break at the proper position and discards further spaces in this line of code
                // \x20 (hex; 32 in decimal) is an ASCII space and an indicator for the first space to be preserved in this line of the string
                Command::Help => {
                    write!(
                        f,
                        ":sos: below are the bot commands:\n\
                            \x20   `!help`: the commands\n\
                            \x20   `!ranking`: current ranking by local score\n\
                        "
                    )
                }

                Command::GetPrivateStandingByLocalScore(data, time) => {
                    let timestamp = format!(
                        "{:02}:{:02}:{:02} (UTC)",
                        time.hour(),
                        time.minute(),
                        time.second()
                    );
                    let ranking =
                        format!(":first_place_medal: Current ranking as of {timestamp}:\n");
                    let scores = data
                        .iter()
                        .map(|(name, score)| format!(" \x20 • {name} => {score}"))
                        .join("\n");

                    write!(f, "{ranking}{scores}")
                }
            },
        }
    }
}

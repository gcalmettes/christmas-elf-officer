use crate::aoc::leaderboard::{GlobalLeaderboard, ScrapedPrivateLeaderboard};
use itertools::Itertools;
use std::fmt;
use std::iter::Iterator;

use chrono::{DateTime, Timelike, Utc};
use tracing::error;

// use itertools::Itertools;
use slack_morphism::{SlackChannelId, SlackTs};

#[derive(Debug, Clone)]
pub enum Event {
    GlobalLeaderboardComplete(GlobalLeaderboard),
    GlobalLeaderboardHeroFound(String),
    PrivateLeaderboardUpdated,
    DailySolutionsThreadToInitialize(u32),
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

    // pub fn get_prefix(&self) -> &str {
    //     match self {
    //         Command::Help => &COMMANDS[0],
    //         Command::GetPrivateStandingByLocalScore(..) => &COMMANDS[1],
    //     }
    // }
}

fn suffix(num: u8) -> &'static str {
    let s = num.to_string();
    if s.ends_with('1') && !s.ends_with("11") {
        "st"
    } else if s.ends_with('2') && !s.ends_with("12") {
        "nd"
    } else if s.ends_with('3') && !s.ends_with("13") {
        "rd"
    } else {
        "th"
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Event::DailySolutionsThreadToInitialize(day) => {
                write!(f, ":point_down: Daily solution thread for day {}", day)
            }
            // TODO: do not send full global leaderboard but just what we need ?
            Event::GlobalLeaderboardComplete(global_leaderboard) => {
                let (fastest_part_one, fastest_part_two) = {
                    if let Some((_pos, Some(first_part_fast), Some(second_part_fast))) =
                        global_leaderboard.get_fastest_times()
                    {
                        let first_part_fast_time = {
                            let fast = first_part_fast.time;
                            let seconds = fast.num_seconds() % 60;
                            let minutes = (fast.num_seconds() / 60) % 60;
                            let hours = (fast.num_seconds() / 60) / 60;
                            format!(
                                "[{}] {:02}:{:02}:{:02}",
                                first_part_fast.rank, hours, minutes, seconds,
                            )
                        };
                        let second_part_fast_time = {
                            let fast = second_part_fast.time;
                            let seconds = fast.num_seconds() % 60;
                            let minutes = (fast.num_seconds() / 60) % 60;
                            let hours = (fast.num_seconds() / 60) / 60;
                            format!(
                                "[{}] {:02}:{:02}:{:02}",
                                second_part_fast.rank, hours, minutes, seconds,
                            )
                        };
                        (first_part_fast_time, second_part_fast_time)
                    } else {
                        ("[1] N/A".to_string(), "[1] N/A".to_string())
                    }
                };
                let (slowest_part_one, slowest_part_two) = {
                    if let Some((_pos, Some(first_part_slow), Some(second_part_slow))) =
                        global_leaderboard.get_slowest_times()
                    {
                        let first_part_slow_time = {
                            let slow = first_part_slow.time;
                            let seconds = slow.num_seconds() % 60;
                            let minutes = (slow.num_seconds() / 60) % 60;
                            let hours = (slow.num_seconds() / 60) / 60;
                            format!(
                                "[{}] {:02}:{:02}:{:02}",
                                first_part_slow.rank, hours, minutes, seconds,
                            )
                        };
                        let second_part_slow_time = {
                            let slow = second_part_slow.time;
                            let seconds = slow.num_seconds() % 60;
                            let minutes = (slow.num_seconds() / 60) % 60;
                            let hours = (slow.num_seconds() / 60) / 60;
                            format!(
                                "[{}] {:02}:{:02}:{:02}",
                                second_part_slow.rank, hours, minutes, seconds,
                            )
                        };
                        (first_part_slow_time, second_part_slow_time)
                    } else {
                        ("[100] N/A".to_string(), "[100] N/A".to_string())
                    }
                };

                let fastest_delta = {
                    if let Some((delta, rank)) = global_leaderboard.get_fastest_delta() {
                        let seconds = delta.num_seconds() % 60;
                        let minutes = (delta.num_seconds() / 60) % 60;
                        let hours = (delta.num_seconds() / 60) / 60;
                        format!(
                            "{:02}:{:02}:{:02} ({}{})",
                            hours,
                            minutes,
                            seconds,
                            rank,
                            suffix(rank)
                        )
                    } else {
                        "".to_string()
                    }
                };
                let slowest_delta = {
                    if let Some((delta, rank)) = global_leaderboard.get_slowest_delta() {
                        let seconds = delta.num_seconds() % 60;
                        let minutes = (delta.num_seconds() / 60) % 60;
                        let hours = (delta.num_seconds() / 60) / 60;
                        format!(
                            "{:02}:{:02}:{:02} ({}{})",
                            hours,
                            minutes,
                            seconds,
                            rank,
                            suffix(rank)
                        )
                    } else {
                        "".to_string()
                    }
                };

                write!(
                    f,
                    ":tada: Global Leaderboard complete\n\
                    Part 1: {fastest_part_one} - {slowest_part_one}\n\
                    Part 2: {fastest_part_two} - {slowest_part_two}\n\
                    Delta times range in top 100: {fastest_delta} - {slowest_delta}"
                )
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

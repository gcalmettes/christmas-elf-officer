use crate::{
    core::{
        display,
        leaderboard::ScrapedLeaderboard,
        standings::{standings_board, Jersey, Ranking, Scoring, Standing},
        templates::invalid_year_day_message,
    },
    utils::current_year_day,
};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{collections::HashMap, iter::Iterator};

const COMMANDS: [&'static str; 4] = ["!help", "!fast", "!board", "!tdf"];
static REGEX_COMMANDS: Lazy<Regex> =
    Lazy::new(|| {
        let commands = COMMANDS.join(r"|^");
        Regex::new(format!(
            // <option> set at the end so all other matches have priority
            r"(?<cmd>^{commands})|(?<year>\b\d{{4}}\b)|(?<day>\b\d{{1,2}}\b)|(?<option>\b[\S]+\b)"
    ).as_str())
    .unwrap()
    });

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    Ranking(i32, u8, Vec<(String, String)>, DateTime<Utc>, Ranking),
    StandingTdf(i32, Option<u8>, String, DateTime<Utc>, Jersey),
    LeaderboardDisplay(i32, String, DateTime<Utc>, Scoring),
    NotValid(String),
}

impl Command {
    pub fn parse_string(input: &str) -> HashMap<&str, &str> {
        REGEX_COMMANDS
            .captures_iter(input)
            .flat_map(|caps| {
                REGEX_COMMANDS
                    .capture_names()
                    .filter_map(|o| o.and_then(|n| Some((n, caps.name(n)?.as_str()))))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .into_iter()
            // if several matches for a capture type, we want the first iteration to prevail
            .rev()
            .collect()
    }
    pub fn is_command(input: &str) -> bool {
        Self::parse_string(input).get("cmd").is_some()
    }

    // Note that we call this command on matching command strings, so we know
    // input string is a command. We might want to return Option<Command> later on.
    pub fn build_from(input: String, leaderboard: &ScrapedLeaderboard) -> Option<Command> {
        let parsed = Self::parse_string(&input);

        match parsed.get("cmd") {
            Some(cmd) if cmd == &COMMANDS[0] => Some(Command::Help),
            Some(cmd) if cmd == &COMMANDS[1] => {
                let ranking_str = parsed
                    .get("option")
                    .and_then(|o| Some(*o))
                    .unwrap_or_else(|| Ranking::get_default_str());
                let ranking = Ranking::from_string(ranking_str).unwrap_or(Ranking::DELTA);
                let year = parsed
                    .get("year")
                    .and_then(|d| d.parse::<i32>().ok())
                    .unwrap_or_else(|| current_year_day().0);
                let day = parsed
                    .get("day")
                    .and_then(|d| d.parse::<u8>().ok())
                    .unwrap_or_else(|| current_year_day().1);

                if let Some(msg) = invalid_year_day_message(year, Some(day)) {
                    Some(Command::NotValid(msg))
                } else {
                    let data = Standing::new(&leaderboard.leaderboard).by_time(&ranking, year, day);

                    Some(Command::Ranking(
                        year,
                        day,
                        data,
                        leaderboard.timestamp,
                        ranking,
                    ))
                }
            }
            Some(cmd) if cmd == &COMMANDS[2] => {
                let scoring_str = parsed
                    .get("option")
                    .and_then(|o| Some(*o))
                    .unwrap_or_else(|| &Scoring::get_default_str());
                let scoring = Scoring::from_string(scoring_str).unwrap_or(Scoring::LOCAL);
                let year = parsed
                    .get("year")
                    .and_then(|d| d.parse::<i32>().ok())
                    .unwrap_or_else(|| current_year_day().0);

                if let Some(msg) = invalid_year_day_message(year, None) {
                    Some(Command::NotValid(msg))
                } else {
                    let data = standings_board(&scoring, &leaderboard.leaderboard, year);
                    let formatted = display::board(data);
                    Some(Command::LeaderboardDisplay(
                        year,
                        formatted,
                        leaderboard.timestamp,
                        scoring,
                    ))
                }
            }
            Some(cmd) if cmd == &COMMANDS[3] => {
                let jersey_str = parsed
                    .get("option")
                    .and_then(|o| Some(*o))
                    .unwrap_or_else(|| &Jersey::get_default_str());
                let jersey = Jersey::from_string(jersey_str).unwrap_or(Jersey::YELLOW);
                let year = parsed
                    .get("year")
                    .and_then(|d| d.parse::<i32>().ok())
                    .unwrap_or_else(|| current_year_day().0);
                let day = parsed.get("day").and_then(|d| d.parse::<u8>().ok());

                if let Some(msg) = invalid_year_day_message(year, day) {
                    Some(Command::NotValid(msg))
                } else {
                    let formatted = match (&jersey, day) {
                        // standing yearly, based on points
                        (Jersey::YELLOW, None) => {
                            let standings = Standing::new(&leaderboard.leaderboard);
                            let data = standings.tdf_season(&jersey, year);
                            display::tdf(data)
                        }
                        // standing yearly, based on points
                        (_, None) => {
                            // TODO: whole season
                            let standings = Standing::new(&leaderboard.leaderboard);
                            let data = standings.tdf_season(&jersey, year);
                            display::tdf_season(data)
                        }
                        // daily, based on time
                        (Jersey::YELLOW, Some(day)) => {
                            // TODO: make sure this is correct
                            let standings = Standing::new(&leaderboard.leaderboard);
                            let data = standings.by_time(&Ranking::PART2, year, day);
                            //TODO: update this display
                            display::tdf_time(&data)
                        }
                        // daily, base on points
                        (_, Some(day)) => {
                            let standings = Standing::new(&leaderboard.leaderboard);
                            let data = standings.by_points(&jersey, year, day);
                            display::tdf_points(&data)
                        }
                    };

                    Some(Command::StandingTdf(
                        year,
                        day,
                        formatted,
                        leaderboard.timestamp,
                        jersey,
                    ))
                }
            }
            _ => None,
        }
    }
}

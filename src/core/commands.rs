use crate::{
    core::{
        leaderboard::ScrapedLeaderboard,
        standings::{standings_by_local_score, standings_tdf, Jersey, JERSEY_COLORS},
    },
    utils::{current_year_day, StandingsFmt},
};

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use std::iter::Iterator;

const COMMANDS: [&'static str; 4] = ["!help", "!standings", "!leaderboard", "!tdf"];
// All words, with optional "!" prefix
static REGEX_WORDS: Lazy<Regex> = Lazy::new(|| Regex::new(r"!?\w+").unwrap());

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    PrivateStandingByLocalScore(i32, Vec<(String, String)>, DateTime<Utc>),
    PrivateStandingTdf(i32, String, DateTime<Utc>, Jersey),
    LeaderboardHistogram(i32, String, DateTime<Utc>),
}

impl Command {
    pub fn is_command(input: &str) -> bool {
        REGEX_WORDS
            .find_iter(&input)
            .map(|mat| mat.as_str())
            .next()
            .and_then(|start_with| Some(COMMANDS.contains(&start_with)))
            .unwrap_or_default()
    }

    // Note that we call this command on matching command strings, so we know
    // input string is a command. We might want to return Option<Command> later on.
    pub fn build_from(input: String, leaderboard: &ScrapedLeaderboard) -> Command {
        let mut input = REGEX_WORDS.find_iter(&input).map(|mat| mat.as_str());
        // Here we know it's safe to unwrap, as we pass only valid commands.
        // That might change in the future.
        let start_with = input.next().unwrap();
        match start_with {
            cmd if cmd == COMMANDS[0] => Command::Help,
            cmd if cmd == COMMANDS[1] => {
                // !ranking
                let year = input
                    .next()
                    .and_then(|y| y.parse::<i32>().ok())
                    .unwrap_or_else(|| current_year_day().0);

                let data = standings_by_local_score(&leaderboard.leaderboard, year)
                    .iter()
                    .map(|(id, s)| (id.name.to_string(), s.to_string()))
                    .collect::<Vec<(String, String)>>();

                Command::PrivateStandingByLocalScore(year, data, leaderboard.timestamp)
            }
            cmd if cmd == COMMANDS[2] => {
                // !leaderboard
                let year = input
                    .next()
                    .and_then(|y| y.parse::<i32>().ok())
                    .unwrap_or_else(|| current_year_day().0);

                let formatted = StandingsFmt::board_by_local_score(&leaderboard.leaderboard, year);
                Command::LeaderboardHistogram(year, formatted, leaderboard.timestamp)
            }

            cmd if cmd == COMMANDS[3] => {
                // !tdf
                let color = input.next().unwrap_or_else(|| JERSEY_COLORS[0]);
                let jersey = Jersey::from_string(color);

                let year = match jersey {
                    // it might be possible that someone requested !tdf <year>
                    None => color
                        .parse::<i32>()
                        .ok()
                        .unwrap_or_else(|| current_year_day().0),
                    Some(_) => input
                        .next()
                        .and_then(|y| y.parse::<i32>().ok())
                        .unwrap_or_else(|| current_year_day().0),
                };

                let jersey = jersey.unwrap_or(Jersey::YELLOW);

                let data = standings_tdf(&jersey, &leaderboard.leaderboard, year);
                let formatted = StandingsFmt::tdf(data);
                Command::PrivateStandingTdf(year, formatted, leaderboard.timestamp, jersey)
            }
            _ => unreachable!(),
        }
    }
}

use crate::{
    core::{
        display,
        leaderboard::ScrapedLeaderboard,
        standings::{standings_board, standings_tdf, standings_time, Jersey, Ranking, Scoring},
    },
    utils::current_year_day,
};

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use std::iter::Iterator;

const COMMANDS: [&'static str; 4] = ["!help", "!fast", "!board", "!tdf"];
// All words, with optional "!" prefix
static REGEX_WORDS: Lazy<Regex> = Lazy::new(|| Regex::new(r"!?\w+").unwrap());

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    Ranking(i32, u8, Vec<(String, String)>, DateTime<Utc>, Ranking),
    StandingTdf(i32, String, DateTime<Utc>, Jersey),
    LeaderboardDisplay(i32, String, DateTime<Utc>, Scoring),
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
                // !fast
                let ranking_method = input.next().unwrap_or_else(|| Ranking::get_default_str());
                let ranking = Ranking::from_string(ranking_method);

                let day = match ranking {
                    // it might be possible that someone requested !leaderboard <year>
                    None => ranking_method
                        .parse::<u8>()
                        .ok()
                        .unwrap_or_else(|| current_year_day().1),
                    Some(_) => input
                        .next()
                        .and_then(|y| y.parse::<u8>().ok())
                        .unwrap_or_else(|| current_year_day().1),
                };

                let year = input
                    .next()
                    .and_then(|y| y.parse::<i32>().ok())
                    .unwrap_or_else(|| current_year_day().0);

                // // TODO: find a syntax to pass the year. Right now only now
                // // maybe !fast [method] [day] [year] ?
                // let year = current_year_day().0;

                let ranking = ranking.unwrap_or(Ranking::DELTA);

                let data = standings_time(&ranking, &leaderboard.leaderboard, year, day);

                Command::Ranking(year, day, data, leaderboard.timestamp, ranking)
            }
            cmd if cmd == COMMANDS[2] => {
                // !board
                let scoring_method = input.next().unwrap_or_else(|| Scoring::get_default_str());
                let scoring = Scoring::from_string(scoring_method);

                let year = match scoring {
                    // it might be possible that someone requested !leaderboard <year>
                    None => scoring_method
                        .parse::<i32>()
                        .ok()
                        .unwrap_or_else(|| current_year_day().0),
                    Some(_) => input
                        .next()
                        .and_then(|y| y.parse::<i32>().ok())
                        .unwrap_or_else(|| current_year_day().0),
                };

                let scoring = scoring.unwrap_or(Scoring::LOCAL);

                let data = standings_board(&scoring, &leaderboard.leaderboard, year);
                let formatted = display::board(data);
                Command::LeaderboardDisplay(year, formatted, leaderboard.timestamp, scoring)
            }

            cmd if cmd == COMMANDS[3] => {
                // !tdf
                let color = input.next().unwrap_or_else(|| Jersey::get_default_str());
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
                let formatted = display::tdf(data);
                Command::StandingTdf(year, formatted, leaderboard.timestamp, jersey)
            }
            _ => unreachable!(),
        }
    }
}

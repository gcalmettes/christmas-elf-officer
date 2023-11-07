use crate::aoc::{
    leaderboard::{Entry, Identifier, Leaderboard},
    standings::PENALTY_UNFINISHED_DAY,
};
use chrono::{Datelike, Duration, Utc};
use itertools::Itertools;
use serde::Serialize;
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
};

pub fn ordinal_number_suffix(num: u8) -> &'static str {
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

pub fn format_rank(rank: u8) -> String {
    format!("{}{}", rank, ordinal_number_suffix(rank))
}

pub fn current_year_day() -> (i32, u8) {
    let now = Utc::now();
    let year = now.year();

    // We start taking the current year into account 10 days before the first puzzle unlocks.
    let start_aoc_period = Entry::puzzle_unlock(year, 1)
        .ok()
        // if something wrong happen in the parsing, we won't take the current year into account
        .map_or_else(|| now + Duration::minutes(10), |t| t - Duration::days(10));

    let _year = match start_aoc_period <= now {
        true => year,
        false => year - 1,
    };
    let _day = now.day() as u8;
    // (year, day)

    // TODO: remove fixed (year, day) used for dev purpose
    (2022, 9)
}

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.num_seconds() % 60;
    let minutes = (duration.num_seconds() / 60) % 60;
    let hours = (duration.num_seconds() / 60) / 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds,)
}

pub fn format_duration_with_days(duration: Duration) -> String {
    let seconds = duration.num_seconds() % 60;
    let minutes = (duration.num_seconds() / 60) % 60;
    let hours = ((duration.num_seconds() / 60) / 60) % 24;
    let days = ((duration.num_seconds() / 60) / 60) / 24;
    format!(
        "{:02} days {:02}:{:02}:{:02}",
        days, hours, minutes, seconds,
    )
}

#[derive(Serialize, Debug)]
pub struct DayHighlight {
    pub parts_duration: Vec<String>,
    pub year: i32,
    pub day: u8,
    pub n_stars: usize,
    pub name: String,
    pub delta: Option<String>,
    pub new_points: usize,
}

/// Retrieve needed info to compute highlights statistics
pub fn compute_highlights(current: &Leaderboard, new: &Leaderboard) -> Vec<DayHighlight> {
    let new_entries = new.difference(current).collect::<HashSet<_>>();

    // buffers
    let mut target_days_per_member = HashMap::new();
    let mut target_year_day_combinations = HashSet::new();

    new_entries.iter().for_each(|e| {
        target_days_per_member
            .entry((e.year, &e.id))
            .or_insert(vec![])
            .push(e.day);
        target_year_day_combinations.insert((e.year, e.day));
    });

    // We can now compute the points changes for each id for the year/day
    let current_scores = current.daily_scores_per_year_member();
    let new_scores = new.daily_scores_per_year_member();
    let entries_of_interest =
        target_year_day_combinations
            .iter()
            .fold(HashMap::new(), |mut acc, (year, day)| {
                acc.extend(new.entries_per_year_day_member(*year, *day).into_iter());
                acc
            });

    let highlights = target_days_per_member
        .iter()
        .flat_map(|((year, id), days)| {
            days.iter()
                .unique()
                .map(|d| {
                    // Difference in score
                    let day_index = *d as usize - 1; // arrays are zero-indexed
                    let score_increase = new_scores
                        .get(&(*year, &id))
                        .and_then(|days| Some(days[day_index]))
                        // we know there is a score, unwrap safely
                        .unwrap()
                        - current_scores
                            .get(&(*year, &id))
                            .and_then(|days| Some(days[day_index]))
                            // if first star for new member, no previous result
                            .unwrap_or(0);

                    // compute delta if any
                    let (year, day) = (year, d);
                    let hits = entries_of_interest.get(&(*year, *day, &id)).unwrap();
                    // compute delta
                    let durations = hits
                        .iter()
                        .filter_map(|s| s.duration_since_release().ok())
                        .sorted()
                        .collect::<Vec<Duration>>();
                    let delta = match durations.len() > 1 {
                        true => Some(format_duration(durations[1] - durations[0])),
                        false => None,
                    };

                    DayHighlight {
                        parts_duration: durations.iter().map(|d| format_duration(*d)).collect(),
                        year: *year,
                        day: *day,
                        name: id.name.clone(),
                        n_stars: days.iter().filter(|d| d == &day).count(),
                        delta,
                        new_points: score_increase,
                    }
                })
                .collect::<Vec<DayHighlight>>()
        })
        .sorted_by_key(|h| Reverse(h.new_points))
        .collect::<Vec<DayHighlight>>();

    highlights
}

pub fn get_new_members(cur: &Leaderboard, new: &Leaderboard) -> Vec<String> {
    let cur = cur.iter().map(|e| &e.id.name).collect::<HashSet<&String>>();
    let new = new.iter().map(|e| &e.id.name).collect::<HashSet<&String>>();
    new.difference(&cur).map(|n| n.to_string()).collect()
}

// Helper for formatting all standings
pub struct StandingsFmt;

impl StandingsFmt {
    pub fn tdf(entries: Vec<(&Identifier, i64, i64)>) -> String {
        // calculate width for positions
        // the width of the maximum position to be displayed, plus one for ')'
        let width_pos = entries.len().to_string().len();

        // calculate width for names
        // the length of the longest name, plus one for ':'
        let width_name = 1 + entries
            .iter()
            .map(|(id, _, _)| id.name.len())
            .max()
            .unwrap_or_default();

        // Max possible width for duration is all days above cutoff time
        let width_duration =
            format_duration_with_days(Duration::seconds(*PENALTY_UNFINISHED_DAY * 25)).len();

        entries
            .iter()
            .enumerate()
            .map(|(idx, (id, total_seconds, penalties))| {
                format!(
                    "{:>width_pos$}) {:<width_name$} {:>width_duration$}  {}",
                    // idx is zero-based
                    idx + 1,
                    id.name,
                    format_duration_with_days(Duration::seconds(*total_seconds)),
                    match penalties > &0 {
                        true => format!("({penalties} days over the cut off)"),
                        false => "(COMPLETED)".to_string(),
                    }
                )
            })
            .join("\n")
    }

    // completions map ranked by local score
    pub fn board_by_local_score(leaderboard: &Leaderboard, year: i32) -> String {
        let scores = leaderboard.daily_parts_scores_per_year_member();
        let entries = scores
            .iter()
            .filter(|((y, _id), _scores)| y == &year)
            .map(|((_y, id), scores)| {
                // we compute total score, and total number of stars
                (
                    id,
                    scores,
                    scores.iter().fold((0, 0), |acc, s| {
                        // (number of stars, score)
                        (acc.0 + s.0, acc.1 + s.1)
                    }),
                )
            })
            // sort by score descending, then by star count descending
            .sorted_unstable_by_key(|entry| (Reverse(entry.2 .1), Reverse(entry.2 .0)))
            .collect::<Vec<_>>();

        // calculate width for positions
        // the width of the maximum position to be displayed, plus one for ')'
        let width_pos = entries.len().to_string().len();

        // calculate width for names
        // the length of the longest name, plus one for ':'
        let width_name = 1 + entries
            .iter()
            .map(|(id, _scores, (_n_stars, _total))| id.name.len())
            .max()
            .unwrap_or_default();

        // calculate width for scores
        // the width of the maximum score, formatted to two decimal places
        let width_score = entries
            .iter()
            .map(|(_id, _scores, (_n_stars, total))| total)
            .max()
            .map(|s| 1 + s.to_string().len())
            .unwrap_or_default();

        entries
            .iter()
            .enumerate()
            .map(|(idx, (id, scores, (_n_stars, total)))| {
                format!(
                    "{:>width_pos$}) {:<width_name$} {:>width_score$}  [{}]",
                    // idx is zero-based
                    idx + 1,
                    id.name,
                    total,
                    scores
                        .iter()
                        .map(|(n_star, _s)| match n_star {
                            0 => " -",
                            1 => " □",
                            2 => " ■",
                            _ => unreachable!(),
                        })
                        .collect::<String>()
                )
            })
            .join("\n")
    }
}

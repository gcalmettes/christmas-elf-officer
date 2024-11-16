use crate::core::leaderboard::{Entry, Leaderboard};
use chrono::{Datelike, Duration, Utc};
use itertools::Itertools;
use serde::Serialize;
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
};

pub fn exponential_decay(max: f32, decay_rate: f32, time: i32) -> usize {
    (max * (1.0 - decay_rate).powi(time)).round() as usize
}

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

/// Get the last valid AOC (year, day) combo.
/// If AOC is ongoing, returns the current year and current day.
/// If AOC is over, returns the previous year, last day (25).
pub fn current_aoc_year_day() -> (i32, u8) {
    let now = Utc::now();
    let year = now.year();

    // We start taking the current year into account 15 days before the first puzzle unlocks.
    let start_aoc_period = Entry::puzzle_unlock(year, 1)
        .ok()
        // if something wrong happen in the parsing, we won't take the current year into account
        .map_or_else(|| now + Duration::minutes(10), |t| t - Duration::days(15));

    let year = match start_aoc_period <= now {
        true => year,
        false => year - 1,
    };
    let day = match (start_aoc_period <= now, now.day() <= 25) {
        (true, true) => now.day() as u8,
        _ => 25,
    };
    (year, day)
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

pub fn get_new_members(cur: &Leaderboard, new: &Leaderboard) -> Vec<String> {
    let cur = cur.iter().map(|e| &e.id.name).collect::<HashSet<&String>>();
    let new = new.iter().map(|e| &e.id.name).collect::<HashSet<&String>>();
    new.difference(&cur).map(|n| n.to_string()).collect()
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
                let year_day_member = new
                    .entries_per_member_for_year_day(*year, *day)
                    .into_iter()
                    .map(|(id, entries)| ((year, day, id), entries));
                acc.extend(year_day_member);
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
                        .get(&(*year, id))
                        .map(|days| days[day_index])
                        // we know there is a score, unwrap safely
                        .unwrap()
                        - current_scores
                            .get(&(*year, id))
                            .map(|days| days[day_index])
                            // if first star for new member, no previous result
                            .unwrap_or(0);

                    // compute delta if any
                    let (year, day) = (year, d);
                    let hits = entries_of_interest.get(&(year, day, id)).unwrap();
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

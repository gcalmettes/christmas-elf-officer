use crate::aoc::leaderboard::Leaderboard;
use chrono::Duration;
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

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.num_seconds() % 60;
    let minutes = (duration.num_seconds() / 60) % 60;
    let hours = (duration.num_seconds() / 60) / 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds,)
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
    let new_entries = new.get_members_entries_differences_with(current);

    // buffers
    let mut target_days_per_member = HashMap::new();
    let mut target_year_day_combinations = HashSet::new();

    new_entries.into_iter().for_each(|e| {
        target_days_per_member
            .entry((e.year, e.id))
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
                        .unwrap()
                        - current_scores
                            .get(&(*year, &id))
                            .and_then(|days| Some(days[day_index]))
                            .unwrap();

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

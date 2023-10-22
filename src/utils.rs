use crate::aoc::leaderboard::Solution;
use chrono::Duration;
use itertools::Itertools;
use serde::Serialize;
use std::collections::HashMap;

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

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.num_seconds() % 60;
    let minutes = (duration.num_seconds() / 60) % 60;
    let hours = (duration.num_seconds() / 60) / 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds,)
}

#[derive(Serialize)]
pub struct Entry {
    pub parts_duration: Vec<String>,
    pub year: i32,
    pub day: u8,
    pub part: String,
    pub name: String,
    pub delta: Option<String>,
}

impl Entry {
    pub fn from_solution_vec(svec: Vec<Solution>) -> Self {
        let durations = svec
            .iter()
            .filter_map(|s| s.duration_since_release().ok())
            .sorted()
            .collect::<Vec<Duration>>();

        let s = svec.iter().last().unwrap();
        Self {
            parts_duration: durations.iter().map(|d| format_duration(*d)).collect(),
            part: s.part.to_string(),
            year: s.year,
            day: s.day,
            name: s.id.name.clone(),
            delta: match durations.len() > 1 {
                true => Some(format_duration(durations[1] - durations[0])),
                false => None,
            },
        }
    }
}

//TODO: need to figure out better unified ordering
pub fn categorize_leaderboard_entries(
    entries: &Vec<Solution>,
    target_day: u8,
) -> (Option<Vec<(String, Entry)>>, Option<Vec<(String, Entry)>>) {
    let mut by_day = entries.iter().into_grouping_map_by(|e| e.day).fold(
        HashMap::<String, Vec<Solution>>::new(),
        |mut acc, _key, sol| {
            acc.entry(sol.id.name.clone())
                .or_insert(vec![])
                .push(sol.clone());
            acc
        },
    );

    let target = by_day.remove(&target_day).and_then(|h| {
        Some(
            h.into_iter()
                .map(|(name, svec)| (name, Entry::from_solution_vec(svec)))
                //TODO: need to figure out better unified ordering
                .sorted_unstable_by_key(|(_name, entry)| 2 - entry.parts_duration.len())
                .collect(),
        )
    });

    let others = match by_day.is_empty() {
        true => None,
        false => Some(
            by_day
                .iter()
                .flat_map(|(_day, h)| {
                    h.into_iter()
                        .map(|(name, svec)| (name.clone(), Entry::from_solution_vec(svec.to_vec())))
                })
                .sorted_by_key(|(_name, entry)| entry.day)
                .collect(),
        ),
    };
    (target, others)
}

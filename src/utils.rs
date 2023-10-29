use crate::aoc::leaderboard::Leaderboard;
use chrono::Duration;
use itertools::Itertools;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

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

// #[derive(Serialize)]
// pub struct Summary {
//     pub new_highlights: Vec<Solution>,
//     pub context: Vec<Solution>,
// }

// impl Summary {
//     pub fn to_highlights(&self) {}
// }

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

// impl DayHighlight {
//     pub fn from_solution_vec(svec: Vec<Solution>) -> Self {
//         let durations = svec
//             .iter()
//             .filter_map(|s| s.duration_since_release().ok())
//             .sorted()
//             .collect::<Vec<Duration>>();

//         let s = svec.iter().last().unwrap();
//         Self {
//             parts_duration: durations.iter().map(|d| format_duration(*d)).collect(),
//             part: s.part.to_string(),
//             year: s.year,
//             day: s.day,
//             name: s.id.name.clone(),
//             delta: match durations.len() > 1 {
//                 true => Some(format_duration(durations[1] - durations[0])),
//                 false => None,
//             },
//         }
//     }
// }

////TODO: need to figure out better unified ordering
//pub fn categorize_leaderboard_entries(
//    entries: &Vec<Solution>,
//    target_day: u8,
//) -> (
//    Option<Vec<(String, DayHighlight)>>,
//    Option<Vec<(String, DayHighlight)>>,
//) {
//    let mut by_day = entries.iter().into_grouping_map_by(|e| e.day).fold(
//        HashMap::<String, Vec<Solution>>::new(),
//        |mut acc, _key, sol| {
//            acc.entry(sol.id.name.clone())
//                .or_insert(vec![])
//                .push(sol.clone());
//            acc
//        },
//    );

//    let target = by_day.remove(&target_day).and_then(|h| {
//        Some(
//            h.into_iter()
//                .map(|(name, svec)| (name, DayHighlight::from_solution_vec(svec)))
//                //TODO: need to figure out better unified ordering
//                .sorted_unstable_by_key(|(_name, entry)| 2 - entry.parts_duration.len())
//                .collect(),
//        )
//    });

//    let others = match by_day.is_empty() {
//        true => None,
//        false => Some(
//            by_day
//                .iter()
//                .flat_map(|(_day, h)| {
//                    h.into_iter().map(|(name, svec)| {
//                        (name.clone(), DayHighlight::from_solution_vec(svec.to_vec()))
//                    })
//                })
//                .sorted_by_key(|(_name, entry)| entry.day)
//                .collect(),
//        ),
//    };
//    (target, others)
//}

/// Retrieve needed info to compute highlights statistics
pub fn compute_highlights(current: &Leaderboard, new: &Leaderboard) -> Vec<DayHighlight> {
    // TODO: here we also might need to compute/retrieve anything we need to
    // provide informative message in template
    // https://github.com/ey3ball/fieldbot-aoc/blob/master/lib/aoc/rank/stats.ex#L109
    // - number of points added (diff of daily_scores_per_member for target days)
    // - is it completion of that specific day ? and if so what is delta

    let new_entries = new.compute_entries_differences_from(current);

    let mut target_days_per_member = HashMap::new();
    let mut year_day_combinations = HashSet::new();

    // new_entries.iter().for_each(|s| {
    //     target_days_per_member
    //         .entry((s.year, s.id.numeric))
    //         .or_insert(HashSet::new())
    //         .insert(s.day);
    //     year_day_combinations.insert((s.year, s.day));
    // });
    new_entries.iter().for_each(|s| {
        target_days_per_member
            .entry((s.year, s.id.numeric))
            .or_insert(vec![])
            .push(s.day);
        year_day_combinations.insert((s.year, s.day));
    });

    // We can now compute the points changes for each id for the year/day
    let current_scores = current.daily_scores_per_member();
    let new_scores = new.daily_scores_per_member();
    let entries_of_interest =
        year_day_combinations
            .iter()
            .fold(HashMap::new(), |mut acc, (year, day)| {
                acc.extend(
                    new.solutions_per_member_for_year_day(*year, *day)
                        .into_iter(),
                );
                acc
            });

    let highlights = target_days_per_member
        .iter()
        .flat_map(|((year, id), days)| {
            days.iter().unique().map(|d| {
                // Difference in score
                let day_index = *d as usize - 1; // arrays are zero-indexed
                let score_increase = new_scores
                    .get(&(*year, *id))
                    .and_then(|days| Some(days[day_index]))
                    .unwrap()
                    - current_scores
                        .get(&(*year, *id))
                        .and_then(|days| Some(days[day_index]))
                        .unwrap();

                // compute delta if any
                let (year, day) = (*year, *d);
                let hits = entries_of_interest.get(&(year, day, *id)).unwrap();
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

                let members = new.members();
                DayHighlight {
                    parts_duration: durations.iter().map(|d| format_duration(*d)).collect(),
                    year,
                    day,
                    name: members.get(id).unwrap().to_string(),
                    n_stars: days.iter().filter(|d| *d == &day).count(),
                    delta,
                    new_points: score_increase,
                }
            })
        })
        .sorted_by_key(|h| Reverse(h.new_points))
        .collect::<Vec<DayHighlight>>();

    // compute delta if any
    // we need to retrieve target_days in new leaderboard
    // let stats = target_days.iter().flat_map(|((year, id), days)| {
    //     days.iter().map()
    // });

    // println!(">> {:#?}", new_points);
    // let context = current.get_filtered_entries_for_ids_year_day(&ids, year, day);
    // true
    highlights
}

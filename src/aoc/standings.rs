use crate::aoc::leaderboard::{Entry, Identifier, Leaderboard, ProblemPart};
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use once_cell::sync::Lazy;
use std::{cmp::Reverse, collections::HashMap, fmt};

// Time penalty added for TDF rankings if a day is not finished
pub static PENALTY_UNFINISHED_DAY: Lazy<i64> = Lazy::new(|| Duration::days(7).num_seconds());
pub const JERSEY_COLORS: [&'static str; 2] = ["yellow", "green"];

#[derive(Debug, Clone)]
pub enum Jersey {
    YELLOW,
    GREEN,
}

impl Jersey {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            color if color == JERSEY_COLORS[0] => Some(Jersey::YELLOW),
            color if color == JERSEY_COLORS[1] => Some(Jersey::GREEN),
            _ => None,
        }
    }
}

impl fmt::Display for Jersey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Jersey::YELLOW => {
                write!(f, "{}", JERSEY_COLORS[0])
            }
            Jersey::GREEN => {
                write!(f, "{}", JERSEY_COLORS[1])
            }
        }
    }
}

pub fn standings_by_local_score(leaderboard: &Leaderboard, year: i32) -> Vec<(&Identifier, usize)> {
    let filtered = leaderboard
        .iter()
        .filter(|s| s.year == year)
        .collect::<Vec<&Entry>>();

    let members_count = filtered.iter().map(|e| &e.id).unique().count();

    let parts = filtered.iter().into_group_map_by(|s| (s.day, s.part));

    let daily_scores_per_member = parts
        .iter()
        .map(|(challenge, entries)| {
            (
                challenge,
                entries
                    .iter()
                    // sort solutions chronologically by timestamp
                    .sorted_unstable()
                    // retrieve author of the solution
                    .map(|s| &s.id)
                    .collect::<Vec<&Identifier>>(),
            )
        })
        .fold(HashMap::new(), |mut acc, ((day, _part), ranked_members)| {
            ranked_members
                .into_iter()
                .enumerate()
                .for_each(|(rank_minus_one, id)| {
                    let star_score = members_count - rank_minus_one;
                    let day_scores = acc.entry(id).or_insert([0; 25]);
                    day_scores[(*day - 1) as usize] += star_score;
                });
            acc
        });

    let ranked = daily_scores_per_member
        .into_iter()
        .map(|(id, scores)| (id, scores.iter().sum::<usize>()))
        .sorted_unstable_by(|a, b| b.1.cmp(&a.1))
        .collect::<Vec<(&Identifier, usize)>>();

    ranked
}

pub fn standings_by_number_of_stars(
    leaderboard: &Leaderboard,
    year: i32,
) -> Vec<(&Identifier, usize)> {
    let per_member = leaderboard
        .iter()
        .filter(|s| s.year == year)
        .into_group_map_by(|s| &s.id);

    per_member
        .into_iter()
        .map(|(id, stars)| (id, stars.len()))
        .sorted_by_key(|x| Reverse(x.1))
        .collect()
}

fn compute_green_jersey_kpis(
    daily_entries: &Vec<&Entry>,
    challenges_min_max_time: &HashMap<(u8, ProblemPart), (DateTime<Utc>, DateTime<Utc>)>,
) -> Option<Duration> {
    match daily_entries.len() {
        1 => {
            // unwrap is safe as len == 1
            let entry = daily_entries.last().unwrap();
            match entry.part {
                ProblemPart::FIRST => None,
                ProblemPart::SECOND => {
                    // Overtimed on first part, but came back strong to score second part
                    // Duration is > (part.1, - max first part). We'll substract 1 sec.
                    let max_time_first_part = challenges_min_max_time
                        .get(&(entry.day, ProblemPart::FIRST))
                        .and_then(|(_p1_fast, p1_slow)| Some(*p1_slow))
                        .unwrap();

                    Some(entry.timestamp - max_time_first_part - Duration::seconds(1))
                }
            }
        }
        2 => {
            let mut ordered_parts = daily_entries.iter().sorted_by_key(|s| s.timestamp);
            // safe unwrap since len == 2
            let (first, second) = (ordered_parts.next().unwrap(), ordered_parts.next().unwrap());
            Some(second.timestamp - first.timestamp)
        }
        _ => unreachable!(),
    }
}

fn compute_yellow_jersey_kpis(daily_entries: &Vec<&Entry>) -> Option<Duration> {
    match daily_entries.len() {
        1 => None,
        2 => {
            // time to fully complete day (second part time)
            daily_entries
                .iter()
                .sorted_by_key(|s| s.timestamp)
                .last()
                .and_then(|e| Some(e.duration_since_release().unwrap()))
        }
        _ => unreachable!(),
    }
}

/// ordered vec of (id, total duration, penalties)
pub fn standings_tdf<'a, 'b>(
    jersey: &'b Jersey,
    leaderboard: &'a Leaderboard,
    year: i32,
) -> Vec<(&'a Identifier, i64, i64)> {
    // We do the computation only when necessary
    let challenges_min_max_time = match jersey {
        // We will use max time of part 1 to infer deltas for members who only scored
        // the second part on that day.
        // Min-Max time for each (day, part)
        Jersey::GREEN => Some(leaderboard.parts_min_max_times_for_year(year)),
        Jersey::YELLOW => None,
    };

    let (delta_sum_per_member, max_n_days) = leaderboard
        .iter()
        .filter(|s| s.year == year)
        .into_group_map_by(|s| (&s.id, s.day))
        .into_iter()
        .filter_map(|((id, _day), entries_for_day)| match jersey {
            Jersey::YELLOW => compute_yellow_jersey_kpis(&entries_for_day)
                .and_then(|duration| Some((id, duration))),
            Jersey::GREEN => compute_green_jersey_kpis(
                &entries_for_day,
                challenges_min_max_time.as_ref().unwrap(),
            )
            .and_then(|duration| Some((id, duration))),
        })
        .fold((HashMap::new(), 0), |mut acc, (id, duration)| {
            let delta_sum_and_count = acc.0.entry(id).or_insert((0, 0));
            // we do not want to be unfair with members having finished a day in a time that exceed
            // the time penalty for finishing a day inflicted to members not having finished a day.
            let duration_to_add = match (*PENALTY_UNFINISHED_DAY - duration.num_seconds()) > 0 {
                // time to complete is not greater than max time penalty
                true => duration.num_seconds(),
                false => *PENALTY_UNFINISHED_DAY,
            };
            *delta_sum_and_count = (
                delta_sum_and_count.0 + duration_to_add,
                delta_sum_and_count.1 + 1,
            );
            // we keep track of the max number of full days resolved, so we can later add penalty
            // for unfinished days
            acc.1 = std::cmp::max(acc.1, delta_sum_and_count.1);
            acc
        });

    let standings = delta_sum_per_member
        .iter()
        .map(|(id, (total_duration, finished_days))| {
            let n_penalties = max_n_days - finished_days;
            match n_penalties {
                0 => (*id, *total_duration, n_penalties),
                diff => {
                    // penalty for every challenge not completed
                    let total_duration = total_duration + diff * (*PENALTY_UNFINISHED_DAY);
                    (*id, total_duration, n_penalties)
                }
            }
        })
        // sort by total time ascending, then by number of penalties ascendings
        .sorted_unstable_by(|a, b| match a.1 == b.1 {
            true => a.2.cmp(&b.2),
            false => a.1.cmp(&b.1),
        })
        .collect::<Vec<(&Identifier, i64, i64)>>();

    standings
}

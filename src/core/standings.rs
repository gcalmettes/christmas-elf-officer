use crate::{
    core::leaderboard::{Entry, Identifier, Leaderboard, ProblemPart},
    utils::{exponential_decay, format_duration},
};
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use once_cell::sync::Lazy;
use std::{cmp::Reverse, collections::HashMap, fmt};

// Time penalty added for TDF rankings if a day is not finished
pub static PENALTY_UNFINISHED_DAY: Lazy<i64> = Lazy::new(|| Duration::days(7).num_seconds());
const JERSEY_COLORS: [&'static str; 3] = ["yellow", "green", "combative"];
const SCORING_METHODS: [&'static str; 2] = ["local", "stars"];
const RANKING_METHODS: [&'static str; 4] = ["delta", "p1", "p2", "limit"];

// see https://en.wikipedia.org/wiki/Points_classification_in_the_Tour_de_France#Current
const GREEN_JERSEY_POINTS: [u8; 15] = [50, 30, 20, 18, 16, 14, 12, 10, 8, 7, 6, 5, 4, 3, 2];
const COMBATIVE_JERSEY_MAX_POINTS: f32 = 500.0;
const COMBATIVE_JERSEY_POINTS_DECAY_RATE: f32 = 0.005;

#[derive(Debug, Clone)]
pub enum Scoring {
    LOCAL,
    STARS,
}

#[derive(Debug, Clone)]
pub enum Jersey {
    YELLOW,
    GREEN,
    COMBATIVE,
}

#[derive(Debug, Clone)]
pub enum Ranking {
    DELTA,
    PART1,
    PART2,
    LIMIT,
}

impl Scoring {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            method if method == SCORING_METHODS[0] => Some(Scoring::LOCAL),
            method if method == SCORING_METHODS[1] => Some(Scoring::STARS),
            _ => None,
        }
    }
    pub fn get_default_str() -> &'static str {
        SCORING_METHODS[0]
    }
}

impl fmt::Display for Scoring {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Scoring::LOCAL => {
                write!(f, "{}", SCORING_METHODS[0])
            }
            Scoring::STARS => {
                write!(f, "{}", SCORING_METHODS[1])
            }
        }
    }
}

impl Jersey {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            color if color == JERSEY_COLORS[0] => Some(Jersey::YELLOW),
            color if color == JERSEY_COLORS[1] => Some(Jersey::GREEN),
            color if color == JERSEY_COLORS[2] => Some(Jersey::COMBATIVE),
            _ => None,
        }
    }
    pub fn get_default_str() -> &'static str {
        JERSEY_COLORS[0]
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
            Jersey::COMBATIVE => {
                write!(f, "{}", JERSEY_COLORS[2])
            }
        }
    }
}

impl Ranking {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            method if method == RANKING_METHODS[0] => Some(Ranking::DELTA),
            method if method == RANKING_METHODS[1] => Some(Ranking::PART1),
            method if method == RANKING_METHODS[2] => Some(Ranking::PART2),
            method if method == RANKING_METHODS[3] => Some(Ranking::LIMIT),
            _ => None,
        }
    }
    pub fn get_default_str() -> &'static str {
        RANKING_METHODS[0]
    }
}

impl fmt::Display for Ranking {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ranking::DELTA => {
                write!(f, "{}", RANKING_METHODS[0])
            }
            Ranking::PART1 => {
                write!(f, "{}", RANKING_METHODS[1])
            }
            Ranking::PART2 => {
                write!(f, "{}", RANKING_METHODS[2])
            }
            Ranking::LIMIT => {
                write!(f, "{}", RANKING_METHODS[3])
            }
        }
    }
}

////////////////////////////////////////////////
/// COMMUN
////////////////////////////////////////////////

fn compute_combative_points(remaining_time: i32) -> usize {
    exponential_decay(
        COMBATIVE_JERSEY_MAX_POINTS,
        COMBATIVE_JERSEY_POINTS_DECAY_RATE,
        remaining_time,
    )
}

fn get_ranking_for_year_day<'a, 'b>(
    ranking_type: &'b Ranking,
    leaderboard: &'a Leaderboard,
    year: i32,
    day: u8,
) -> impl Iterator<Item = (&'a Identifier, Duration)> + 'a {
    leaderboard
        .entries_per_member_for_year_day(year, day)
        .into_iter()
        .filter_map(|(id, entries_for_day)| match ranking_type {
            Ranking::DELTA => {
                compute_delta(&entries_for_day).and_then(|duration| Some((id, duration)))
            }
            Ranking::PART1 => get_time_for_part(&entries_for_day, Ranking::PART1)
                .and_then(|duration| Some((id, duration))),
            Ranking::PART2 => get_time_for_part(&entries_for_day, Ranking::PART2)
                .and_then(|duration| Some((id, duration))),
            Ranking::LIMIT => compute_time_before_next_release(&entries_for_day)
                .and_then(|duration| Some((id, duration))),
        })
        .sorted_unstable_by(|a, b| a.1.cmp(&b.1))
}

fn compute_delta(daily_entries: &Vec<&Entry>) -> Option<Duration> {
    match daily_entries.len() {
        2 => {
            let mut ordered_parts = daily_entries.iter().sorted_unstable_by_key(|s| s.timestamp);
            // safe unwrap since len == 2
            let (first, second) = (ordered_parts.next().unwrap(), ordered_parts.next().unwrap());
            Some(second.timestamp - first.timestamp)
        }
        1 => None,
        _ => unreachable!(),
    }
}

fn compute_time_before_next_release(daily_entries: &Vec<&Entry>) -> Option<Duration> {
    match daily_entries.len() {
        2 => {
            let ordered_parts = daily_entries.iter().sorted_unstable_by_key(|s| s.timestamp);
            ordered_parts.last().and_then(|e| {
                Entry::puzzle_unlock(e.year, e.day)
                    .ok()
                    .and_then(|puzzle_release_time| {
                        let next_release = puzzle_release_time + Duration::hours(24);
                        let remaining_time_before_next_release = next_release - e.timestamp;
                        match remaining_time_before_next_release > Duration::seconds(0) {
                            true => Some(remaining_time_before_next_release),
                            false => None,
                        }
                    })
            })
        }
        _ => None,
    }
}

fn get_time_for_part(daily_entries: &Vec<&Entry>, part: Ranking) -> Option<Duration> {
    match (daily_entries.len(), part) {
        (2, Ranking::PART1) => {
            let ordered_parts = daily_entries.iter().sorted_unstable_by_key(|s| s.timestamp);
            // safe unwrap since len == 2
            Some(
                ordered_parts
                    .map(|e| e.duration_since_release().unwrap())
                    .next()
                    .unwrap(),
            )
        }
        (2, Ranking::PART2) => {
            let ordered_parts = daily_entries.iter().sorted_unstable_by_key(|s| s.timestamp);
            // safe unwrap since len == 2
            Some(
                ordered_parts
                    .map(|e| e.duration_since_release().unwrap())
                    .last()
                    .unwrap(),
            )
        }
        (1, Ranking::PART1) => Some(daily_entries[0].duration_since_release().unwrap()),
        (1, Ranking::PART2) => None, // did not finished part 2
        _ => unreachable!(),
    }
}

////////////////////////////////////////////////
/// TIME (!fast command)
////////////////////////////////////////////////

/// ordered vec of (id, duration)
pub fn standings_time<'a, 'b>(
    ranking_type: &'b Ranking,
    leaderboard: &'a Leaderboard,
    year: i32,
    day: u8,
) -> Vec<(String, String)> {
    get_ranking_for_year_day(ranking_type, leaderboard, year, day)
        .map(|(id, duration)| (id.name.clone(), format_duration(duration)))
        .collect::<Vec<_>>()
}

////////////////////////////////////////////////
/// TDF POINTS (green / combative)
////////////////////////////////////////////////

pub fn points_tdf<'a, 'b>(
    jersey: &'b Jersey,
    leaderboard: &'a Leaderboard,
    year: i32,
    day: u8,
) -> Vec<(&'a Identifier, usize)> {
    match jersey {
        Jersey::GREEN => GREEN_JERSEY_POINTS
            .iter()
            .zip(get_ranking_for_year_day(
                &Ranking::DELTA,
                leaderboard,
                year,
                day,
            ))
            .map(|(points, (id, _duration))| (id, *points as usize))
            .collect::<Vec<_>>(),
        Jersey::COMBATIVE => get_ranking_for_year_day(&Ranking::LIMIT, leaderboard, year, day)
            .map(|(id, duration)| (id, compute_combative_points(duration.num_minutes() as i32)))
            .collect::<Vec<_>>(),
        _ => vec![],
    }
}

////////////////////////////////////////////////
/// TOTAL SCORE/STARS
////////////////////////////////////////////////

/// ordered vec of (id, [(n_stars, daily score) for the 25 days], total_stars or total_score)
pub fn standings_board<'a, 'b>(
    score_type: &'b Scoring,
    leaderboard: &'a Leaderboard,
    year: i32,
) -> Vec<(&'a Identifier, [(u8, usize); 25], usize)> {
    let scores = leaderboard.daily_stars_and_scores_per_member_for_year(year);
    let entries = scores
        .into_iter()
        .map(|(id, scores)| {
            // we compute total score, and total number of stars
            (
                id,
                scores,
                scores.iter().fold((0, 0), |acc, s| {
                    // (number of stars, score)
                    (acc.0 + s.0 as usize, acc.1 + s.1)
                }),
            )
        })
        .sorted_unstable_by_key(|entry| match score_type {
            // sort by score descending, then by number of stars descending
            Scoring::LOCAL => (Reverse(entry.2 .1), Reverse(entry.2 .0)),
            // sort by number of stars descending, then by score descending
            Scoring::STARS => (Reverse(entry.2 .0), Reverse(entry.2 .1)),
        })
        .map(
            |(id, scores, (total_stars, total_score))| match score_type {
                Scoring::LOCAL => (id, scores, total_score),
                Scoring::STARS => (id, scores, total_stars),
            },
        )
        .collect::<Vec<_>>();
    entries
}

////////////////////////////////////////////////
/// TDF JERSEY
////////////////////////////////////////////////

fn tdf_points_0<'a, 'b>(
    _jersey: &'b Jersey,
    leaderboard: &'a Leaderboard,
    year: i32,
    day: u8,
) -> Vec<(&'a Identifier, usize)> {
    let scores = leaderboard.daily_delta_and_scores_per_member_for_year(year);

    scores
        .into_iter()
        .map(|(id, deltas_and_scores)| {
            let score = match day > 0 {
                // no valid day => full year
                false => deltas_and_scores
                    .iter()
                    // only 10 spots each day
                    .filter(|(_day_delta, rank, _day_score)| rank <= &10)
                    .map(|(_day_delta, _rank, day_score)| day_score)
                    .sum(),
                true => {
                    let (_day_delta, rank, day_score) = deltas_and_scores[day as usize - 1];
                    // only 10 spots each day
                    match rank <= 10 {
                        true => day_score,
                        false => 0,
                    }
                }
            };
            (id, score)
        })
        .filter(|(_id, score)| score > &0)
        .sorted_unstable_by(|a, b| b.1.cmp(&a.1))
        .collect()
}

/// ordered vec of (id, total duration, days over the cut off)
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
        _ => None,
    };

    let (duration_sum_per_member, max_n_days) = leaderboard
        .iter()
        .filter(|s| s.year == year)
        .into_group_map_by(|s| (&s.id, s.day))
        .into_iter()
        .filter_map(|((id, _day), entries_for_day)| match jersey {
            Jersey::YELLOW => compute_yellow_jersey_duration(&entries_for_day)
                .and_then(|duration| Some((id, duration))),
            // TODO: switch to points system
            Jersey::GREEN => compute_green_jersey_duration(
                &entries_for_day,
                challenges_min_max_time.as_ref().unwrap(),
            )
            .and_then(|duration| Some((id, duration))),
            // TODO: implement
            Jersey::COMBATIVE => todo!(),
        })
        .fold((HashMap::new(), 0), |mut acc, (id, duration)| {
            let duration_sum_and_count = acc.0.entry(id).or_insert((0, 0));
            // we do not want to be unfair with members having finished a day in a time that exceed
            // the time penalty for finishing a day inflicted to members not having finished a day.
            let duration_to_add = match (*PENALTY_UNFINISHED_DAY - duration.num_seconds()) > 0 {
                // time to complete is not greater than max time penalty
                true => duration.num_seconds(),
                false => *PENALTY_UNFINISHED_DAY,
            };
            *duration_sum_and_count = (
                duration_sum_and_count.0 + duration_to_add,
                duration_sum_and_count.1 + 1,
            );
            // we keep track of the max number of full days resolved, so we can later add penalty
            // for unfinished days
            acc.1 = std::cmp::max(acc.1, duration_sum_and_count.1);
            acc
        });

    let standings = duration_sum_per_member
        .iter()
        .map(|(id, (total_duration, finished_days))| {
            let days_over_cutoff = max_n_days - finished_days;
            match days_over_cutoff {
                0 => (*id, *total_duration, days_over_cutoff),
                diff => {
                    // penalty for every challenge not completed
                    let total_duration = total_duration + diff * (*PENALTY_UNFINISHED_DAY);
                    (*id, total_duration, days_over_cutoff)
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

fn compute_green_jersey_duration(
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

fn compute_yellow_jersey_duration(daily_entries: &Vec<&Entry>) -> Option<Duration> {
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

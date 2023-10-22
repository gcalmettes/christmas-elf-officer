use crate::error::{BotError, BotResult};
use chrono::{naive::NaiveDateTime, DateTime, Duration, TimeZone, Utc};
use itertools::Itertools;
use itertools::MinMaxResult;
use scraper::{Node, Selector};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt;
use std::iter::Iterator;
use std::ops::{Deref, DerefMut};

static AOC_PUZZLE_UTC_STARTING_HOUR: u32 = 5;
static AOC_MONTH: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ProblemPart {
    FIRST,
    SECOND,
}

#[derive(Debug)]
pub struct LeaderboardStatistics {
    pub p1_fast: Option<Duration>,
    pub p1_slow: Option<Duration>,
    pub p2_fast: Option<Duration>,
    pub p2_slow: Option<Duration>,
    // (Delta,final rank (part 2))
    pub delta_fast: Option<(Duration, Option<u8>)>,
    pub delta_slow: Option<(Duration, Option<u8>)>,
}

// Puzzle completion events parsed from AoC API.
// Year and day fields match corresponding components of DateTime<Utc>.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Solution {
    pub timestamp: DateTime<Utc>,
    pub year: i32,
    pub day: u8,
    pub part: ProblemPart,
    pub id: Identifier,
    pub rank: Option<u8>,
}

// unique identifier for a participant on this leaderboard
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Identifier {
    pub name: String,
    pub numeric: u64,
    pub global_score: u64,
}

type SolutionVec = Vec<Solution>;

#[derive(Debug)]
pub struct Leaderboard(SolutionVec);

#[derive(Debug)]
pub struct ScrapedLeaderboard {
    pub timestamp: chrono::DateTime<Utc>,
    pub leaderboard: Leaderboard,
}

impl fmt::Display for ProblemPart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProblemPart::FIRST => {
                write!(f, "1")
            }
            ProblemPart::SECOND => {
                write!(f, "2")
            }
        }
    }
}

impl ProblemPart {
    pub fn from(input: usize) -> Self {
        match input {
            1 => ProblemPart::FIRST,
            2 => ProblemPart::SECOND,
            // only two parts for each problem
            _ => unreachable!(),
        }
    }
}

impl Solution {
    pub fn from_html(
        entry: scraper::element_ref::ElementRef,
        year: i32,
        day: u8,
        part: ProblemPart,
    ) -> Option<Self> {
        let rank_selector = Selector::parse(r#".leaderboard-position"#).unwrap();
        let time_selector = Selector::parse(r#".leaderboard-time"#).unwrap();

        let id = match entry.value().attr("data-user-id") {
            Some(id) => id.parse::<u64>().ok(),
            None => None,
        };

        // Depending on whether users have declared their github, are sponsors, etc ... the name
        // will be accessible in different possible DOM hierarchy layouts.
        let name = entry
            .children()
            .filter_map(|node| match node.value() {
                Node::Text(text) => Some(text.trim()),
                Node::Element(el) => match el.name() {
                    // Name wrapped into <a> tags to link to user's github.
                    "a" => {
                        let text = node.last_child().unwrap().value();
                        let text = text.as_text().unwrap().trim();
                        // We ignore <a> tags related to (AoC++) or (Sponsor) labels.
                        match (text.starts_with("("), text.ends_with(")")) {
                            (false, false) => Some(text),
                            (_, _) => None,
                        }
                    }
                    _ => None,
                },
                _ => None,
            })
            .filter(|text| !text.is_empty())
            .last();

        let rank = match entry.select(&rank_selector).next() {
            Some(text) => match text.text().next() {
                Some(t) => t
                    .split(")")
                    .next()
                    .and_then(|rank| rank.trim().parse::<u8>().ok()),
                None => None,
            },
            None => None,
        };

        let timestamp = match entry.select(&time_selector).next() {
            Some(t) => t
                .text()
                .filter_map(|time| {
                    let with_year = format!("{} {}", year, time);
                    let naive_datetime =
                        NaiveDateTime::parse_from_str(&with_year, "%Y %b %d  %H:%M:%S").ok();
                    naive_datetime
                })
                .map(|d| {
                    // Global leaderboard entries are starting at 00:00:00, so we need to offset by
                    // 5 hours to get real UTC time.
                    DateTime::<Utc>::from_utc(d, Utc)
                        + Duration::hours(AOC_PUZZLE_UTC_STARTING_HOUR.into())
                })
                .last(),
            None => None,
        };

        match (id, name, rank, timestamp) {
            (Some(id), _, Some(rank), Some(timestamp)) => Some(Solution {
                id: Identifier {
                    // Name of anonymous user will be None
                    name: name
                        .map_or(format!("anonymous user #{}", id), |n| n.to_string())
                        .to_string(),
                    numeric: id,
                    // We won't use it
                    global_score: 0,
                },
                rank: Some(rank),
                part,
                year,
                day,
                timestamp,
            }),
            _ => None,
        }
    }

    pub fn puzzle_unlock(year: i32, day: u8) -> BotResult<DateTime<Utc>> {
        // Problems are released at 05:00:00 UTC
        Utc.with_ymd_and_hms(
            year,
            AOC_MONTH,
            day.into(),
            AOC_PUZZLE_UTC_STARTING_HOUR,
            0,
            0,
        )
        .single()
        .ok_or(BotError::Parse)
    }

    pub fn duration_since_release(&self) -> BotResult<Duration> {
        let release_time = Solution::puzzle_unlock(self.year, self.day)?;
        Ok(self.timestamp - release_time)
    }
}

impl Leaderboard {
    pub fn new() -> Leaderboard {
        Leaderboard(SolutionVec::new())
    }

    fn is_entry_count_equal_to(&self, n: usize) -> bool {
        self.len() == n
    }

    pub fn is_global_complete(&self) -> bool {
        // 100 entries for each part, so completion of global leaderboard is 2*100
        self.is_entry_count_equal_to(200)
    }

    /// Members => (unordered) stars
    fn solutions_per_member(&self) -> HashMap<&Identifier, Vec<&Solution>> {
        self.iter().into_group_map_by(|a| &a.id)
    }

    /// Idem, but for a specific day
    fn solutions_per_member_for_year_day(
        &self,
        year: i32,
        day: u8,
    ) -> HashMap<&Identifier, Vec<&Solution>> {
        self.iter()
            .filter(|s| s.year == year && s.day == day)
            .into_group_map_by(|a| &a.id)
    }

    fn solutions_per_challenge(&self) -> HashMap<(u8, ProblemPart), Vec<&Solution>> {
        self.iter().into_group_map_by(|a| (a.day, a.part))
    }

    pub fn members_ids(&self) -> Vec<u64> {
        self.solutions_per_member()
            .iter()
            .map(|(id, _)| id.numeric)
            .collect::<Vec<u64>>()
    }

    pub fn get_member_by_id(&self, id: u64) -> Option<&Identifier> {
        self.solutions_per_member()
            .into_iter()
            .find_map(|(m_id, _)| match m_id.numeric == id {
                true => Some(m_id),
                false => None,
            })
    }

    fn standings_per_challenge(&self) -> HashMap<(u8, ProblemPart), Vec<&Identifier>> {
        self.solutions_per_challenge()
            .into_iter()
            .map(|(challenge, solutions)| {
                (
                    challenge,
                    solutions
                        .into_iter()
                        // sort solutions chronologically by timestamp
                        .sorted_unstable()
                        // retrieve author of the solution
                        .map(|s| &s.id)
                        .collect(),
                )
            })
            .collect::<HashMap<(u8, ProblemPart), Vec<&Identifier>>>()
    }

    fn daily_scores_per_member(&self) -> HashMap<&Identifier, [usize; 25]> {
        // Max point earned for each star is number of members in leaderboard
        let n_members = self.solutions_per_member().len();

        let standings_per_challenge = self.standings_per_challenge();
        standings_per_challenge
            .iter()
            .fold(HashMap::new(), |mut acc, ((day, _part), star_rank)| {
                star_rank
                    .iter()
                    .enumerate()
                    .for_each(|(rank_minus_one, id)| {
                        let star_score = n_members - rank_minus_one;
                        let day_scores = acc.entry(*id).or_insert([0; 25]);
                        day_scores[(*day - 1) as usize] += star_score;
                    });
                acc
            })
    }

    fn local_scores_per_member(&self) -> HashMap<&Identifier, usize> {
        self.daily_scores_per_member()
            .iter()
            .map(|(id, daily_scores)| (*id, daily_scores.iter().sum()))
            .collect()
    }

    pub fn compute_entries_differences_from(
        &self,
        current_leaderboard: &Leaderboard,
    ) -> Vec<Solution> {
        let current_solutions = current_leaderboard
            .iter()
            .map(|s| (s.id.numeric, s.day, s.part))
            .collect::<Vec<(u64, u8, ProblemPart)>>();

        self.iter()
            .filter_map(
                |s| match !current_solutions.contains(&(s.id.numeric, s.day, s.part)) {
                    true => Some(s.clone()),
                    false => None,
                },
            )
            .collect()
    }

    pub fn standings_by_local_score(&self) -> Vec<(String, usize)> {
        let scores = self.local_scores_per_member();

        scores
            .into_iter()
            .sorted_by_key(|x| Reverse(x.1))
            .map(|(id, score)| (id.name.clone(), score))
            .collect::<Vec<(String, usize)>>()
    }

    pub fn standings_by_number_of_stars(&self) -> Vec<(String, usize)> {
        let stars = self.solutions_per_member();

        stars
            .into_iter()
            .map(|(id, stars)| {
                (
                    id.name.clone(),
                    stars.len(),
                    // Get the timestamp of the last earned star
                    stars.into_iter().sorted_unstable().last(),
                )
            })
            // Sort by number of star (reverse) then by most recent star on equality
            .sorted_by_key(|x| (Reverse(x.1), x.2))
            .map(|(name, n_stars, _)| (name, n_stars))
            .collect::<Vec<(String, usize)>>()
    }

    pub fn standings_by_global_score(&self) -> Vec<(String, u64)> {
        self.solutions_per_member()
            .iter()
            .filter(|(id, _)| id.global_score > 0)
            .map(|(id, _)| (id.name.clone(), id.global_score))
            .sorted_by_key(|h| Reverse(h.1))
            .collect::<Vec<(String, u64)>>()
    }

    pub fn standings_by_local_score_for_day(&self, day: usize) -> Vec<(String, usize)> {
        self.daily_scores_per_member()
            .iter()
            .map(|(id, daily_scores)| (id.name.clone(), daily_scores[day - 1]))
            .filter(|(_, score)| *score > 0)
            .sorted_by_key(|m| Reverse(m.1))
            .collect::<Vec<(String, usize)>>()
    }

    fn compute_min_max_times_for_year_day(
        &self,
        year: i32,
        day: u8,
    ) -> HashMap<ProblemPart, (DateTime<Utc>, DateTime<Utc>)> {
        // Compute max time for each part, in order to infer deltas for members who only scored
        // one part of the global leaderboard that day.
        self.iter()
            .filter(|s| s.year == year && s.day == day)
            .into_group_map_by(|s| s.part)
            .iter()
            .map(
                |(p, solutions)| match solutions.iter().minmax_by_key(|s| s.timestamp) {
                    MinMaxResult::OneElement(s) => (*p, (s.timestamp, s.timestamp)),
                    MinMaxResult::MinMax(s1, s2) => (*p, (s1.timestamp, s2.timestamp)),
                    MinMaxResult::NoElements => unreachable!(),
                },
            )
            .collect::<HashMap<ProblemPart, (DateTime<Utc>, DateTime<Utc>)>>()
    }

    pub fn standings_by_delta_for_year_day(
        &self,
        year: i32,
        day: u8,
    ) -> BotResult<Vec<(String, Duration, Option<u8>)>> {
        // We will use max time of part 1 to infer deltas for members who only scored
        // the second part on that day.
        let max_time_first_part = self
            .compute_min_max_times_for_year_day(year, day)
            .get(&ProblemPart::FIRST)
            .and_then(|(_p1_fast, p1_slow)| Some(*p1_slow))
            .ok_or(BotError::Compute(
                "MinMax times could not be computed".to_string(),
            ))?;

        let standings = self
            .solutions_per_member_for_year_day(year, day)
            .into_iter()
            .filter_map(|(id, solutions_for_day)| match solutions_for_day.len() {
                1 => {
                    // unwrap is safe as len == 1
                    let entry = solutions_for_day.last().unwrap();
                    match entry.part {
                        ProblemPart::FIRST => None,
                        ProblemPart::SECOND => {
                            // Overtimed on first part, but came back strong to score second part
                            // Duration is > (part.1, - max first part). We'll substract 1 sec.
                            Some((
                                id.name.clone(),
                                entry.timestamp - max_time_first_part - Duration::seconds(1),
                                entry.rank,
                            ))
                        }
                    }
                }
                2 => {
                    let mut ordered_parts = solutions_for_day.iter().sorted_by_key(|s| s.timestamp);
                    // safe unwrap since len == 2
                    let (first, second) =
                        (ordered_parts.next().unwrap(), ordered_parts.next().unwrap());
                    Some((
                        id.name.clone(),
                        second.timestamp - first.timestamp,
                        second.rank,
                    ))
                }
                _ => unreachable!(),
            })
            .sorted_by_key(|r| r.1)
            .collect::<Vec<(String, Duration, Option<u8>)>>();
        Ok(standings)
    }

    pub fn daily_statistics(&self, year: i32, day: u8) -> BotResult<LeaderboardStatistics> {
        let minmax_by_part = self.compute_min_max_times_for_year_day(year, day);
        let (p1_fast, p1_slow) =
            minmax_by_part
                .get(&ProblemPart::FIRST)
                .ok_or(BotError::Compute(
                    "Could not retrieve minmax for part 1".to_string(),
                ))?;
        let (p2_fast, p2_slow) =
            minmax_by_part
                .get(&ProblemPart::SECOND)
                .ok_or(BotError::Compute(
                    "Could not retrieve minmax for part 2".to_string(),
                ))?;

        let sorted_deltas = self.standings_by_delta_for_year_day(year, day)?;
        let mut sorted_deltas_iter = sorted_deltas.iter();

        let challenge_start_time = Solution::puzzle_unlock(year, day)?;

        let stats = LeaderboardStatistics {
            p1_fast: Some(*p1_fast - challenge_start_time),
            p1_slow: Some(*p1_slow - challenge_start_time),
            p2_fast: Some(*p2_fast - challenge_start_time),
            p2_slow: Some(*p2_slow - challenge_start_time),
            delta_fast: sorted_deltas_iter
                .next()
                .and_then(|(_name, duration, rank)| Some((*duration, *rank))),
            delta_slow: sorted_deltas_iter
                .last()
                .and_then(|(_name, duration, rank)| Some((*duration, *rank))),
        };
        Ok(stats)
    }

    pub fn look_for_other_leaderboard_members(
        &self,
        other_leaderboard: &Leaderboard,
    ) -> Vec<(Identifier, ProblemPart)> {
        let other_members_ids = other_leaderboard.members_ids();
        let heroes = self
            .iter()
            .filter(|entry| other_members_ids.contains(&entry.id.numeric))
            .map(|entry| {
                (
                    other_leaderboard
                        .get_member_by_id(entry.id.numeric)
                        // we can safely unwrap as if it enters the map there is a match
                        .unwrap()
                        .clone(),
                    entry.part,
                )
            })
            .collect::<Vec<(Identifier, ProblemPart)>>();
        heroes
    }
}

impl Deref for Leaderboard {
    type Target = SolutionVec;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Leaderboard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ScrapedLeaderboard {
    pub fn new() -> ScrapedLeaderboard {
        ScrapedLeaderboard {
            timestamp: Utc::now(),
            leaderboard: Leaderboard::new(),
        }
    }
}

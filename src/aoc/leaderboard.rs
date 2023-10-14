use chrono::naive::NaiveDateTime;
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use scraper::Selector;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt;
use std::iter::Iterator;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ProblemPart {
    FIRST,
    SECOND,
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

// Puzzle completion events parsed from AoC API.
// Year and day fields match corresponding components of DateTime<Utc>.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Solution {
    pub timestamp: DateTime<Utc>,
    pub year: i32,
    pub day: u8,
    pub part: ProblemPart,
    pub id: Identifier,
}

// unique identifier for a participant on this leaderboard
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Identifier {
    pub name: String,
    pub numeric: u64,
    pub global_score: u64,
}

type SolutionVec = Vec<Solution>;
type GlobalLeaderboardEntryVec = Vec<GlobalLeaderboardEntry>;

#[derive(Debug)]
pub struct PrivateLeaderboard(SolutionVec);

#[derive(Debug)]
pub struct ScrapedPrivateLeaderboard {
    pub timestamp: chrono::DateTime<Utc>,
    pub leaderboard: PrivateLeaderboard,
}

#[derive(Debug, Clone)]
pub struct GlobalLeaderboard(pub GlobalLeaderboardEntryVec);

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalLeaderboardEntry {
    pub id: u64,
    pub rank: u8,
    pub part: ProblemPart,
    pub time: Duration,
}

impl PrivateLeaderboard {
    pub fn new() -> PrivateLeaderboard {
        PrivateLeaderboard(SolutionVec::new())
    }

    /// Members => (unordered) stars
    fn solutions_per_member(&self) -> HashMap<&Identifier, Vec<&Solution>> {
        self.iter().into_group_map_by(|a| &a.id)
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
            .fold(HashMap::new(), |mut acc, ((day, _), star_rank)| {
                star_rank.iter().enumerate().for_each(|(rank, id)| {
                    let star_score = n_members - rank;
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

    pub fn compute_diffs(&self, current_leaderboard: &PrivateLeaderboard) -> Vec<&Solution> {
        let current_solutions = current_leaderboard
            .iter()
            .map(|s| (s.id.numeric, s.day, s.part));

        self.iter()
            // The curent_solutions iterator needs to be cloned as .contains() consumes it partially
            // (or totally if no match found)
            .filter(|s| {
                !current_solutions
                    .clone()
                    .contains(&(s.id.numeric, s.day, s.part))
            })
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

    // ranking by time between part 1 and part 2 completions
    pub fn standings_by_delta_for_day(&self, day: u8) -> Vec<(String, Duration)> {
        self.solutions_per_member()
            .into_iter()
            .filter_map(|(id, solutions)| {
                let solutions_for_day = solutions.iter().filter(|s| s.day == day);
                match solutions_for_day.clone().count() {
                    0 | 1 => None,
                    2 => {
                        let mut ordered_parts =
                            solutions_for_day.sorted_by_key(|s| s.timestamp).tuples();
                        let (first, second) = ordered_parts.next().unwrap();
                        Some((id.name.clone(), second.timestamp - first.timestamp))
                    }
                    _ => unreachable!(),
                }
            })
            .sorted_by_key(|r| r.1)
            .collect::<Vec<(String, Duration)>>()
    }
}

impl Deref for PrivateLeaderboard {
    type Target = SolutionVec;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PrivateLeaderboard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl GlobalLeaderboard {
    pub fn is_complete(&self) -> bool {
        self.0.len() == 200
    }

    fn sorted_ranks(
        &self,
    ) -> impl Iterator<
        Item = (
            u8,
            Option<&GlobalLeaderboardEntry>,
            Option<&GlobalLeaderboardEntry>,
        ),
    > {
        let by_rank = self.0.iter().into_group_map_by(|e| e.rank);
        by_rank
            .into_iter()
            .map(|(rank, times)| {
                let mut chronologically_ordered = times.into_iter().sorted_by_key(|e| e.time);
                (
                    rank,
                    chronologically_ordered.next(),
                    chronologically_ordered.next(),
                )
            })
            .sorted_by_key(|t| t.0)
    }

    fn sorted_deltas(&self) -> std::vec::IntoIter<(Duration, u8)> {
        // Needed for computation of deltas for members who only scored one part of the global
        // leaderboard that day.
        let (max_time_for_first_part, max_time_for_second_part) = self.0.iter().fold(
            (Duration::milliseconds(0), Duration::milliseconds(0)),
            |mut acc, entry| match entry.part {
                ProblemPart::FIRST => {
                    if entry.time > acc.0 {
                        acc.0 = entry.time
                    };
                    acc
                }
                ProblemPart::SECOND => {
                    if entry.time > acc.1 {
                        acc.1 = entry.time
                    };
                    acc
                }
            },
        );

        let by_id = self.0.iter().into_group_map_by(|e| e.id);
        by_id
            .into_iter()
            .map(|(_id, entries)| {
                match entries.len() {
                    1 => {
                        // unwrap is safe as len == 1
                        let entry = entries.last().unwrap();
                        match entry.part {
                            ProblemPart::FIRST => {
                                // Scored only first part, second part overtime.
                                // Duration is > (max second part - part.1), so we'll add 1 to the
                                // diff as we have no way to know exactly
                                (
                                    max_time_for_second_part - entry.time + Duration::seconds(1),
                                    101,
                                )
                            }
                            ProblemPart::SECOND => {
                                // Overtimed on first part, but came back strong to score second part
                                // Duration is > (part.1, - max first part). We'll substract 1 sec.
                                (
                                    entry.time - max_time_for_first_part - Duration::seconds(1),
                                    entry.rank,
                                )
                            }
                        }
                    }
                    2 => {
                        let mut sorted = entries.into_iter().sorted_by_key(|e| e.time);
                        // unwrap are safe as len == 2
                        let (p1, p2) = (sorted.next().unwrap(), sorted.next().unwrap());
                        (p2.time - p1.time, p2.rank)
                    }
                    _ => unreachable!(),
                }
            })
            .sorted()
    }

    pub fn get_fastest_and_slowest_times(
        &self,
    ) -> (
        Option<(
            u8,
            Option<&GlobalLeaderboardEntry>,
            Option<&GlobalLeaderboardEntry>,
        )>,
        Option<(
            u8,
            Option<&GlobalLeaderboardEntry>,
            Option<&GlobalLeaderboardEntry>,
        )>,
    ) {
        let mut ranked = self.sorted_ranks();
        (ranked.next(), ranked.last())
    }

    pub fn get_fastest_and_slowest_deltas(
        &self,
    ) -> (Option<(Duration, u8)>, Option<(Duration, u8)>) {
        // only keep people who finished in top 100 of part 2
        let mut deltas = self
            .sorted_deltas()
            .filter(|(_duration, rank)| rank <= &100);
        (deltas.next(), deltas.last())
    }

    pub fn check_for_private_members(
        &self,
        private_leaderboard: &PrivateLeaderboard,
    ) -> Vec<(Identifier, ProblemPart)> {
        let private_members_ids = private_leaderboard.members_ids();
        let heroes = self
            .iter()
            .filter(|entry| private_members_ids.contains(&entry.id))
            .map(|entry| {
                (
                    private_leaderboard
                        .get_member_by_id(entry.id)
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

impl Deref for GlobalLeaderboard {
    type Target = GlobalLeaderboardEntryVec;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GlobalLeaderboard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ScrapedPrivateLeaderboard {
    pub fn new() -> ScrapedPrivateLeaderboard {
        ScrapedPrivateLeaderboard {
            timestamp: Utc::now(),
            leaderboard: PrivateLeaderboard::new(),
        }
    }
}

impl GlobalLeaderboardEntry {
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

        let rank = match entry.select(&rank_selector).next() {
            Some(text) => match text.text().next() {
                Some(t) => {
                    if let Some(rank) = t.split(")").next() {
                        rank.trim().parse::<u8>().ok()
                    } else {
                        None
                    }
                }
                None => None,
            },
            None => None,
        };

        let time = match entry.select(&time_selector).next() {
            Some(t) => {
                t.text()
                    .filter_map(|time| {
                        let with_year = format!("{} {}", year, time);
                        // This will provably never happen, but in theory, even competitors of the global leaderboard could take more than 24h to solve a challenge.
                        // So we will compute the duration based on day.
                        let start_time = format!("{} Dec  {}  00:00:00", year, day);
                        let naive_datetime =
                            NaiveDateTime::parse_from_str(&with_year, "%Y %b %d  %H:%M:%S").ok();
                        let naive_start =
                            NaiveDateTime::parse_from_str(&start_time, "%Y %b %d  %H:%M:%S").ok();

                        match (naive_datetime, naive_start) {
                            (Some(finish), Some(start)) => Some(finish - start),
                            (_, _) => None,
                        }
                    })
                    .last()
            }
            None => None,
        };

        match (id, rank, time) {
            (Some(id), Some(rank), Some(time)) => Some(GlobalLeaderboardEntry {
                id,
                rank,
                time,
                part,
            }),
            (_, _, _) => None,
        }
    }
}

use chrono::naive::NaiveDateTime;
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use scraper::Selector;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

// Puzzle completion events parsed from AoC API.
// Year and day fields match corresponding components of DateTime<Utc>.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Solution {
    pub timestamp: DateTime<Utc>,
    pub year: i32,
    pub day: u8,
    pub part: u8,
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

#[derive(Debug)]
pub struct GlobalLeaderboard(pub GlobalLeaderboardEntryVec);

#[derive(Debug)]
pub struct GlobalLeaderboardEntry {
    pub id: u64,
    pub rank: u8,
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

    fn solutions_per_challenge(&self) -> HashMap<(u8, u8), Vec<&Solution>> {
        self.iter().into_group_map_by(|a| (a.day, a.part))
    }

    fn members_ids(&self) -> Vec<u64> {
        self.solutions_per_member()
            .iter()
            .map(|(id, _)| id.numeric)
            .collect::<Vec<u64>>()
    }

    fn standings_per_challenge(&self) -> HashMap<(u8, u8), Vec<&Identifier>> {
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
            .collect::<HashMap<(u8, u8), Vec<&Identifier>>>()
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

    // fn daily_scores_per_member(&self) -> HashMap<&Identifier, [usize; 25]> {
    pub fn standings_for_day(&self, day: usize) -> Vec<(String, usize)> {
        self.daily_scores_per_member()
            .iter()
            .map(|(id, daily_scores)| (id.name.clone(), daily_scores[day - 1]))
            .filter(|(_, score)| *score > 0)
            .sorted_by_key(|m| Reverse(m.1))
            .collect::<Vec<(String, usize)>>()
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

impl PrivateLeaderboard {
    //TODO:
    // is_complete
    // check_heroes
    // get delta times
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
    pub fn from_html(entry: scraper::element_ref::ElementRef, year: i32, day: u8) -> Option<Self> {
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
            (Some(id), Some(rank), Some(time)) => Some(GlobalLeaderboardEntry { id, rank, time }),
            (_, _, _) => None,
        }
    }
}

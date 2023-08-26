use chrono::{DateTime, Utc};
use itertools::Itertools;
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

#[derive(Debug)]
pub struct Leaderboard(SolutionVec);

impl Leaderboard {
    pub fn new() -> Leaderboard {
        Leaderboard(SolutionVec::new())
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

    pub fn compute_diffs(&self, current_leaderboard: &Leaderboard) -> Vec<&Solution> {
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

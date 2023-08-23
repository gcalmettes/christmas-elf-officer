use chrono::{DateTime, TimeZone, Utc};
use itertools::Itertools;
use reqwest::{Client, StatusCode};
use std::fmt;
use std::ops::{Deref, DerefMut};

use std::collections::HashMap;

use crate::error::{BotError, BotResult};

enum Endpoint {
    GlobalLeaderboard(u16, u16),
    PrivateLeaderboard(u16, u64),
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Endpoint::GlobalLeaderboard(year, day) => {
                write!(f, "/{}/leaderboard/day/{}", year, day)
            }
            Endpoint::PrivateLeaderboard(year, id) => {
                write!(f, "/{}/leaderboard/private/view/{}.json", year, id)
            }
        }
    }
}

pub struct AoC {
    http_client: Client,
    base_url: String,
    session_cookie: String,
    private_leaderboard_id: u64,
}

impl AoC {
    pub fn new(
        base_url: String,
        timeout: std::time::Duration,
        private_leaderboard_id: u64,
        session_cookie: String,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            base_url,
            private_leaderboard_id,
            session_cookie,
        }
    }

    async fn get(&self, endpoint: &Endpoint, session_cookie: Option<String>) -> BotResult<String> {
        let url = format!("{}{}", self.base_url, endpoint);

        let mut request = self.http_client.get(&url);

        if let Some(session) = session_cookie {
            request = request.header("cookie", format!("session={session}"))
        }
        let response = request.send().await?;

        match response.status() {
            StatusCode::OK => response.text().await.map_err(|_| BotError::Parse),
            // AoC responds with INTERNAL_SERVER_ERROR when the session cookie is invalid.
            StatusCode::INTERNAL_SERVER_ERROR => Err(BotError::Http(format!(
                "{}. The session cookie might have expired.",
                StatusCode::INTERNAL_SERVER_ERROR
            ))),
            _ => Err(BotError::Http(format!("{}", response.status()))),
        }
    }

    async fn get_global_leaderboard(&self, year: u16, day: u16) -> BotResult<String> {
        let endpoint = Endpoint::GlobalLeaderboard(year, day);
        let resp = self.get(&endpoint, None).await?;
        Ok(resp)
    }

    pub async fn global_leaderboard(&self, year: u16, day: u16) -> BotResult<String> {
        let resp = self.get_global_leaderboard(year, day).await?;
        Ok(resp)
    }

    async fn get_private_leaderboard(&self, year: u16) -> BotResult<String> {
        let endpoint = Endpoint::PrivateLeaderboard(year, self.private_leaderboard_id);
        let resp = self
            .get(&endpoint, Some(self.session_cookie.clone()))
            .await?;
        Ok(resp)
    }

    fn parse_private_leaderboard(leaderboard: &str) -> BotResult<Leaderboard> {
        // Response from AOC private leaderboard API.
        // Defined here as it is only used by this function.
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct AOCPrivateLeaderboardResponse {
            // owner_id: u64,
            event: String,
            members: HashMap<String, AOCPrivateLeaderboardMember>,
        }

        #[derive(Debug, Deserialize)]
        struct AOCPrivateLeaderboardMember {
            /// anonymous users appear with null names in the AoC API
            name: Option<String>,
            // global_score: u64,
            local_score: u64,
            id: u64,
            // last_star_ts: u64,
            // stars: u64,
            completion_day_level:
                HashMap<String, HashMap<String, AOCPrivateLeaderboardMemberSolution>>,
        }

        #[derive(Debug, Deserialize)]
        struct AOCPrivateLeaderboardMemberSolution {
            // star_index: u64,
            get_star_ts: i64,
        }

        let parsed = serde_json::from_str::<AOCPrivateLeaderboardResponse>(&leaderboard).unwrap();
        let mut earned_stars = Leaderboard::new();

        for (_, member) in parsed.members.iter() {
            let name = match &member.name {
                Some(name) => name.to_string(),
                None => format!("anonymous user #{}", member.id),
            };

            for (day, stars) in member.completion_day_level.iter() {
                for (star, info) in stars.iter() {
                    earned_stars.push(Solution {
                        timestamp: Utc
                            .timestamp_opt(info.get_star_ts, 0)
                            .single()
                            .ok_or(BotError::Parse)?,
                        year: parsed.event.parse().map_err(|_| BotError::Parse)?,
                        day: day.parse::<u8>().map_err(|_| BotError::Parse)?,
                        part: star.parse().map_err(|_| BotError::Parse)?,
                        id: Identifier {
                            name: name.clone(),
                            numeric: member.id,
                            local_score: member.local_score,
                        },
                    });
                }
            }
        }

        // Solutions are sorted chronologically
        earned_stars.sort_unstable();

        Ok(earned_stars)
    }

    pub async fn private_leaderboard(&self, year: u16) -> BotResult<Leaderboard> {
        let leaderboard_response = self.get_private_leaderboard(year).await?;
        let leaderboard = AoC::parse_private_leaderboard(&leaderboard_response)?;
        Ok(leaderboard)
    }
}

// Puzzle completion events parsed from AoC API.
// Year and day fields match corresponding components of DateTime<Utc>.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Solution {
    timestamp: DateTime<Utc>,
    year: i32,
    day: u8,
    part: u8,
    id: Identifier,
}

// unique identifier for a participant on this leaderboard
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
struct Identifier {
    name: String,
    numeric: u64,
    local_score: u64,
}

type Underlying = Vec<Solution>;

#[derive(Debug)]
pub struct Leaderboard(Underlying);

impl Leaderboard {
    fn new() -> Leaderboard {
        Leaderboard(Underlying::new())
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
        // Max point per solution is number of players
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

    pub fn standings_by_local_score(&self) -> Vec<(String, usize)> {
        let scores = self.local_scores_per_member();

        scores
            .into_iter()
            .sorted_by_key(|x| x.1)
            .rev()
            .map(|(id, score)| (id.name.clone(), score))
            .collect::<Vec<(String, usize)>>()
    }
}

impl Deref for Leaderboard {
    type Target = Underlying;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Leaderboard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// data.into_iter()
//         .into_group_map_by(|x| x.0)
//         .into_iter()
//         .map(|(key, values)| (key, values.into_iter().fold(0,|acc, (_,v)| acc + v )))
//         .collect::<HashMap<u32,u32>>()[&0]

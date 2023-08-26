use chrono::{TimeZone, Utc};
use reqwest::{Client, StatusCode};
use std::fmt;

use std::collections::HashMap;

use crate::aoc::leaderboard::{Identifier, Leaderboard, Solution};
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

struct AoCSettings {
    base_url: String,
    timeout: std::time::Duration,
    private_leaderboard_id: u64,
    session_cookie: String,
}

fn get_default_settings() -> AoCSettings {
    AoCSettings {
        base_url: "http://localhost:5001".to_string(),
        timeout: std::time::Duration::new(5, 0),
        private_leaderboard_id: 261166,
        session_cookie: "yolo".to_string(),
    }
}

impl AoC {
    pub fn new() -> Self {
        let settings = get_default_settings();
        let http_client = Client::builder().timeout(settings.timeout).build().unwrap();
        Self {
            http_client,
            base_url: settings.base_url,
            private_leaderboard_id: settings.private_leaderboard_id,
            session_cookie: settings.session_cookie,
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
            global_score: u64,
            // local_score: u64,
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
                            global_score: member.global_score,
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

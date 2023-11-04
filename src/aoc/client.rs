use crate::{
    aoc::leaderboard::{Entry, Identifier, Leaderboard, ProblemPart, ScrapedLeaderboard},
    config,
    error::{BotError, BotResult},
};
use chrono::{TimeZone, Utc};
use reqwest::{Client, StatusCode};
use scraper::{Html, Selector};
use std::{collections::HashMap, fmt};

enum Endpoint {
    GlobalLeaderboard(i32, u8),
    DailyChallenge(i32, u8),
    PrivateLeaderboard(i32, u64),
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Endpoint::GlobalLeaderboard(year, day) => {
                write!(f, "/{}/leaderboard/day/{}", year, day)
            }
            Endpoint::DailyChallenge(year, day) => {
                write!(f, "/{}/day/{}", year, day)
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
    pub fn new() -> Self {
        let settings = &config::SETTINGS;
        let http_client = Client::builder()
            .timeout(std::time::Duration::new(settings.aoc_api_timeout_sec, 0))
            .build()
            .unwrap();
        Self {
            http_client,
            base_url: settings.aoc_base_url.clone(),
            private_leaderboard_id: settings.aoc_private_leaderboard_id,
            session_cookie: settings.aoc_session_cookie.clone(),
        }
    }

    pub async fn global_leaderboard(&self, year: i32, day: u8) -> BotResult<ScrapedLeaderboard> {
        let leaderboard_response = self.get_global_leaderboard(year, day).await?;
        let leaderboard = AoC::parse_global_leaderboard(&leaderboard_response, year, day)?;
        Ok(ScrapedLeaderboard {
            timestamp: Utc::now(),
            leaderboard,
        })
    }

    pub async fn private_leaderboard(&self, year: i32) -> BotResult<ScrapedLeaderboard> {
        let leaderboard_response = self.get_private_leaderboard(year).await?;
        let leaderboard = AoC::parse_private_leaderboard(&leaderboard_response)?;
        Ok(ScrapedLeaderboard {
            timestamp: Utc::now(),
            leaderboard,
        })
    }

    pub async fn daily_challenge(&self, year: i32, day: u8) -> BotResult<String> {
        let daily_challenge = self.get_daily_challenge(year, day).await?;
        let title = AoC::parse_daily_challenge_title(&daily_challenge)?;
        Ok(title)
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

    async fn get_global_leaderboard(&self, year: i32, day: u8) -> BotResult<String> {
        let endpoint = Endpoint::GlobalLeaderboard(year, day);
        let resp = self.get(&endpoint, None).await?;
        Ok(resp)
    }

    async fn get_daily_challenge(&self, year: i32, day: u8) -> BotResult<String> {
        let endpoint = Endpoint::DailyChallenge(year, day);
        let resp = self.get(&endpoint, None).await?;
        Ok(resp)
    }

    async fn get_private_leaderboard(&self, year: i32) -> BotResult<String> {
        let endpoint = Endpoint::PrivateLeaderboard(year, self.private_leaderboard_id);
        let resp = self
            .get(&endpoint, Some(self.session_cookie.clone()))
            .await?;
        Ok(resp)
    }

    fn parse_daily_challenge_title(challenge: &str) -> BotResult<String> {
        let document = Html::parse_document(&challenge);
        let selector_title = Selector::parse(r#"article.day-desc > h2"#).unwrap();

        let default = "N/A";
        let title = document
            .select(&selector_title)
            .next()
            .map_or(default, |node| node.text().next().unwrap_or(default));
        Ok(title.to_string())
    }

    fn parse_global_leaderboard(leaderboard: &str, year: i32, day: u8) -> BotResult<Leaderboard> {
        // The HTML document is organized like so:
        //
        //      <p>First hundred users to get <span class="leaderboard-daydesc-both">both stars</span> on Day XX:</p>
        //      <div>some entry</div>
        //      <div>some entry</div>
        //      <p>First hundred users to get the <span class="leaderboard-daydesc-first">first star</span> on Day XX:</p>
        //      <div>some entry</div>
        //      <div>some entry</div>
        //
        // Instead of just selecting all the div and having some logic based on parsing to get the
        // entries associated with the first or second part, we will directly extract the part
        // information based on the siblings of the <p> elements.

        let document = Html::parse_document(&leaderboard);
        let selector_first_part = Selector::parse(r#"span.leaderboard-daydesc-first"#).unwrap();
        let selector_second_part = Selector::parse(r#"span.leaderboard-daydesc-both"#).unwrap();

        // Entries first part. The selector will only give us the div below the p>span.leaderboard-daydesc-first element
        let entries_first = document
            .select(&selector_first_part)
            .last()
            .map_or(vec![], |span| {
                span.parent().map_or(vec![], |p| {
                    p.next_siblings()
                        .filter_map(|entry| scraper::element_ref::ElementRef::wrap(entry))
                        .filter_map(|entry| Entry::from_html(entry, year, day, ProblemPart::FIRST))
                        .collect::<Vec<Entry>>()
                })
            });

        // Because the p>span.leaderboard-daydesc-both element is at the top, the selector will give us the entry divs for both parts.
        // We will need to filter out entries already matched in first part.
        let entries_second = document
            .select(&selector_second_part)
            .last()
            .map_or(vec![], |span| {
                span.parent().map_or(vec![], |p| {
                    p.next_siblings()
                        .filter_map(|entry| scraper::element_ref::ElementRef::wrap(entry))
                        .filter_map(|entry| Entry::from_html(entry, year, day, ProblemPart::SECOND))
                        // Filter out entries of first part.
                        .filter(|e| {
                            !entries_first.contains(&Entry {
                                id: e.id.clone(),
                                timestamp: e.timestamp,
                                rank: e.rank,
                                day: e.day,
                                year: e.year,
                                part: ProblemPart::FIRST,
                            })
                        })
                        .collect::<Vec<Entry>>()
                })
            });

        let mut all_entries = Leaderboard::new();
        all_entries.extend(entries_first);
        all_entries.extend(entries_second);

        Ok(all_entries)
    }

    fn parse_private_leaderboard(leaderboard: &str) -> BotResult<Leaderboard> {
        // Response from AOC private leaderboard API.
        // Structs defined here as it is only used by this function.
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
            // local_score: u64,
            id: u64,
            // last_star_ts: u64,
            // stars: u64,
            completion_day_level:
                HashMap<String, HashMap<String, AOCPrivateLeaderboardMemberEntry>>,
        }

        #[derive(Debug, Deserialize)]
        struct AOCPrivateLeaderboardMemberEntry {
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
                    let star = star.parse().map_err(|_| BotError::Parse)?;
                    earned_stars.insert(Entry {
                        timestamp: Utc
                            .timestamp_opt(info.get_star_ts, 0)
                            .single()
                            .ok_or(BotError::Parse)?,
                        year: parsed.event.parse().map_err(|_| BotError::Parse)?,
                        day: day.parse::<u8>().map_err(|_| BotError::Parse)?,
                        part: ProblemPart::from(star),
                        rank: None,
                        id: Identifier {
                            name: name.clone(),
                            numeric: member.id,
                        },
                    });
                }
            }
        }

        Ok(earned_stars)
    }
}

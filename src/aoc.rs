use reqwest::{Client, StatusCode};
use std::fmt;

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

    pub async fn global_leaderboard(&self, year: u16, day: u16) -> BotResult<String> {
        let endpoint = Endpoint::GlobalLeaderboard(year, day);
        let resp = self.get(&endpoint, None).await?;
        Ok(resp)
    }

    pub async fn private_leaderboard(&self, year: u16) -> BotResult<String> {
        let endpoint = Endpoint::PrivateLeaderboard(year, self.private_leaderboard_id);
        let resp = self
            .get(&endpoint, Some(self.session_cookie.clone()))
            .await?;
        Ok(resp)
    }
}

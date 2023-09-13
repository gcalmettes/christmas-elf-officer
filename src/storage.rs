use std::sync::{Arc, Mutex};

use crate::aoc::leaderboard::ScrapedPrivateLeaderboard;

type SharedLeaderboard = Arc<Mutex<ScrapedPrivateLeaderboard>>;

#[derive(Clone)]
pub struct MemoryCache {
    pub data: SharedLeaderboard,
}

impl MemoryCache {
    pub fn new() -> MemoryCache {
        MemoryCache {
            data: Arc::new(Mutex::new(ScrapedPrivateLeaderboard::new())),
        }
    }
}

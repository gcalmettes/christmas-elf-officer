use crate::core::leaderboard::ScrapedLeaderboard;
use std::sync::{Arc, Mutex};

type SharedLeaderboard = Arc<Mutex<ScrapedLeaderboard>>;

#[derive(Clone)]
pub struct MemoryCache {
    pub data: SharedLeaderboard,
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryCache {
    pub fn new() -> MemoryCache {
        MemoryCache {
            data: Arc::new(Mutex::new(ScrapedLeaderboard::new())),
        }
    }
}

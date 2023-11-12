use crate::core::leaderboard::ScrapedLeaderboard;
use std::sync::{Arc, Mutex};

type SharedLeaderboard = Arc<Mutex<ScrapedLeaderboard>>;

#[derive(Clone)]
pub struct MemoryCache {
    pub data: SharedLeaderboard,
}

impl MemoryCache {
    pub fn new() -> MemoryCache {
        MemoryCache {
            data: Arc::new(Mutex::new(ScrapedLeaderboard::new())),
        }
    }
}

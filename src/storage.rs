use std::sync::{Arc, Mutex};

use crate::aoc::leaderboard::Leaderboard;

type SharedLeaderboard = Arc<Mutex<Leaderboard>>;

#[derive(Clone)]
pub struct MemoryCache {
    pub data: SharedLeaderboard,
}

impl MemoryCache {
    pub fn new() -> MemoryCache {
        MemoryCache {
            data: Arc::new(Mutex::new(Leaderboard::new())),
        }
    }
}

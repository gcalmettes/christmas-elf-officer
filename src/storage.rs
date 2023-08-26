// use chrono::Duration;
// use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use crate::aoc::leaderboard::Leaderboard;

type CacheDatabase = Arc<Mutex<Leaderboard>>;

#[derive(Clone)]
pub struct Cache {
    pub data: CacheDatabase,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            data: Arc::new(Mutex::new(Leaderboard::new())),
        }
    }
}

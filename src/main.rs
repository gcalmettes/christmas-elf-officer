use chrono::{Timelike, Utc};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, Level};

use messaging::client::AoCSlackClient;
use messaging::events::Event;
use scheduler::{JobProcess, Scheduler};
use storage::MemoryCache;

pub mod aoc;
pub mod config;
pub mod error;
pub mod messaging;
pub mod scheduler;
pub mod storage;
pub mod utils;

#[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let settings = &config::SETTINGS;

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(settings.get_trace_level())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    // Capacity of 64 should be more than plenty to handle all the messages
    let (tx, mut rx) = mpsc::channel::<Event>(64);

    // Retrieve current minute to initialize schedule of private leaderbaord updates.
    // AoC API rules states to not fetch leaderboard at a frequency higher than 15min.
    let now = Utc::now();
    let now_minute = now.minute();
    let now_second = now.second();

    // At every 15th minute from (now_minute % 15) through 59.
    let private_leaderboard_schedule = format!("{} {}/15 * 1-25 12 *", now_second, now_minute % 15);

    // Initialize global cache
    let cache = MemoryCache::new();

    let sched = Scheduler::new(cache.clone(), Arc::new(tx.clone())).await?;

    let jobs = vec![
        JobProcess::InitializePrivateLeaderboard, // only ran once, at startup.
        // JobProcess::UpdatePrivateLeaderboard(&private_leaderboard_schedule),
        JobProcess::UpdatePrivateLeaderboard("1/8 * * * * *"),
        // JobProcess::InitializeDailySolutionsThread("1/15 * * * * *"),
        // JobProcess::WatchGlobalLeaderboard("1/30 * * * * *"),
        JobProcess::ParseDailyChallenge("1/5 * * * * *"),
    ];
    for job in jobs {
        sched.add_job(job).await?;
    }

    info!("Starting scheduler.");
    sched.start().await?;

    info!("Initializing messaging engine.");
    let slack_client = AoCSlackClient::new();
    slack_client
        .handle_messages_and_events(cache, tx, rx)
        .await?;
    Ok(())
}

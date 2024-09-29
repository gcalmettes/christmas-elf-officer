use chrono::{Timelike, Utc};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use client::slack::AoCSlackClient;
use core::events::Event;
use scheduler::{JobProcess, Scheduler};
use storage::MemoryCache;

pub mod cli;
pub mod client;
pub mod config;
pub mod core;
pub mod error;
pub mod scheduler;
pub mod storage;
pub mod utils;

#[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let settings = &config::SETTINGS;

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(settings.get_trace_level())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    // Silencing the warning, as removing the mut here would actually break compilation.
    #[allow(unused_mut)]
    // Capacity of 64 should be more than plenty to handle all the messages
    let (tx, mut rx) = mpsc::channel::<Event>(64);

    // Retrieve current minute to initialize schedule of private leaderbaord updates.
    // AoC API rules states to not fetch leaderboard at a frequency higher than 15min.
    let now = Utc::now();
    let now_minute = now.minute();
    let now_second = now.second();

    // At every 15th minute from (now_minute % 15) through 59.
    let private_leaderboard_schedule = format!("{} {}/15 * * 12,1 *", now_second, now_minute % 15);

    // Initialize global cache
    let cache = MemoryCache::new();

    let sched = Scheduler::new(cache.clone(), Arc::new(tx.clone())).await?;

    let jobs = vec![
        JobProcess::InitializePrivateLeaderboard, // only ran once, at startup.
        JobProcess::UpdatePrivateLeaderboard(&private_leaderboard_schedule),
        JobProcess::InitializeDailySolutionsThread("0 30 8 1-25 12 *"),
        JobProcess::WatchGlobalLeaderboard("0 0 5 1-25 12 *"),
        JobProcess::ParseDailyChallenge("1 0 5 1-25 12 *"),
        JobProcess::SendDailySummary("0 30 16 1-25 12 *"),
    ];
    for job in jobs {
        sched.add_job(job).await?;
    }

    info!("Starting scheduler.");
    sched.start().await?;

    info!("Initializing messaging engine.");

    let slack_client = AoCSlackClient::new().expect("Slack client could not be initialized");
    slack_client
        .handle_messages_and_events(cache, tx, rx)
        .await?;
    Ok(())
}

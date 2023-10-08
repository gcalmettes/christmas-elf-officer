use ceo_bot::messaging::client::initialize_messaging;
use ceo_bot::messaging::models::MyEvent;
use ceo_bot::scheduler::{JobProcess, Scheduler};

use tokio::sync::mpsc;

use chrono::{Timelike, Utc};
use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    // Capacity of 64 should be more than plenty to handle all the messages
    let (tx, mut rx) = mpsc::channel::<MyEvent>(64);

    // Retrieve current minute to initialize schedule of private leaderbaord updates.
    // AoC API rules states to not fetch leaderboard at a frequency higher than 15min.
    let now = Utc::now();
    let now_minute = now.minute();
    let now_second = now.second();

    // At every 15th minute from (now_minute % 15) through 59.
    let private_leaderboard_schedule = format!("{} {}/15 * 1-25 12 *", now_second, now_minute % 15);

    let sched = Scheduler::new(Arc::new(tx)).await?;

    let jobs = vec![
        JobProcess::InitializePrivateLeaderboard, // only ran once, at startup.
        // JobProcess::UpdatePrivateLeaderboard(&private_leaderboard_schedule),
        JobProcess::UpdatePrivateLeaderboard("1/8 * * * * *"),
        JobProcess::WatchGlobalLeaderboard("1/20 * * * * *"),
    ];
    for job in jobs {
        sched.add_job(job).await?;
    }

    info!("Starting scheduler.");
    sched.start().await?;

    info!("Initializing messaging engine.");
    initialize_messaging(rx).await?;

    Ok(())
}

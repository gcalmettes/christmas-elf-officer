use ceo_bot::messaging::client::{client_with_socket_mode, initialize_messaging};
use ceo_bot::messaging::models::MyEvent;
use ceo_bot::scheduler::{JobProcess, Scheduler};
use tokio::time::{sleep, Duration};

use tokio::sync::mpsc;

use chrono::{Timelike, Utc};
use cron::Schedule;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    let (tx, mut rx) = mpsc::unbounded_channel::<MyEvent>();

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

    // Start the scheduler
    sched.start().await?;

    info!("Connecting to slack in socket mode");
    // let matterbridge = MatterBridgeClient::new("http://localhost:4243".to_string());
    // let _stream = matterbridge.acquire_stream().await?;

    // let subscriber = tracing_subscriber::fmt()
    //     .with_env_filter("slack_morphism=info,tokio_cron_scheduler=debug")
    //     .finish();

    // // Handle posting message from internal aoc events
    // tokio::spawn(async move {
    //     while let Some(message) = rx.recv().await {
    //         println!("Your message event: {:?}", message); // This is where you handle everything one by one now
    //     }
    // });

    // // Handle message from users
    // client_with_socket_mode().await?;
    initialize_messaging(rx).await?;

    // loop {
    //     println!(">> Attempting stream acquisition");
    //     // let _stream = matterbridge.read_stream().await?;
    //     match matterbridge.read_stream().await {
    //         Ok(_r) => {
    //             ()
    //             // println!("Connected ...");
    //         }
    //         Err(e) => {
    //             println!(">>> {:?}", e);
    //             println!("[Lost stream] retrying in 5 secs ...");
    //         }
    //     }
    //     sleep(Duration::from_millis(5000)).await;
    // }

    // loop {
    //     let matterbridge = MatterBridgeClient::new("http://localhost:4243".to_string());
    //     let stream = matterbridge.read_stream();
    //     let res = tokio::spawn(stream).await;
    //     match res {
    //         Ok(output) => { /* handle successfull exit */ }
    //         Err(err) if err.is_panic() => {
    //             /* handle panic in task, e.g. by going around loop to restart task */
    //             println!("YOLO ERROR")
    //         }
    //         Err(err) => {
    //             /* handle other errors (mainly runtime shutdown) */
    //             println!("ABA ERROR")
    //         }
    //     }
    // }

    // loop {
    //     println!(">> Attempting stream acquisition");
    //     let matterbridge = MatterBridgeClient::new("http://localhost:4243".to_string());
    //     let _stream = matterbridge.acquire_stream().await?;
    //     println!("[Lost stream] retrying ...");
    //     sleep(Duration::from_millis(5000)).await;
    // }

    // while let Some(msg) = stream.read_line(&mut line).await {
    //     println!("MSG: {:?}", msg)
    // }
    // while let Some(msg) = stream.next() {
    //     println!("MSG: {:?}", msg)
    // }
    // // Wait while the jobs run

    // loop {
    //     let size = sched.cache_size();
    //     let ref_count = sched.ref_count();

    //     println!("[{:?}] {:?}", size, ref_count);
    //     sleep(Duration::from_millis(5000)).await;
    // }

    //               sec  min   hour   day of month   month   day of week   year
    // let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";

    // let schedule = Schedule::from_str(&private_leaderboard_schedule).unwrap();
    // println!("Upcoming fire times:");
    // for datetime in schedule.upcoming(Utc).take(10) {
    //     println!("-> {}", datetime);
    // }

    Ok(())
}

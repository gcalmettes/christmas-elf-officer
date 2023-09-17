use ceo_bot::messaging::client::MatterBridgeClient;
use ceo_bot::scheduler::{JobProcess, Scheduler};
use tokio::time::{sleep, Duration};

use chrono::{Timelike, Utc};
use cron::Schedule;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Retrieve current minute to initialize schedule of private leaderbaord updates.
    // AoC API rules states to not fetch leaderboard at a frequency higher than 15min.
    let now = Utc::now();
    let now_minute = now.minute();
    let now_second = now.second();

    // At every 15th minute from (now_minute % 15) through 59.
    let private_leaderboard_schedule = format!("{} {}/15 * 1-25 12 *", now_second, now_minute % 15);

    let sched = Scheduler::new().await?;

    let jobs = vec![
        JobProcess::InitializePrivateLeaderboard, // only ran once, at startup.
        JobProcess::UpdatePrivateLeaderboard(&private_leaderboard_schedule),
        // JobProcess::UpdatePrivateLeaderboard("1/8 * * * * *"),
        JobProcess::WatchGlobalLeaderboard("1/20 * * * * *"),
    ];
    for job in jobs {
        sched.add_job(job).await?;
    }

    // Start the scheduler
    sched.start().await?;

    println!(">> Attempting stream acquisition");
    let matterbridge = MatterBridgeClient::new("http://localhost:4243".to_string());
    // let _stream = matterbridge.acquire_stream().await?;

    loop {
        println!(">> Attempting stream acquisition");
        // let _stream = matterbridge.read_stream().await?;
        match matterbridge.read_stream().await {
            Ok(_r) => {
                println!("Connected ...");
            }
            Err(e) => {
                println!(">>> {:?}", e);
                println!("[Lost stream] retrying ...");
            }
        }
        sleep(Duration::from_millis(5000)).await;
    }

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

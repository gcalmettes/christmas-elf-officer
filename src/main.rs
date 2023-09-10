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
        // JobProcess::UpdatePrivateLeaderboard(&private_leaderboard_schedule),
        JobProcess::WatchGlobalLeaderboard("1/20 * * * * *"),
    ];
    for job in jobs {
        sched.add_job(job).await?;
    }

    // Start the scheduler
    sched.start().await?;

    // Wait while the jobs run
    loop {
        let size = sched.cache_size();
        let ref_count = sched.ref_count();

        println!("[{:?}] {:?}", size, ref_count);
        sleep(Duration::from_millis(5000)).await;
    }

    //               sec  min   hour   day of month   month   day of week   year
    // let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";

    // let schedule = Schedule::from_str(&private_leaderboard_schedule).unwrap();
    // println!("Upcoming fire times:");
    // for datetime in schedule.upcoming(Utc).take(10) {
    //     println!("-> {}", datetime);
    // }

    Ok(())
}

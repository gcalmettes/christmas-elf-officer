use ceo_bot::scheduler::{JobProcess, Scheduler};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sched = Scheduler::new().await?;

    let jobs = vec![
        JobProcess::UpdatePrivateLeaderboard("1/3 * * * * *"),
        JobProcess::UpdatePrivateLeaderboard("1/6 * * * * *"),
        JobProcess::UpdatePrivateLeaderboard("1/9 * * * * *"),
        JobProcess::WatchGlobalLeaderboard("1/15 * * * * *"),
    ];
    for job in jobs {
        let _ = sched.add_job(job).await;
    }

    // Start the scheduler
    sched.start().await?;

    // Wait while the jobs run
    loop {
        let size = sched.cache_size();
        let ref_count = sched.ref_count();

        println!("[{:?}] {:?}", size, ref_count);
        sleep(Duration::from_millis(1000)).await;
    }
}

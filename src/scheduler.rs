use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use std::sync::Arc;

use crate::aoc::client::AoC;
use crate::storage::Cache;

pub enum JobProcess<'schedule> {
    UpdatePrivateLeaderboard(&'schedule str),
    WatchGlobalLeaderboard(&'schedule str),
}

pub async fn update_private_leaderboard_job(
    schedule: &str,
    cache: Cache,
) -> Result<Job, JobSchedulerError> {
    Job::new_async(schedule, move |uuid, mut l| {
        let cache = cache.clone();

        Box::pin(async move {
            let aoc_client = AoC::new();
            match aoc_client.private_leaderboard(2022).await {
                Ok(leaderboard) => {
                    let mut data = cache.data.lock().unwrap();
                    *data = leaderboard;
                }
                Err(e) => {
                    // err
                    ()
                }
            };

            // Query the next execution time for this job
            let next_tick = l.next_tick_for_job(uuid).await;
            match next_tick {
                Ok(Some(ts)) => println!(">> Next refresh leaderboard at {:?}", ts),
                _ => println!(">> Could not get next tick for refresh leaderboard job"),
            }
        })
    })
}

pub async fn watch_global_leaderboard_job(
    schedule: &str,
    _cache: Cache,
) -> Result<Job, JobSchedulerError> {
    Job::new_async(schedule, move |uuid, mut l| {
        Box::pin(async move {
            // Query the next execution time for this job
            let next_tick = l.next_tick_for_job(uuid).await;
            match next_tick {
                Ok(Some(ts)) => println!(">> Next global leaderboard watch at {:?}", ts),
                _ => println!(">> Could not get next tick for global leaderboard job"),
            }
        })
    })
}

pub struct Scheduler {
    scheduler: JobScheduler,
    cache: Cache,
}

impl Scheduler {
    pub async fn new() -> Result<Self, JobSchedulerError> {
        let scheduler = JobScheduler::new().await?;
        let cache = Cache::new();
        Ok(Scheduler { scheduler, cache })
    }

    pub async fn add_job(
        &self,
        job_process: JobProcess<'_>,
    ) -> Result<uuid::Uuid, JobSchedulerError> {
        let job = match job_process {
            JobProcess::UpdatePrivateLeaderboard(schedule) => {
                update_private_leaderboard_job(schedule, self.cache.clone()).await?
            }
            JobProcess::WatchGlobalLeaderboard(schedule) => {
                watch_global_leaderboard_job(schedule, self.cache.clone()).await?
            }
        };
        self.scheduler.add(job).await
    }

    pub async fn start(&self) -> Result<(), JobSchedulerError> {
        self.scheduler.start().await
    }

    pub fn cache_size(&self) -> usize {
        let data = self.cache.data.lock().unwrap();
        data.len()
    }

    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.cache.data)
    }
}

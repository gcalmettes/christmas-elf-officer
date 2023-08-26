use tokio_cron_scheduler::{Job, JobScheduler};

use std::sync::Arc;

use crate::aoc::client::AoC;
use crate::error::{BotError, BotResult};
use crate::storage::MemoryCache;

pub enum JobProcess<'schedule> {
    UpdatePrivateLeaderboard(&'schedule str),
    WatchGlobalLeaderboard(&'schedule str),
}

pub async fn update_private_leaderboard_job(schedule: &str, cache: MemoryCache) -> BotResult<Job> {
    let job = Job::new_async(schedule, move |uuid, mut l| {
        let cache = cache.clone();

        Box::pin(async move {
            let aoc_client = AoC::new();
            match aoc_client.private_leaderboard(2022).await {
                Ok(leaderboard) => {
                    let mut data = cache.data.lock().unwrap();
                    *data = leaderboard;
                }
                Err(e) => {
                    let error = BotError::AOC(format!("Could not retrieve leaderboard. {e}"));
                    println!("{}", error);
                }
            };

            // Query the next execution time for this job
            let next_tick = l.next_tick_for_job(uuid).await;
            match next_tick {
                Ok(Some(ts)) => println!(">> Next refresh leaderboard at {:?}", ts),
                _ => println!(">> Could not get next tick for refresh leaderboard job"),
            }
        })
    })?;
    Ok(job)
}

pub async fn watch_global_leaderboard_job(schedule: &str, _cache: MemoryCache) -> BotResult<Job> {
    let job = Job::new_async(schedule, |uuid, mut l| {
        // let cache = cache.clone();

        Box::pin(async move {
            let _aoc_client = AoC::new();

            // Query the next execution time for this job
            let next_tick = l.next_tick_for_job(uuid).await;
            match next_tick {
                Ok(Some(ts)) => println!(">> Next refresh leaderboard at {:?}", ts),
                _ => println!(">> Could not get next tick for refresh leaderboard job"),
            }
        })
    })?;
    Ok(job)
}

pub struct Scheduler {
    scheduler: JobScheduler,
    cache: MemoryCache,
}

impl Scheduler {
    pub async fn new() -> BotResult<Self> {
        let cache = MemoryCache::new();
        let scheduler = JobScheduler::new().await?;
        Ok(Scheduler { scheduler, cache })
    }

    pub async fn add_job(&self, job_process: JobProcess<'_>) -> BotResult<uuid::Uuid> {
        let job = match job_process {
            JobProcess::UpdatePrivateLeaderboard(schedule) => {
                update_private_leaderboard_job(schedule, self.cache.clone()).await?
            }
            JobProcess::WatchGlobalLeaderboard(schedule) => {
                watch_global_leaderboard_job(schedule, self.cache.clone()).await?
            }
        };
        Ok(self.scheduler.add(job).await?)
    }

    pub async fn start(&self) -> BotResult<()> {
        Ok(self.scheduler.start().await?)
    }

    pub fn cache_size(&self) -> usize {
        let data = self.cache.data.lock().unwrap();
        data.len()
    }

    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.cache.data)
    }
}

use tokio_cron_scheduler::{Job, JobScheduler};

use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time;
use tracing::{error, info};

use std::sync::Arc;

use crate::aoc::client::AoC;
use crate::error::{BotError, BotResult};
use crate::messaging::models::Event;
use crate::storage::MemoryCache;

pub struct Scheduler {
    scheduler: JobScheduler,
    cache: MemoryCache,
    sender: Arc<Sender<Event>>, // communication to messaging service
}

pub enum JobProcess<'schedule> {
    InitializePrivateLeaderboard,
    UpdatePrivateLeaderboard(&'schedule str),
    WatchGlobalLeaderboard(&'schedule str),
}

impl Scheduler {
    pub async fn new(cache: MemoryCache, sender: Arc<Sender<Event>>) -> BotResult<Self> {
        // let cache = MemoryCache::new();
        let scheduler = JobScheduler::new().await?;
        Ok(Scheduler {
            scheduler,
            cache,
            sender,
        })
    }

    pub async fn add_job(&self, job_process: JobProcess<'_>) -> BotResult<uuid::Uuid> {
        let job = match job_process {
            JobProcess::InitializePrivateLeaderboard => {
                initialize_private_leaderboard_job(self.cache.clone()).await?
            }
            JobProcess::UpdatePrivateLeaderboard(schedule) => {
                update_private_leaderboard_job(schedule, self.cache.clone(), self.sender.clone())
                    .await?
            }
            JobProcess::WatchGlobalLeaderboard(schedule) => {
                watch_global_leaderboard_job(schedule, self.cache.clone(), self.sender.clone())
                    .await?
            }
        };
        Ok(self.scheduler.add(job).await?)
    }

    pub async fn start(&self) -> BotResult<()> {
        Ok(self.scheduler.start().await?)
    }

    pub fn cache_size(&self) -> usize {
        let data = self.cache.data.lock().unwrap();
        data.leaderboard.len()
    }

    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.cache.data)
    }
}

//////////////////
// Jobs definition
//////////////////

async fn initialize_private_leaderboard_job(cache: MemoryCache) -> BotResult<Job> {
    let job = Job::new_one_shot_async(Duration::from_secs(0), move |_uuid, _l| {
        let cache = cache.clone();
        Box::pin(async move {
            let aoc_client = AoC::new();
            match aoc_client.private_leaderboard(2022).await {
                Ok(scraped_leaderboard) => {
                    let mut data = cache.data.lock().unwrap();
                    *data = scraped_leaderboard;
                }
                Err(e) => {
                    let error = BotError::AOC(format!("Could not scrape leaderboard. {e}"));
                    error!("{error}");
                }
            };
        })
    })?;
    Ok(job)
}

async fn update_private_leaderboard_job(
    schedule: &str,
    cache: MemoryCache,
    sender: Arc<Sender<Event>>,
) -> BotResult<Job> {
    let job = Job::new_async(schedule, move |uuid, mut l| {
        let cache = cache.clone();
        let sender = sender.clone();
        Box::pin(async move {
            let aoc_client = AoC::new();
            match aoc_client.private_leaderboard(2022).await {
                Ok(scraped_leaderboard) => {
                    // Scoped to force 'data' to drop before 'await' so future can be Send
                    {
                        let mut data = cache.data.lock().unwrap();
                        *data = scraped_leaderboard;
                    }
                    if let Err(e) = sender.send(Event::PrivateLeaderboardUpdated).await {
                        let error = BotError::ChannelSend(format!(
                            "Could not send message to MPSC channel. {e}"
                        ));
                        error!("{error}");
                    };
                }
                Err(e) => {
                    let error = BotError::AOC(format!("Could not scrape leaderboard. {e}"));
                    error!("{error}");
                }
            };

            // Query the next execution time for this job
            let next_tick = l.next_tick_for_job(uuid).await;
            match next_tick {
                Ok(Some(ts)) => info!("Next refresh for private leaderboard at {:?}", ts),
                _ => error!("Could not get next tick for refresh private leaderboard job"),
            }
        })
    })?;
    Ok(job)
}

async fn watch_global_leaderboard_job(
    schedule: &str,
    cache: MemoryCache,
    sender: Arc<Sender<Event>>,
) -> BotResult<Job> {
    let job = Job::new_async(schedule, move |uuid, mut l| {
        let cache = cache.clone();
        let sender = sender.clone();

        Box::pin(async move {
            let aoc_client = AoC::new();

            //TODO: set interval to what we want
            let mut interval = time::interval(Duration::from_secs(3));

            let mut global_leaderboard_is_complete = false;
            while !global_leaderboard_is_complete {
                info!("GLobal leaderboard not complete");
                //TODO: Set year and day programmatically from Utc::now()
                match aoc_client.global_leaderboard(2022, 10).await {
                    Ok(scraped_leaderboard) => {
                        info!(
                            "Global Leaderboard is complete {}",
                            scraped_leaderboard.is_complete()
                        );
                        global_leaderboard_is_complete = scraped_leaderboard.is_complete();

                        if global_leaderboard_is_complete {
                            if let Err(e) = sender.send(Event::GlobalLeaderboardComplete).await {
                                let error = BotError::ChannelSend(format!(
                                    "Could not send message to MPSC channel. {e}"
                                ));
                                error!("{error}");
                            };
                        }

                        // Scoped to not held data across .await
                        let heroes = {
                            // check if private members made it to the global leaderboard
                            let private_leaderboard = cache.data.lock().unwrap();
                            scraped_leaderboard
                                .look_for_private_members(&private_leaderboard.leaderboard)
                        };

                        // TODO: replace with function that sends message to matterbridge
                        for hero in heroes {
                            if let Err(e) = sender
                                .send(Event::GlobalLeaderboardHeroFound(hero.name))
                                .await
                            {
                                let error = BotError::ChannelSend(format!(
                                    "Could not send message to MPSC channel. {e}"
                                ));
                                error!("{error}");
                            };
                        }
                    }
                    Err(e) => {
                        let error =
                            BotError::AOC(format!("Could not scrape global leaderboard. {e}"));
                        error!("{error}");
                    }
                };

                interval.tick().await;
            }
        })
    })?;
    Ok(job)
}

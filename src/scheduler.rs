use chrono::{Datelike, Utc};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use std::sync::Arc;

use crate::aoc::client::AoC;
use crate::aoc::leaderboard::{Identifier, ProblemPart};
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
    InitializeDailySolutionsThread(&'schedule str),
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
            JobProcess::InitializeDailySolutionsThread(schedule) => {
                initialize_daily_solutions_thread_job(schedule, self.sender.clone()).await?
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

async fn initialize_daily_solutions_thread_job(
    schedule: &str,
    sender: Arc<Sender<Event>>,
) -> BotResult<Job> {
    let job = Job::new_async(schedule, move |_uuid, _l| {
        let sender = sender.clone();
        Box::pin(async move {
            let now = Utc::now();
            let day = now.day();
            if let Err(e) = sender
                .send(Event::DailySolutionsThreadToInitialize(day))
                .await
            {
                let error =
                    BotError::ChannelSend(format!("Could not send message to MPSC channel. {e}"));
                error!("{error}");
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

            //TODO: Set year and day programmatically from Utc::now()

            // let now = Utc::now();
            // let year = now.year();
            // let day = now.day() as u8;

            let (year, day) = (2022, 9);

            let mut known_hero_hits: Vec<(Identifier, ProblemPart)> = vec![];

            info!("Starting polling Global Leaderboard for day {day}.");
            let mut global_leaderboard_is_complete = false;

            while !global_leaderboard_is_complete {
                info!("Global Leaderboard for day {day} not complete yet.");
                match aoc_client.global_leaderboard(year, day).await {
                    Ok(global_leaderboard) => {
                        // 100 entries for each part, so completion is 2*100
                        global_leaderboard_is_complete = global_leaderboard.is_count_equal_to(200);

                        // Scoped to not held data across .await
                        let hero_hits = {
                            // check if private members made it to the global leaderboard
                            let private_leaderboard = cache.data.lock().unwrap();
                            global_leaderboard
                                .check_for_private_members(&private_leaderboard.leaderboard)
                        };

                        // TODO: replace with function that sends message to matterbridge
                        for hero_hit in hero_hits {
                            // If not already known, send shoutout to hero
                            if !known_hero_hits.contains(&hero_hit) {
                                let (hero, part) = &hero_hit;
                                if let Err(e) = sender
                                    .send(Event::GlobalLeaderboardHeroFound((
                                        hero.name.clone(),
                                        part.to_string(),
                                    )))
                                    .await
                                {
                                    let error = BotError::ChannelSend(format!(
                                        "Could not send message to MPSC channel. {e}"
                                    ));
                                    error!("{error}");
                                } else {
                                    // Announcement successful, let's register the hero.
                                    known_hero_hits.push(hero_hit);
                                };
                            }
                        }

                        if global_leaderboard_is_complete {
                            info!("Global Leaderboard for day {day} is complete!");

                            // TODO: send only needed data for announcement (fast and slow)
                            if let Err(e) = sender
                                .send(Event::GlobalLeaderboardComplete(
                                    global_leaderboard.statistics(year, day),
                                ))
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

use tokio_cron_scheduler::{Job, JobScheduler};

use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time;
use tracing::{error, info};

use std::sync::Arc;

use crate::aoc::client::AoC;
use crate::error::{BotError, BotResult};
use crate::messaging::models::MyEvent;
use crate::storage::MemoryCache;

pub struct Scheduler {
    scheduler: JobScheduler,
    cache: MemoryCache,
    sender: Arc<UnboundedSender<MyEvent>>, // communication to messaging service
}

pub enum JobProcess<'schedule> {
    InitializePrivateLeaderboard,
    UpdatePrivateLeaderboard(&'schedule str),
    WatchGlobalLeaderboard(&'schedule str),
}

impl Scheduler {
    pub async fn new(sender: Arc<UnboundedSender<MyEvent>>) -> BotResult<Self> {
        let cache = MemoryCache::new();
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
                    error!("{}", error);
                }
            };
        })
    })?;
    Ok(job)
}

async fn update_private_leaderboard_job(
    schedule: &str,
    cache: MemoryCache,
    sender: Arc<UnboundedSender<MyEvent>>,
) -> BotResult<Job> {
    let job = Job::new_async(schedule, move |uuid, mut l| {
        let cache = cache.clone();
        let sender = sender.clone();
        Box::pin(async move {
            let aoc_client = AoC::new();
            match aoc_client.private_leaderboard(2022).await {
                Ok(scraped_leaderboard) => {
                    let mut data = cache.data.lock().unwrap();
                    *data = scraped_leaderboard;
                    sender.send(MyEvent {
                        event: "private updated!".to_string(),
                    });
                }
                Err(e) => {
                    let error = BotError::AOC(format!("Could not scrape leaderboard. {e}"));
                    error!("{}", error);
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
    sender: Arc<UnboundedSender<MyEvent>>,
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
                match aoc_client.global_leaderboard(2022, 1).await {
                    Ok(scraped_leaderboard) => {
                        info!(
                            "Global Leaderboard is complete {}",
                            scraped_leaderboard.is_complete()
                        );
                        global_leaderboard_is_complete = scraped_leaderboard.is_complete();

                        if global_leaderboard_is_complete {
                            sender.send(MyEvent {
                                event: "Global Leaderboard Complete!".to_string(),
                            });
                        }

                        // check if private members made it to the global leaderboard
                        let private_leaderboard = cache.data.lock().unwrap();
                        let heroes = scraped_leaderboard
                            .look_for_private_members(&private_leaderboard.leaderboard);

                        // TODO: replace with function that sends message to matterbridge
                        for hero in heroes {
                            sender.send(MyEvent {
                                event: format!("HERO made the leaderboard: {}", hero.name),
                            });
                        }
                    }
                    Err(e) => {
                        let error =
                            BotError::AOC(format!("Could not scrape global leaderboard. {e}"));
                        error!("{}", error);
                    }
                };

                interval.tick().await;
            }

            ////TODO: Set year and day programmatically from Utc::now()
            //match aoc_client.global_leaderboard(2022, 1).await {
            //    Ok(scraped_leaderboard) => {
            //        // check if private members made it to the global leaderboard
            //        let private_leaderboard = cache.data.lock().unwrap();
            //        let heroes = scraped_leaderboard
            //            .look_for_private_members(&private_leaderboard.leaderboard);

            //        // TODO: replace with function that sends message to matterbridge
            //        for hero in heroes {
            //            println!("HERO made the leaderboard: {}", hero.name);
            //        }

            //        // println!(
            //        //     ">> {:?} [{:?}]",
            //        //     scraped_leaderboard.len(),
            //        //     scraped_leaderboard.is_complete()
            //        // );

            //        // let deltas_min = scraped_leaderboard.get_fastest_delta();
            //        // let deltas_max = scraped_leaderboard.get_slowest_delta();
            //        // println!(">> DELTAS: {:?}, {:?}", deltas_min, deltas_max);

            //        // let mut data = cache.data.lock().unwrap();
            //        // *data = scraped_leaderboard;
            //    }
            //    Err(e) => {
            //        let error = BotError::AOC(format!("Could not scrape global leaderboard. {e}"));
            //        eprintln!("{}", error);
            //    }
            //};

            // let mut interval = time::interval(Duration::from_secs(1));

            // let mut complete = false;
            // let mut counter = 0;
            // while !complete {
            //     interval.tick().await;
            //     println!("not complete yet");
            //     counter += 1;
            //     if counter > 5 {
            //         complete = true;
            //         println!("Complete !!");
            //     }
            // }

            // // Query the next execution time for this job
            // let next_tick = l.next_tick_for_job(uuid).await;
            // match next_tick {
            //     Ok(Some(ts)) => println!(">> Next refresh leaderboard at {:?}", ts),
            //     _ => println!(">> Could not get next tick for refresh leaderboard job"),
            // }
        })
    })?;
    Ok(job)
}

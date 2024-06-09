use crate::{
    client::aoc::AoC,
    config,
    core::{
        events::Event,
        standings::{Ranking, Standing},
    },
    error::{BotError, BotResult},
    storage::MemoryCache,
    utils::{compute_highlights, current_aoc_year_day, get_new_members},
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc::Sender, time};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

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
    ParseDailyChallenge(&'schedule str),
    SendDailySummary(&'schedule str),
}

impl Scheduler {
    pub async fn new(cache: MemoryCache, sender: Arc<Sender<Event>>) -> BotResult<Self> {
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
            JobProcess::ParseDailyChallenge(schedule) => {
                parse_daily_challenge_job(schedule, self.sender.clone()).await?
            }
            JobProcess::SendDailySummary(schedule) => {
                send_daily_summary_job(schedule, self.cache.clone(), self.sender.clone()).await?
            }
        };
        Ok(self.scheduler.add(job).await?)
    }

    pub async fn start(&self) -> BotResult<()> {
        Ok(self.scheduler.start().await?)
    }

    // pub fn cache_size(&self) -> usize {
    //     let data = self.cache.data.lock().unwrap();
    //     data.leaderboard.len()
    // }

    // pub fn ref_count(&self) -> usize {
    //     Arc::strong_count(&self.cache.data)
    // }
}

//////////////////
// Jobs definition
//////////////////

async fn initialize_private_leaderboard_job(cache: MemoryCache) -> BotResult<Job> {
    let job = Job::new_one_shot_async(Duration::from_secs(0), move |_uuid, _l| {
        let cache = cache.clone();
        Box::pin(async move {
            let aoc_client = AoC::new();
            let settings = &config::SETTINGS;

            let (current_year, _day) = current_aoc_year_day();
            let mut live_years = vec![current_year];
            if settings.all_years {
                live_years.extend(2015..current_year)
            };

            for year in live_years {
                match aoc_client.private_leaderboard(year).await {
                    Ok(scraped_leaderboard) => {
                        let mut data = cache.data.lock().unwrap();
                        data.merge_with(scraped_leaderboard);
                    }
                    Err(e) => {
                        let error = BotError::AOC(format!("Could not scrape leaderboard. {e}"));
                        error!("{error}");
                    }
                };
            }
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
            let (_year, day) = current_aoc_year_day();
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

            let (year, _day) = current_aoc_year_day();
            match aoc_client.private_leaderboard(year).await {
                Ok(scraped_leaderboard) => {
                    // Scoped to force 'current_leaderboard' to drop before 'await' so future can be Send.
                    let (highlights, new_members) = {
                        let mut current_leaderboard = cache.data.lock().unwrap();

                        // Check for new parts completions
                        let highlights = compute_highlights(
                            &current_leaderboard.leaderboard,
                            &scraped_leaderboard.leaderboard,
                        );

                        // Check for new members
                        let new_members = get_new_members(
                            &current_leaderboard.leaderboard,
                            &scraped_leaderboard.leaderboard,
                        );

                        // Update leadearboard in cache.
                        current_leaderboard.merge_with(scraped_leaderboard);

                        (highlights, new_members)
                    };

                    // Conditionnally trigger internal events, base on leaderboard processing.
                    if !new_members.is_empty() {
                        if let Err(e) = sender
                            .send(Event::PrivateLeaderboardNewMembers(new_members))
                            .await
                        {
                            let error = BotError::ChannelSend(format!(
                                "Could not send message to MPSC channel. {e}"
                            ));
                            error!("{error}");
                        };
                    }
                    if !highlights.is_empty() {
                        if let Err(e) = sender
                            .send(Event::PrivateLeaderboardNewEntries(highlights))
                            .await
                        {
                            let error = BotError::ChannelSend(format!(
                                "Could not send message to MPSC channel. {e}"
                            ));
                            error!("{error}");
                        };
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
    let job = Job::new_async(schedule, move |_uuid, _l| {
        let cache = cache.clone();
        let sender = sender.clone();

        Box::pin(async move {
            let settings = &config::SETTINGS;
            let aoc_client = AoC::new();

            let mut interval = time::interval(Duration::from_secs(
                settings.global_leaderboard_polling_interval_sec,
            ));

            // Note: the first interval tick ticks immediately, so we trigger it
            // to ensure the counter reflects interval time multiples.
            interval.tick().await;

            let (year, day) = current_aoc_year_day();

            let mut known_hero_hashes: Vec<String> = vec![];

            info!("Starting polling Global Leaderboard for day {day}.");
            let mut is_global_leaderboard_complete = false;
            let mut counter = 0;

            while !is_global_leaderboard_complete {
                match aoc_client.global_leaderboard(year, day).await {
                    Ok(global_leaderboard) => {
                        is_global_leaderboard_complete =
                            global_leaderboard.leaderboard.is_global_complete();

                        // Scoped to not held data across .await
                        let hero_entries = {
                            // check if private members made it to the global leaderboard
                            let private_leaderboard = cache.data.lock().unwrap();
                            global_leaderboard
                                .leaderboard
                                .get_common_members_with(&private_leaderboard.leaderboard)
                        };

                        for entry in hero_entries {
                            let entry_hash = entry.to_key();
                            // If not already known, send shoutout to hero
                            if !known_hero_hashes.contains(&entry_hash) {
                                // let (name, part, rank) = &hero_hit;
                                let (name, part, rank) = (
                                    entry.id.name.clone(),
                                    entry.part,
                                    entry.rank.unwrap_or_default(),
                                );
                                if let Err(e) = sender
                                    .send(Event::GlobalLeaderboardHeroFound((name, part, rank)))
                                    .await
                                {
                                    let error = BotError::ChannelSend(format!(
                                        "Could not send message to MPSC channel. {e}"
                                    ));
                                    error!("{error}");
                                } else {
                                    // Announcement successful, let's register the hero.
                                    known_hero_hashes.push(entry_hash);
                                };
                            }
                        }

                        if is_global_leaderboard_complete {
                            info!("Global Leaderboard for day {day} is now complete!");
                            match global_leaderboard
                                .leaderboard
                                .statistics_for_year_day(year, day)
                            {
                                Ok(stats) => {
                                    if let Err(e) = sender
                                        .send(Event::GlobalLeaderboardComplete((day, stats)))
                                        .await
                                    {
                                        let error = BotError::ChannelSend(format!(
                                            "Could not send message to MPSC channel. {e}"
                                        ));
                                        error!("{error}");
                                    };
                                }
                                Err(e) => {
                                    let error = BotError::Compute(format!(
                                        "Could not compute global statistics. {e}"
                                    ));
                                    error!("{error}");
                                }
                            }
                        } else {
                            info!("Global Leaderboard for day {day} not complete yet.");
                            if [5, 8, 11, 14].contains(&counter) {
                                let num_sec = interval.period().as_secs() * counter;
                                if let Err(e) = sender
                                    .send(Event::GlobalLeaderboardUpdateMessage(counter, num_sec))
                                    .await
                                {
                                    let error = BotError::ChannelSend(format!(
                                        "Could not send message to MPSC channel. {e}"
                                    ));
                                    error!("{error}");
                                };
                            }
                        }
                    }
                    Err(e) => {
                        let error =
                            BotError::AOC(format!("Could not scrape global leaderboard. {e}"));
                        error!("{error}");
                    }
                };

                counter += 1;
                interval.tick().await;
            }
        })
    })?;
    Ok(job)
}

async fn parse_daily_challenge_job(schedule: &str, sender: Arc<Sender<Event>>) -> BotResult<Job> {
    let job = Job::new_async(schedule, move |_uuid, _l| {
        let sender = sender.clone();
        Box::pin(async move {
            let aoc_client = AoC::new();

            let (year, day) = current_aoc_year_day();

            info!("Retrieving challenge title for day {day}.");

            match aoc_client.daily_challenge(year, day).await {
                Ok(title) => {
                    if let Err(e) = sender
                        .send(Event::DailyChallengeIsUp(day, title.clone()))
                        .await
                    {
                        let error = BotError::ChannelSend(format!(
                            "Could not send message to MPSC channel. {e}"
                        ));
                        error!("{error}");
                    };
                }
                Err(e) => {
                    let error = BotError::AOC(format!("Could not scrape global leaderboard. {e}"));
                    error!("{error}");
                }
            };
        })
    })?;
    Ok(job)
}

async fn send_daily_summary_job(
    schedule: &str,
    cache: MemoryCache,
    sender: Arc<Sender<Event>>,
) -> BotResult<Job> {
    let job = Job::new_async(schedule, move |uuid, mut l| {
        let cache = cache.clone();
        let sender = sender.clone();
        Box::pin(async move {
            let (year, day) = current_aoc_year_day();
            let (p1, p2, delta) = {
                let leaderboard = cache.data.lock().unwrap();
                let standings = Standing::new(&leaderboard.leaderboard);
                let p1 = standings.by_time(&Ranking::PART1, year, day);
                let p2 = standings.by_time(&Ranking::PART2, year, day);
                let delta = standings.by_time(&Ranking::DELTA, year, day);
                (p1, p2, delta)
            };

            if let Err(e) = sender
                .send(Event::DailySummary(year, day, p1, p2, delta))
                .await
            {
                let error =
                    BotError::ChannelSend(format!("Could not send message to MPSC channel. {e}"));
                error!("{error}");
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

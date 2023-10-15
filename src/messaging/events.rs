use crate::aoc::leaderboard::{LeaderboardStatistics, ScrapedLeaderboard};
use crate::messaging::templates::MessageTemplate;
use crate::utils::{format_duration, suffix};
use minijinja::context;
use std::fmt;
use std::iter::Iterator;

use chrono::{DateTime, Local, Utc};

use slack_morphism::{SlackChannelId, SlackTs};

const COMMANDS: [&'static str; 2] = ["!help", "!ranking"];

#[derive(Debug)]
pub enum Event {
    GlobalLeaderboardComplete((u8, LeaderboardStatistics)),
    GlobalLeaderboardHeroFound((String, String)),
    PrivateLeaderboardUpdated,
    DailySolutionsThreadToInitialize(u32),
    CommandReceived(SlackChannelId, SlackTs, Command),
}

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    GetPrivateStandingByLocalScore(Vec<(String, String)>, DateTime<Utc>),
}

impl Command {
    pub fn is_command(input: &str) -> bool {
        let start_with = input.trim().split(" ").next().unwrap();
        COMMANDS.contains(&start_with)
    }

    pub fn build_from(input: String, leaderboard: &ScrapedLeaderboard) -> Command {
        let start_with = input.trim().split(" ").next().unwrap();
        match start_with {
            cmd if cmd == COMMANDS[0] => Command::Help,
            cmd if cmd == COMMANDS[1] => {
                let data = leaderboard
                    .leaderboard
                    .standings_by_local_score()
                    .into_iter()
                    .map(|(m, s)| (m, s.to_string()))
                    .collect::<Vec<(String, String)>>();
                Command::GetPrivateStandingByLocalScore(data, leaderboard.timestamp)
            }
            _ => unreachable!(),
        }
    }

    // pub fn get_prefix(&self) -> &str {
    //     match self {
    //         Command::Help => &COMMANDS[0],
    //         Command::GetPrivateStandingByLocalScore(..) => &COMMANDS[1],
    //     }
    // }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Event::DailySolutionsThreadToInitialize(day) => {
                write!(f, ":point_down: Daily solution thread for day {}", day)
            }
            Event::GlobalLeaderboardComplete((day, statistics)) => {
                write!(
                    f,
                    "{}",
                        MessageTemplate::GlobalStatistics.get()
                        .render(context! {
                            day => day,
                            p1_fast => statistics.p1_time_fast.map_or("N/A".to_string(), |d| format_duration(d)),
                            p1_slow => statistics.p1_time_slow.map_or("N/A".to_string(), |d| format_duration(d)),
                            p2_fast => statistics.p2_time_fast.map_or("N/A".to_string(), |d| format_duration(d)),
                            p2_slow => statistics.p2_time_slow.map_or("N/A".to_string(), |d| format_duration(d)),
                            delta_fast => statistics.delta_fast.map_or("N/A".to_string(), |(d, rank)| format!("*{}* ({}{})", format_duration(d), rank, suffix(rank))),
                            delta_slow => statistics.delta_slow.map_or("N/A".to_string(), |(d, rank)| format!("*{}* ({}{})", format_duration(d), rank, suffix(rank))),
                        })
                        .unwrap()
                )
            }
            Event::GlobalLeaderboardHeroFound((hero, part)) => {
                write!(
                    f,
                    "{}",
                    MessageTemplate::Hero
                        .get()
                        .render(context! { name => hero, part => part })
                        .unwrap()
                )
            }
            Event::PrivateLeaderboardUpdated => {
                write!(f, ":repeat: Private Leaderboard updated")
            }
            Event::CommandReceived(_channel_id, ts, cmd) => match cmd {
                Command::Help => {
                    write!(f, "{}", MessageTemplate::Help.get().render({}).unwrap())
                }
                Command::GetPrivateStandingByLocalScore(data, time) => {
                    let timestamp =
                        format!("{}", time.with_timezone(&Local).format("%d/%m/%Y %H:%M:%S"));

                    write!(
                        f,
                        "{}",
                        MessageTemplate::Ranking
                            .get()
                            .render(context! { timestamp => timestamp, scores => data })
                            .unwrap()
                    )
                }
            },
        }
    }
}

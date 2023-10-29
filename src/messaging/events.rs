use crate::aoc::leaderboard::{LeaderboardStatistics, ScrapedLeaderboard, Solution};
use crate::messaging::templates::MessageTemplate;
use crate::utils::{format_duration, ordinal_number_suffix, DayHighlight};
use itertools::Itertools;
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
    DailyChallengeIsUp(String),
    PrivateLeaderboardNewCompletions(Vec<DayHighlight>),
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
                // TODO: handle year
                let year = 2015;
                let data = leaderboard
                    .leaderboard
                    .standings_by_local_score()
                    .get(&year)
                    .unwrap_or(&vec![])
                    .into_iter()
                    .map(|(m, s)| (m.clone(), s.to_string()))
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
                write!(
                    f,
                    "{}",
                    MessageTemplate::DailySolutionThread
                        .get()
                        .render(context! { day => day })
                        .unwrap()
                )
            }
            Event::DailyChallengeIsUp(title) => {
                write!(
                    f,
                    "{}",
                    MessageTemplate::DailyChallenge
                        .get()
                        .render(context! { title => title })
                        .unwrap()
                )
            }
            Event::GlobalLeaderboardComplete((day, statistics)) => {
                write!(
                    f,
                    "{}",
                        MessageTemplate::GlobalStatistics.get()
                        .render(context! {
                            day => day,
                            p1_fast => statistics.p1_fast.map_or("N/A".to_string(), |d| format_duration(d)),
                            p1_slow => statistics.p1_slow.map_or("N/A".to_string(), |d| format_duration(d)),
                            p2_fast => statistics.p2_fast.map_or("N/A".to_string(), |d| format_duration(d)),
                            p2_slow => statistics.p2_slow.map_or("N/A".to_string(), |d| format_duration(d)),
                            delta_fast => statistics.delta_fast.map_or("N/A".to_string(), |(d, rank)| {
                                let rank = rank.unwrap_or_default();
                                format!("*{}* ({}{})", format_duration(d), rank, ordinal_number_suffix(rank))
                            }),
                            delta_slow => statistics.delta_slow.map_or("N/A".to_string(), |(d, rank)| {
                                let rank = rank.unwrap_or_default();
                                format!("*{}* ({}{})", format_duration(d), rank, ordinal_number_suffix(rank))
                            }),
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
                write!(
                    f,
                    "{}",
                    MessageTemplate::PrivateLeaderboardUpdated
                        .get()
                        .render({})
                        .unwrap()
                )
            }

            Event::PrivateLeaderboardNewCompletions(completions) => {
                // TODO: get day programmatically
                let (year, today): (i32, u8) = (2022, 9);

                let is_today_completions = completions
                    .iter()
                    .into_group_map_by(|h| h.year == year && h.day == today);

                let mut output = String::new();
                if let Some(today_completions) = is_today_completions.get(&true) {
                    output.push_str(
                        &MessageTemplate::NewTodayCompletions
                            .get()
                            .render(context! {completions => today_completions})
                            .unwrap(),
                    );
                };
                if let Some(late_completions) = is_today_completions.get(&false) {
                    if !output.is_empty() {
                        output.push_str("\n");
                    };
                    output.push_str(
                        &MessageTemplate::NewLateCompletions
                            .get()
                            .render(context! {completions => late_completions})
                            .unwrap(),
                    );
                };

                write!(f, "{}", output)
            }
            Event::CommandReceived(_channel_id, _ts, cmd) => match cmd {
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

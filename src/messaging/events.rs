use crate::{
    aoc::leaderboard::{LeaderboardStatistics, ProblemPart, ScrapedLeaderboard},
    messaging::templates::MessageTemplate,
    utils::{current_year_day, format_duration, format_rank, DayHighlight},
};
use chrono::{DateTime, Datelike, Local, Utc};
use itertools::Itertools;
use minijinja::context;
use slack_morphism::{SlackChannelId, SlackTs};
use std::{fmt, iter::Iterator};

const COMMANDS: [&'static str; 3] = ["!help", "!standings", "!leaderboard"];

#[derive(Debug)]
pub enum Event {
    GlobalLeaderboardComplete((u8, LeaderboardStatistics)),
    GlobalLeaderboardHeroFound((String, ProblemPart, u8)),
    DailyChallengeIsUp(String),
    PrivateLeaderboardNewEntries(Vec<DayHighlight>),
    PrivateLeaderboardUpdated,
    PrivateLeaderboardNewMembers(Vec<String>),
    DailySolutionsThreadToInitialize(u32),
    CommandReceived(SlackChannelId, SlackTs, Command),
}

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    GetPrivateStandingByLocalScore(i32, Vec<(String, String)>, DateTime<Utc>),
    GetLeaderboardHistogram(i32, String, DateTime<Utc>),
}

impl Command {
    pub fn is_command(input: &str) -> bool {
        let start_with = input.trim().split(" ").next().unwrap();
        COMMANDS.contains(&start_with)
    }

    pub fn build_from(input: String, leaderboard: &ScrapedLeaderboard) -> Command {
        let mut input = input.trim().split(" ");
        let start_with = input.next().unwrap();
        match start_with {
            cmd if cmd == COMMANDS[0] => Command::Help,
            cmd if cmd == COMMANDS[1] => {
                // !ranking
                let year = input
                    .next()
                    .and_then(|y| y.parse::<i32>().ok())
                    .unwrap_or_else(|| current_year_day().0);

                let data = leaderboard
                    .leaderboard
                    .standings_by_local_score_per_year()
                    .get(&year)
                    .unwrap_or(&vec![])
                    .into_iter()
                    .map(|(m, s)| (m.clone(), s.to_string()))
                    .collect::<Vec<(String, String)>>();
                Command::GetPrivateStandingByLocalScore(year, data, leaderboard.timestamp)
            }
            cmd if cmd == COMMANDS[2] => {
                // !leaderboard
                let year = input
                    .next()
                    .and_then(|y| y.parse::<i32>().ok())
                    .unwrap_or_else(|| current_year_day().0);

                let formatted = leaderboard.leaderboard.show_year(year);
                Command::GetLeaderboardHistogram(year, formatted, leaderboard.timestamp)
            }
            _ => unreachable!(),
        }
    }
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
                                format!("*{}* ({})", format_duration(d), format_rank(rank))
                            }),
                            delta_slow => statistics.delta_slow.map_or("N/A".to_string(), |(d, rank)| {
                                let rank = rank.unwrap_or_default();
                                format!("*{}* ({})", format_duration(d), format_rank(rank))
                            }),
                        })
                        .unwrap()
                )
            }
            Event::GlobalLeaderboardHeroFound((hero, part, rank)) => {
                write!(
                    f,
                    "{}",
                    MessageTemplate::Hero
                        .get()
                        .render(context! { name => hero, part => part.to_string(), rank => format_rank(*rank) })
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
            Event::PrivateLeaderboardNewEntries(entries) => {
                let (year, today) = current_year_day();

                let is_today_entries = entries
                    .iter()
                    .into_group_map_by(|h| h.year == year && h.day == today);

                let mut output = String::new();
                if let Some(today_entries) = is_today_entries.get(&true) {
                    output.push_str(
                        &MessageTemplate::NewEntriesToday
                            .get()
                            .render(context! {completions => today_entries})
                            .unwrap(),
                    );
                };
                if let Some(late_entries) = is_today_entries.get(&false) {
                    if !output.is_empty() {
                        output.push_str("\n");
                    };
                    output.push_str(
                        &MessageTemplate::NewEntriesLate
                            .get()
                            .render(context! {completions => late_entries})
                            .unwrap(),
                    );
                };

                write!(f, "{}", output)
            }
            Event::PrivateLeaderboardNewMembers(members) => {
                write!(
                    f,
                    "{}",
                    MessageTemplate::LeaderboardMemberJoin
                        .get()
                        .render(context! {members => members})
                        .unwrap()
                )
            }
            Event::CommandReceived(_channel_id, _ts, cmd) => match cmd {
                Command::Help => {
                    write!(f, "{}", MessageTemplate::Help.get().render({}).unwrap())
                }
                Command::GetPrivateStandingByLocalScore(year, data, time) => {
                    let now = time.with_timezone(&Local);
                    let timestamp = format!("{}", now.format("%d/%m/%Y %H:%M:%S"));

                    write!(
                        f,
                        "{}",
                        MessageTemplate::Ranking
                            .get()
                            .render(context! { year => year, current_year => year == &now.year(), timestamp => timestamp, scores => data })
                            .unwrap()
                    )
                }
                Command::GetLeaderboardHistogram(year, histogram, time) => {
                    let now = time.with_timezone(&Local);
                    let timestamp = format!("{}", now.format("%d/%m/%Y %H:%M:%S"));

                    write!(
                        f,
                        "{}",
                        MessageTemplate::Leaderboard
                            .get()
                            .render(context! { year => year, current_year => year == &now.year(), timestamp => timestamp, leaderboard => histogram })
                            .unwrap()
                    )
                }
            },
        }
    }
}

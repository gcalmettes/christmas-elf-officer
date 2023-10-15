use crate::aoc::leaderboard::{LeaderboardStatistics, ScrapedLeaderboard};
use crate::utils::{format_duration, suffix};
use minijinja::{context, Environment};
use once_cell::sync::Lazy;
use std::fmt;
use std::iter::Iterator;

use chrono::{DateTime, Local, Utc};
use tracing::info;

use slack_morphism::{SlackChannelId, SlackTs};

static TEMPLATES_ENVIRONMENT: Lazy<Environment> = Lazy::new(|| {
    info!("Initializing templating engine environment.");
    let mut env = Environment::new();
    env.add_template(
        "hero.txt",
        ":tada: Our very own *{{ name }}* made it to the global leaderboard on part {{ part }}!",
    )
    .unwrap();
    env.add_template(
        "ranking.txt",
        ":first_place_medal: Current ranking as of {{timestamp}}:\n\
        {%- for (name, score) in scores %}
            \x20 • {{name}} => {{score}}
        {%- endfor %}",
    )
    .unwrap();
    env.add_template(
        "global_leaderboard_statistics.txt",
        ":tada: Global Leaderboard complete for *day {{day}}*, here is how it went:\n\
            \x20 • Part 1 finish time range: *{{p1_fast}}* - *{{p1_slow}}*\n\
            \x20 • Part 2 finish time range: *{{p2_fast}}* - *{{p2_slow}}*\n\
            \x20 • Delta times range: {{delta_fast}} - {{delta_slow}}",
    )
    .unwrap();

    info!("Templates loaded in templating engine environment.");
    env
});

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
            // TODO: do not send full global leaderboard but just what we need ?
            Event::GlobalLeaderboardComplete((day, statistics)) => {
                let template = TEMPLATES_ENVIRONMENT
                    .get_template("global_leaderboard_statistics.txt")
                    .unwrap();
                write!(
                    f,
                    "{}",
                    template
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
                let template = TEMPLATES_ENVIRONMENT.get_template("hero.txt").unwrap();
                write!(
                    f,
                    "{}",
                    template
                        .render(context! { name => hero, part => part })
                        .unwrap()
                )
            }
            Event::PrivateLeaderboardUpdated => {
                write!(f, ":repeat: Private Leaderboard updated")
            }
            Event::CommandReceived(_channel_id, ts, cmd) => match cmd {
                // \n\ at each code line end creates a line break at the proper position and discards further spaces in this line of code
                // \x20 (hex; 32 in decimal) is an ASCII space and an indicator for the first space to be preserved in this line of the string
                Command::Help => {
                    write!(
                        f,
                        ":sos: below are the bot commands:\n\
                            \x20   `!help`: the commands\n\
                            \x20   `!ranking`: current ranking by local score\n\
                        "
                    )
                }

                Command::GetPrivateStandingByLocalScore(data, time) => {
                    let template = TEMPLATES_ENVIRONMENT.get_template("ranking.txt").unwrap();

                    let timestamp =
                        format!("{}", time.with_timezone(&Local).format("%d/%m/%Y %H:%M:%S"));

                    write!(
                        f,
                        "{}",
                        template
                            .render(context! { timestamp => timestamp, scores => data })
                            .unwrap()
                    )
                }
            },
        }
    }
}

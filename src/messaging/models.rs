use crate::aoc::leaderboard::{GlobalLeaderboard, ScrapedPrivateLeaderboard};
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
        ":tada: Our very own {{ name }} made it to the global leaderboard on part {{ part }}!",
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

    info!("Templates loaded in templating engine environment.");
    env
});

const COMMANDS: [&'static str; 2] = ["!help", "!ranking"];

#[derive(Debug, Clone)]
pub enum Event {
    GlobalLeaderboardComplete(GlobalLeaderboard),
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

    pub fn build_from(input: String, leaderboard: &ScrapedPrivateLeaderboard) -> Command {
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

fn suffix(num: u8) -> &'static str {
    let s = num.to_string();
    if s.ends_with('1') && !s.ends_with("11") {
        "st"
    } else if s.ends_with('2') && !s.ends_with("12") {
        "nd"
    } else if s.ends_with('3') && !s.ends_with("13") {
        "rd"
    } else {
        "th"
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Event::DailySolutionsThreadToInitialize(day) => {
                write!(f, ":point_down: Daily solution thread for day {}", day)
            }
            // TODO: do not send full global leaderboard but just what we need ?
            Event::GlobalLeaderboardComplete(global_leaderboard) => {
                let (fastest_part_one, fastest_part_two, slowest_part_one, slowest_part_two) = {
                    if let (
                        Some((_p1, Some(first_part_fast), Some(second_part_fast))),
                        Some((_p100, Some(first_part_slow), Some(second_part_slow))),
                    ) = global_leaderboard.get_fastest_and_slowest_times()
                    {
                        let first_part_fast_time = {
                            let fast = first_part_fast.time;
                            let seconds = fast.num_seconds() % 60;
                            let minutes = (fast.num_seconds() / 60) % 60;
                            let hours = (fast.num_seconds() / 60) / 60;
                            format!(
                                "[{}] {:02}:{:02}:{:02}",
                                first_part_fast.rank, hours, minutes, seconds,
                            )
                        };
                        let second_part_fast_time = {
                            let fast = second_part_fast.time;
                            let seconds = fast.num_seconds() % 60;
                            let minutes = (fast.num_seconds() / 60) % 60;
                            let hours = (fast.num_seconds() / 60) / 60;
                            format!(
                                "[{}] {:02}:{:02}:{:02}",
                                second_part_fast.rank, hours, minutes, seconds,
                            )
                        };
                        let first_part_slow_time = {
                            let slow = first_part_slow.time;
                            let seconds = slow.num_seconds() % 60;
                            let minutes = (slow.num_seconds() / 60) % 60;
                            let hours = (slow.num_seconds() / 60) / 60;
                            format!(
                                "[{}] {:02}:{:02}:{:02}",
                                first_part_slow.rank, hours, minutes, seconds,
                            )
                        };
                        let second_part_slow_time = {
                            let slow = second_part_slow.time;
                            let seconds = slow.num_seconds() % 60;
                            let minutes = (slow.num_seconds() / 60) % 60;
                            let hours = (slow.num_seconds() / 60) / 60;
                            format!(
                                "[{}] {:02}:{:02}:{:02}",
                                second_part_slow.rank, hours, minutes, seconds,
                            )
                        };
                        (
                            first_part_fast_time,
                            second_part_fast_time,
                            first_part_slow_time,
                            second_part_slow_time,
                        )
                    } else {
                        (
                            "[1] N/A".to_string(),
                            "[1] N/A".to_string(),
                            "[100] N/A".to_string(),
                            "[100] N/A".to_string(),
                        )
                    }
                };

                let (fastest_delta, slowest_delta) = {
                    if let (Some((delta_fast, rank_fast)), Some((delta_slow, rank_slow))) =
                        global_leaderboard.get_fastest_and_slowest_deltas()
                    {
                        let seconds_fast = delta_fast.num_seconds() % 60;
                        let minutes_fast = (delta_fast.num_seconds() / 60) % 60;
                        let hours_fast = (delta_fast.num_seconds() / 60) / 60;
                        let fmt_fast = format!(
                            "{:02}:{:02}:{:02} ({}{})",
                            hours_fast,
                            minutes_fast,
                            seconds_fast,
                            rank_fast,
                            suffix(rank_fast)
                        );

                        let seconds_slow = delta_slow.num_seconds() % 60;
                        let minutes_slow = (delta_slow.num_seconds() / 60) % 60;
                        let hours_slow = (delta_slow.num_seconds() / 60) / 60;
                        let fmt_slow = format!(
                            "{:02}:{:02}:{:02} ({}{})",
                            hours_slow,
                            minutes_slow,
                            seconds_slow,
                            rank_slow,
                            suffix(rank_slow)
                        );

                        (fmt_fast, fmt_slow)
                    } else {
                        ("".to_string(), "".to_string())
                    }
                };

                write!(
                    f,
                    ":tada: Global Leaderboard complete\n\
                    Part 1: {fastest_part_one} - {slowest_part_one}\n\
                    Part 2: {fastest_part_two} - {slowest_part_two}\n\
                    Delta times range in top 100: {fastest_delta} - {slowest_delta}"
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

                    // let timestamp = format!(
                    //     "{:02}:{:02}:{:02} (UTC)",
                    //     time.hour(),
                    //     time.minute(),
                    //     time.second()
                    // );
                    // let ranking =
                    //     format!(":first_place_medal: Current ranking as of {timestamp}:\n");
                    // let scores = data
                    //     .iter()
                    //     .map(|(name, score)| format!(" \x20 • {name} => {score}"))
                    //     .join("\n");

                    // write!(f, "{ranking}{scores}")
                }
            },
        }
    }
}

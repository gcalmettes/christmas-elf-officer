use std::fmt;

use slack_morphism::{SlackChannelId, SlackTs};

#[derive(Debug)]
pub enum Command {
    Help,
    GetRanking,
}

#[derive(Debug)]
pub enum Event {
    GlobalLeaderboardComplete,
    GlobalLeaderboardHeroFound(String),
    PrivateLeaderboardUpdated,
    CommandReceived(SlackChannelId, SlackTs, Command),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Event::GlobalLeaderboardComplete => {
                write!(f, ":tada: Global Leaderboard complete")
            }
            Event::GlobalLeaderboardHeroFound(hero) => {
                write!(
                    f,
                    ":tada: Our own {} made it to the global leaderboard !",
                    hero
                )
            }
            Event::PrivateLeaderboardUpdated => {
                write!(f, ":repeat: Private Leaderboard updated")
            }
            Event::CommandReceived(channel_id, ts, _cmd) => {
                write!(f, ":tada: Command received {} {}", channel_id, ts)
            }
        }
    }
}

// #[derive(Debug)]
// pub struct MyEvent {
//     pub event: Event,
// }


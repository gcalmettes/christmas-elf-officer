use minijinja::{Environment, Template};
use once_cell::sync::Lazy;

use strum::{EnumIter, IntoEnumIterator};
use tracing::info;

static TEMPLATES_ENVIRONMENT: Lazy<Environment> = Lazy::new(|| {
    info!("Initializing templating engine environment.");
    let mut env = Environment::new();

    // Use strum to iterate over the variants of the enum.
    for template in MessageTemplate::iter() {
        env.add_template(template.name(), template.template())
            .unwrap();
    }

    info!("Templates loaded in templating engine environment.");
    env
});

#[derive(EnumIter)]
pub enum MessageTemplate {
    Help,
    DailyChallenge,
    DailySolutionThread,
    GlobalStatistics,
    PrivateLeaderboardUpdated,
    Ranking,
    Hero,
}

impl MessageTemplate {
    pub fn name(&self) -> &'static str {
        match self {
            MessageTemplate::Help => "help.txt",
            MessageTemplate::DailyChallenge => "challenge.txt",
            MessageTemplate::DailySolutionThread => "solution_thread.txt",
            MessageTemplate::PrivateLeaderboardUpdated => "private_leaderboard_updated.txt",
            MessageTemplate::GlobalStatistics => "global_leaderboard_statistics.txt",
            MessageTemplate::Ranking => "ranking.txt",
            MessageTemplate::Hero => "hero.txt",
        }
    }

    pub fn get(&self) -> Template<'_, '_> {
        TEMPLATES_ENVIRONMENT.get_template(self.name()).unwrap()
    }

    pub fn template(&self) -> &'static str {
        // \n\ at each code line end creates a line break at the proper position and discards further spaces in this line of code.
        // \x20 (hex; 32 in decimal) is an ASCII space and an indicator for the first space to be preserved in this line of the string.
        match self {
            MessageTemplate::Help => {
                ":sos: below are the bot commands:\n\
                    \x20   `!help`: the commands\n\
                    \x20   `!ranking`: current ranking by local score\n\
                "
            },
            MessageTemplate::DailyChallenge => {
                ":tada: Today's challenge is up!\n\
                    \x20 *{{title}}*
                "
            },
            MessageTemplate::DailySolutionThread => {
                ":point_down: Daily solution thread for *day {{day}}*"
            },
            MessageTemplate::PrivateLeaderboardUpdated => {
                ":repeat: Private Leaderboard successfully updated!"
            },
            MessageTemplate::GlobalStatistics => {
                ":tada: Global Leaderboard complete for *day {{day}}*, here is how it went for the big dogs:\n\
                    \x20 • Part 1 finish time range: *{{p1_fast}}* - *{{p1_slow}}*\n\
                    \x20 • Part 2 finish time range: *{{p2_fast}}* - *{{p2_slow}}*\n\
                    \x20 • Delta times range: {{delta_fast}} - {{delta_slow}}"
            }
            MessageTemplate::Ranking => {
                ":first_place_medal: Current ranking as of {{timestamp}}:\n\
                {%- for (name, score) in scores %}
                    \x20 • {{name}} => {{score}}
                {%- endfor %}"
            }
            MessageTemplate::Hero => {
                ":tada: Our very own *{{ name }}* made it to the global leaderboard on part {{ part }}!"
            },
        }
    }
}

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
    NewTodayCompletions,
    NewLateCompletions,
    Ranking,
    Leaderboard,
    Hero,
}

impl MessageTemplate {
    pub fn name(&self) -> &'static str {
        match self {
            MessageTemplate::Help => "help.txt",
            MessageTemplate::DailyChallenge => "challenge.txt",
            MessageTemplate::DailySolutionThread => "solution_thread.txt",
            MessageTemplate::PrivateLeaderboardUpdated => "private_leaderboard_updated.txt",
            MessageTemplate::NewTodayCompletions => "today_completions.txt",
            MessageTemplate::NewLateCompletions => "late_completions.txt",
            MessageTemplate::GlobalStatistics => "global_leaderboard_statistics.txt",
            MessageTemplate::Ranking => "ranking.txt",
            MessageTemplate::Leaderboard => "leaderboard.txt",
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
                "🆘 below are the bot commands:\n\
                 \x20 `!help`: the commands\n\
                 \x20 `!standings [year]`: standings by local score for the current year [or specified year]\n\
                 \x20 `!leaderboard [year]`: leaderboard state for the current year [or specified year]\n\
                "
            },
            MessageTemplate::DailyChallenge => {
                "🎉 Today's challenge is up!\n\
                    \x20 *{{title}}*
                "
            },
            MessageTemplate::DailySolutionThread => {
                "👇 *Daily discussion thread for day {{day}}*\n\
                    \x20   Refrain yourself to open until you complete part 2!\n\
                 🚨 *Spoilers Ahead* :rotating_light:"
            },
            MessageTemplate::PrivateLeaderboardUpdated => {
                "🔁 Private Leaderboard successfully updated!"
            },
            MessageTemplate::NewTodayCompletions => {
                "{%- for entry in completions %}\n\
                    {% with both = entry.parts_duration|length > 1, double = ':white_check_mark:', single = ':heavy_check_mark:' %}\
                    📣 {{entry.name}} just earned *{{entry.n_stars}}* more star{{ 's' if entry.n_stars > 1 }} {{ ['(day', entry.day, double, 'completed!)']|join(' ')  if both else ['for day', entry.day, single]|join(' ') }} +{{entry.new_points}}pts
                    {%- endwith %}
                 {%- endfor %}\n"
            },
            MessageTemplate::NewLateCompletions => {
                "{%- for entry in completions %}\n\
                    {% with both = entry.parts_duration|length > 1, double = ':white_check_mark:', single = ':heavy_check_mark:' %}\
                    🚂  {{entry.name}} just catched up on *{{entry.n_stars}}* more star{{ 's' if entry.n_stars > 1 }} ({{ ['day', entry.day, double, 'completed!']|join(' ')  if both else single }}) +{{entry.new_points}}pts
                    {%- endwith %}
                 {%- endfor %}"
            },
            MessageTemplate::GlobalStatistics => {
                "📣 🌍 Global Leaderboard is complete for *day {{day}}*! Here is how it went for the big dogs:\n\
                    \x20 • Part 1 finish time range: 🔥 *{{p1_fast}}* - *{{p1_slow}}* ❄️\n\
                    \x20 • Part 2 finish time range: 🔥 *{{p2_fast}}* - *{{p2_slow}}* ❄️\n\
                    \x20 • Delta times range: 🏃‍♀️ {{delta_fast}} - {{delta_slow}} 🚶‍♀️"
            }
            MessageTemplate::Ranking => {
                "{% if current_year %}
                    :first_place_medal: Current ranking as of {{timestamp}}:\n\
                {% else %}
                    :first_place_medal: Ranking from the {{ year }} event:\n\
                {% endif %}
                {%- for (name, score) in scores %}\n\
                 • {{name}} \t {{score}}
                {%- endfor %}"
            }
            MessageTemplate::Hero => {
                "🎉 🥳 Our very own *{{ name }}* made it to the global leaderboard on part *{{ part }}*! (*{{ rank }}*) 🙌"
            },
            MessageTemplate::Leaderboard => {
                "{%- if current_year -%}
                    📓 Current Leaderboard as of {{timestamp}}:
                {%- else -%}
                    📓 Learderboard from the {{ year }} event:
                {%- endif -%}
                ```{{ leaderboard }}```"
            }
        }
    }
}

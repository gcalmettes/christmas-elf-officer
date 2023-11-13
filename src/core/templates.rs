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
    LeaderboardMemberJoin,
    NewEntriesToday,
    NewEntriesLate,
    TdfStandings,
    Ranking,
    LeaderboardDisplay,
    Hero,
}

impl MessageTemplate {
    pub fn name(&self) -> &'static str {
        match self {
            MessageTemplate::Help => "help.txt",
            MessageTemplate::DailyChallenge => "challenge.txt",
            MessageTemplate::DailySolutionThread => "solution_thread.txt",
            MessageTemplate::PrivateLeaderboardUpdated => "private_leaderboard_updated.txt",
            MessageTemplate::LeaderboardMemberJoin => "private_leaderboard_new_members.txt",
            MessageTemplate::NewEntriesToday => "today_entries.txt",
            MessageTemplate::NewEntriesLate => "late_entries.txt",
            MessageTemplate::GlobalStatistics => "global_leaderboard_statistics.txt",
            MessageTemplate::Ranking => "ranking.txt",
            MessageTemplate::TdfStandings => "tdf.txt",
            MessageTemplate::LeaderboardDisplay => "leaderboard.txt",
            MessageTemplate::Hero => "hero.txt",
        }
    }

    pub fn get(&self) -> Template<'_, '_> {
        TEMPLATES_ENVIRONMENT.get_template(self.name()).unwrap()
    }

    pub fn template(&self) -> &'static str {
        // \n\ at each code line end creates a line break at the proper position and discards further spaces in this line of code.
        // \x20 (hex; 32 in decimal) is an ASCII space and an indicator for the first space to be preserved in this line of the string.

        // !fast [method] [day] [year]

        match self {
            MessageTemplate::Help => {
                "🗒️ Please find below the CEO commands handbook.\n\
                Note that the order of the optional arguments passed to a command does (or should) not matter.\n\n\
                👉 🆘 *How to*\n\
                ```!help```\n\
                Explains the bot commands. You're currently reading this.\n\n\
                👉 🏎️ *Fastest of the West!*\n\
                ```!fast [ranking method] [day] [year]```\n\
                Fastest time(s) for the day. By default, the ranking is based on the `delta` time for the day,\
                but individual `p1` and `p2` rankings are also available.\
                If no day and/or year is set, the current day is automatically defined.`\n\n\
                👉 📊 *Show me the board!*\n\
                ```!board [ranking method] [year]```\n\
                Current score and stars completion for the year, shown as a neat ascii board. Default is ranking by `local`\
                score for the current year, but ranking by number of `stars` is also available.`\n\n\
                👉 🚴 *The long haul!*\n\
                ```!tdf [jersey color] [year]```\n\
                Tour de France kind-of standing! Come and compete in to earn `yellow` or `green` points.\
                Default is ranking for the Yellow jersey for the current year.
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
            MessageTemplate::LeaderboardMemberJoin => {
                "{%- for name in members %}\n\
                    🕺 A new player has joined the christmas arena ! Happy to have you on board *{{name}}* !
                 {%- endfor %}"
            },
            MessageTemplate::NewEntriesToday => {
                "{%- for entry in completions %}\n\
                    {% with both = entry.parts_duration|length > 1, double = ':white_check_mark:', single = ':heavy_check_mark:' %}\
                    📣 {{entry.name}} just earned *{{entry.n_stars}}* more star{{ 's' if entry.n_stars > 1 }} {{ ['(day', entry.day, double, 'completed!)']|join(' ')  if both else ['for day', entry.day, single]|join(' ') }} +{{entry.new_points}}pts
                    {%- endwith %}
                 {%- endfor %}\n"
            },
            MessageTemplate::NewEntriesLate => {
                "{%- for entry in completions %}\n\
                    {% with both = entry.parts_duration|length > 1, double = ':white_check_mark:', single = ':heavy_check_mark:' %}\
                    🚂  {{entry.name}} just catched up on *{{entry.n_stars}}* more star{{ 's' if entry.n_stars > 1 }} ({{ ['day', entry.day, double, 'completed!']|join(' ')  if both else single }}) +{{entry.new_points}}pts
                    {%- endwith %}
                 {%- endfor %}"
            },
            MessageTemplate::GlobalStatistics => {
                "🌍 Global Leaderboard is complete for *day {{day}}*! Here is how it went for the big dogs:\n\
                    \x20 • Part 1 finish time range: 🔥 *{{p1_fast}}* - *{{p1_slow}}* ❄️\n\
                    \x20 • Part 2 finish time range: 🔥 *{{p2_fast}}* - *{{p2_slow}}* ❄️\n\
                    \x20 • Delta times range: 🏃‍♀️ {{delta_fast}} - {{delta_slow}} 🚶‍♀️"
            }
            MessageTemplate::Ranking => {
                "{% if current_day %}
                    Today's fastest *{{ ranking_method }} time* (as of {{timestamp}}):\n\
                {% else %}
                    Fastest *{{ ranking_method }} time* for day {{ day }}/12/{{ year }}:\n\
                {% endif %}
                {%- for (name, time) in ranking %}\n\
                 • {{name}} \t {{time}}
                {%- endfor %}"
            }
            MessageTemplate::Hero => {
                "🎉 🥳 Our very own *{{ name }}* made it to the global leaderboard on part *{{ part }}*! (*{{ rank }}*) 🙌"
            },
            MessageTemplate::LeaderboardDisplay => {
                "{%- if current_year -%}
                    📓 Current Leaderboard by {{ '*local score*' if scoring_method == 'local' else '*number of stars*' }} as of {{timestamp}}:
                {%- else -%}
                    📓 Learderboard by {{ '*local score*' if scoring_method == 'local' else '*number of stars*' }} from the {{ year }} event:
                {%- endif -%}
                ```{{ leaderboard }}```"
            }
            MessageTemplate::TdfStandings => {
                "{%- if current_year -%}
                    🚴 {{ '🟡🛵' if jersey=='yellow' else ('🟢' if jersey=='green' else '⚫')}} Jersey standings as of {{timestamp}}:\n\
                {%- else -%}
                    🚴 {{ '🟡🛵' if jersey=='yellow' else ('🟢' if jersey=='green' else '⚫')}} Jersey standings from the {{ year }} event:\n\
                {%- endif -%}
                ```{{ standings }}```"
            }
        }
    }
}

// year => year,
// day => day,
// current_year => year == &now.year(),
// timestamp => timestamp,
// ranking => data,
// ranking_method => method.to_string()

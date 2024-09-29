use crate::{
    core::leaderboard::Entry,
    utils::{current_aoc_year_day, format_rank},
};
use chrono::{Duration, Utc};
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
    CustomMessage,
    DailyChallenge,
    DailySolutionThread,
    DailySummary,
    GlobalStatistics,
    HardChallenge,
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
            MessageTemplate::CustomMessage => "custom.txt",
            MessageTemplate::DailyChallenge => "challenge.txt",
            MessageTemplate::DailySolutionThread => "solution_thread.txt",
            MessageTemplate::DailySummary => "summary.txt",
            MessageTemplate::PrivateLeaderboardUpdated => "private_leaderboard_updated.txt",
            MessageTemplate::LeaderboardMemberJoin => "private_leaderboard_new_members.txt",
            MessageTemplate::NewEntriesToday => "today_entries.txt",
            MessageTemplate::NewEntriesLate => "late_entries.txt",
            MessageTemplate::GlobalStatistics => "global_leaderboard_statistics.txt",
            MessageTemplate::HardChallenge => "hard_challenge.txt",
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

        match self {
            MessageTemplate::Help => {
                "🗒️ Nice work, you've found the *CEO commands handbook*.\n\
                Note that the command arguments parsing system is a marvel of regex engineering, and as such \
                the order of the optional arguments passed to a command does not (or at least should not...) matter.\n\n\
                👉 🆘 *How to*\n\
                ```!help```\n\
                List and explains the bot commands. You're currently reading this.\n\n\
                👉 🏎️ *Fastest of the West!*\n\
                ```!fast [ranking method] [day] [year]```\n\
                Fastest time(s) for the day. By default, the ranking is based on the `delta` time for the day, \
                but individual `p1` and `p2` rankings are also available. Note that you can also access the \
                ranking of the closest finishes before cuttoff (i.e.: the least amount of time before the next puzzle release) \
                with the `limit` method (those times are used to attribute points for the `!tdf combative` jersey). \
                If no day and/or year is set, the current day/or year is automatically defined.`\n\n\
                👉 📊 *Show me the board!*\n\
                ```!board [ranking method] [year]```\n\
                Current score and stars completion for the year, shown as a neat ascii board. Default is ranking by `local` \
                score for the current year, but ranking by number of `stars` is also available.`\n\n\
                👉 🚴 *The long haul!*\n\
                ```!tdf [jersey color] [day] [year]```\n\
                Tour de France alternative standings! Come join the peloton and compete to earn `yellow` jersey credentials, \
                or accumulate points for the coveted `green` or `combative` jerseys. \
                Default is ranking for the Yellow jersey for the current year.\n\
                - `yellow` jersey ranking is based on the accumulated time for the full (part 2) solve each day (a penalty of \
                7 days is applied for every day not fully solved, or any day taking longer to solve than the penalty time).\n\
                - `green` jersey points are earned each day by going full blast between part 1 and part 2 ! The points attributed are \
                based on the official Tour de France green jersey points.\n\
                - `combative` jersey points are attributed each day to the brave soul showing grit by not throwing the towel too early and keeping \
                their focus on finishing a day before the next one starts ... The closer to the cutoff, the more points earned !"
            },
            MessageTemplate::CustomMessage => {
                "🙅 {{message}}"
            },
            MessageTemplate::HardChallenge => {
                "😱 *{{minutes}} minutes* went by already and there are still some spots to grab in the global leaderboard ...\n\
                {% if cycle == 5 -%}
                    Not sure about you, but it feels like the temperature 🤒 is suddenly rising...
                {% elif cycle == 8 -%}
                    I guess now is a good time to have some handkerchief ready nearby in case you need to cry 😭.
                {% elif cycle == 11 -%}
                    Don't worry, feeling the urge to phone ☎️  a friend in order to cry for help 🆘 is a normal desire today.
                {% else -%}
                    Oh boy, time to raise the flag for hope 🏴 ... I can only wish you good luck 🤞, you will definitely need it today ...
                {% endif %}"
            },
            MessageTemplate::DailyChallenge => {
                "```{{header}}```\n\
                🎉 Today's challenge is up! (<{{url}}|link>)\n\
                    \x20 *{{title}}*\n\
                🔫 Go after it and get some fun, ⏱️ time is ticking !"
            },
            MessageTemplate::DailySolutionThread => {
                "👇 *Daily discussion thread for day {{day}}*\n\
                    \x20   Refrain yourself to open until you complete part 2!\n\
                 🚨 *Spoilers Ahead* :rotating_light:"
            },
            MessageTemplate::DailySummary => {
                "🗓️ *December, {{day}} {{year}}*\n\
                ----- 🥁 *Daily update* 🗞️ -----\n\
                Here is how things went down at the front of the pack today:\n\
                ___________________________________________________________________\n\
                Top 5 to finish *PART 1* 🏁\n\
                {%- for (prefix, name, time) in ranking_p1 %}\n\
                    {{prefix}} in ⏱️ {{time}} 👉🏻 *{{name}}*
                {%- endfor %}\n\
                ___________________________________________________________________\n\
                Top 5 to finish *PART 2* 🏁\n\
                {%- for (prefix, name, time) in ranking_p2 %}\n\
                    {{prefix}} in ⏱️ {{time}} 👉🏻 *{{name}}*
                {%- endfor %}\n\
                ___________________________________________________________________\n\
                Top 5 *DELTA* 🏁\n\
                {%- for (prefix, name, time) in ranking_delta %}\n\
                    {{prefix}} in ⏱️ {{time}} 👉🏻 *{{name}}*
                {%- endfor %}"
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
                    {% with both = entry.parts_duration|length > 1, double = '⭐⭐', single = '⭐' %}\
                    📣 {{entry.name}} just earned *{{entry.n_stars}}* more star{{ 's' if entry.n_stars > 1 }} for day {{entry.day}} ({{[double, '*<->', entry.delta, '*']|join(' ') if both else single}}) +{{entry.new_points}}pts
                    {%- endwith %}
                 {%- endfor %}\n"
            },
            MessageTemplate::NewEntriesLate => {
                "{%- for entry in completions %}\n\
                    {% with both = entry.parts_duration|length > 1, double = '🤩', single = '✔️' %}\
                    🚂  {{entry.name}} just caught up on *{{entry.n_stars}}* more star{{ 's' if entry.n_stars > 1 }} for day {{entry.day}} ({{ [double, 'both parts completed!', '*<->', entry.delta, '*']|join(' ')  if both else single }}) +{{entry.new_points}}pts
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
                "{%- if current_day -%}
                    Today's {{'fastest' if not is_limit else 'closest'}} *{{ ranking_method }} time* (as of {{timestamp}}):
                {%- else -%}
                    {{'Fastest' if not is_limit else 'Closest'}} *{{ ranking_method }} time* for day {{ day }}/12/{{ year }}:
                {%- endif %}\n\
                {%- for (prefix, name, time) in ranking %}\n\
                    {{prefix}} in ⏱️ {{time}} 👉🏻 *{{name}}*
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
                {%- endif %}\n\
                ```{{ leaderboard }}```"
            }
            MessageTemplate::TdfStandings => {
                "{%- if current_year and not day -%}
                    🚴 {{ '🟡 Yellow 🛵' if jersey=='yellow' else ('🟢 Green 🍏' if jersey=='green' else '⚫Combative 🥋')}} Jersey current standings as of {{timestamp}}:
                {%- elif not day -%}
                    🚴 {{ '🟡 Yellow 🛵' if jersey=='yellow' else ('🟢 Green 🍏' if jersey=='green' else '⚫Combative 🥋')}} Jersey standings from the *{{year}}* event:
                {%- else -%}
                    🚴 {{ '🟡 Yellow 🛵' if jersey=='yellow' else ('🟢 Green 🍏' if jersey=='green' else '⚫Combative 🥋')}} Jersey standings for *day {{day}}* of the {{year}} event:
                {%- endif %}\n\
                ```{{ standings }}```"
            }
        }
    }
}

pub fn invalid_year_day_message(year: i32, day: Option<u8>) -> Option<String> {
    // no AOC before 2015
    if year < 2015 {
        return Some(format!(
            "I see that you are like me, loving the thrill of exploring old archives 🗃️!\n\
            However, sorry to break it to you, but there is *no gem to be found in {year}* \
            as the the AOC event only started in 2015..."
        ));
    };

    let (current_year, current_day) = current_aoc_year_day();

    // in the future
    if year > current_year {
        let delta = year - current_year;
        let potential_s = match delta > 1 {
            true => "s",
            false => "",
        };
        return Some(format!(
            "I like the enthusiam, but unfortunately I am no Nostradamus 🧙 and can't see in the future 🔮 ...\n\
            *Come back in {delta} year{}* to discover the standings for *{year}*!",
            potential_s
        ));
    };

    // specific case of zero
    if day == Some(0) {
        return Some(
            "Mmmhhh, looks like you wrote too much Python 🐍 and are now convinced that everything is zero-indexed, \
            but in real-life the first day of the month is one 1️⃣."
                .to_string(),
        );
    };

    // after Christmas
    if day > Some(25) {
        return Some(
            "You're definitely free to code after Christmas 🎄, but *AOC puzzles stop after the 25th*."
                .to_string(),
        );
    };

    match (
        year == current_year,
        day == Some(current_day),
        day > Some(current_day),
    ) {
        // future day
        (true, _, true) => {
            // Safe since day is some
            let day = day.unwrap();
            let delta = day - current_day;
            let potential_s = match delta > 1 {
                true => "s",
                false => "",
            };
            Some(format!(
                "I know the suspense is unbearable, but I can't go faster than the music 🎶...\n\
                *Come back in {delta} day{}* to see what's happening on December {}.",
                potential_s,
                format_rank(day)
            ))
        }
        // it's today, make sure AOC puzzle was released
        (true, true, _) => {
            let now = Utc::now();
            // safe unwrap since day is some
            let day = day.unwrap();
            Entry::puzzle_unlock(year, day)
                .ok()
                .and_then(|release_time| match now - release_time > Duration::seconds(0) {
                    false => Some(
                        "The wait is almost over ⌛, today's first puzzle will be released at 05:00 UTC!".to_string()
                    ),
                    // puzzle already released
                    true => None,
                })
        }
        (_, _, _) => None, // any other combination is valid
    }
}

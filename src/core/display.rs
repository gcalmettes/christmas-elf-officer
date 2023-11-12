use crate::{
    core::{leaderboard::Identifier, standings::PENALTY_UNFINISHED_DAY},
    utils::{format_duration, format_duration_with_days},
};
use chrono::Duration;
use itertools::Itertools;

// Display tdf ranking
pub fn tdf(entries: Vec<(&Identifier, i64, i64)>) -> String {
    // calculate width for positions
    // the width of the maximum position to be displayed, plus one for ')'
    let width_pos = entries.len().to_string().len();

    // calculate width for names
    // the length of the longest name, plus one for ':'
    let width_name = 1 + entries
        .iter()
        .map(|(id, _, _)| id.name.len())
        .max()
        .unwrap_or_default();

    // Max possible width for duration is all days above cutoff time
    let width_duration =
        format_duration_with_days(Duration::seconds(*PENALTY_UNFINISHED_DAY * 25)).len();
    // Max possible width for delta duration is all days above cutoff time
    let width_delta_duration =
        format_duration(Duration::seconds(*PENALTY_UNFINISHED_DAY * 25)).len() + 3;
    // Max possible width for penalties
    let width_penalties = "(25 stages out)".len() + 1;

    // Fastest member
    let fastest = entries
        .iter()
        .map(|(_id, time, _count)| time)
        .next()
        .unwrap_or(&0);

    entries
        .iter()
        .enumerate()
        .map(|(idx, (id, total_seconds, penalties))| {
            format!(
                "{:>width_pos$}) {:<width_name$} {:>width_duration$} {:>width_delta_duration$} {:>width_penalties$}",
                // idx is zero-based
                idx + 1,
                id.name,
                format_duration_with_days(Duration::seconds(*total_seconds)),
                match idx == 0 {
                    true => "".to_string(),
                    false => format!(
                        "(+ {})",
                        format_duration(Duration::seconds(*total_seconds - fastest))
                    ),
                },
                match (penalties > &0, penalties==&1) {
                    (true, false) => format!("({penalties} stages out)"),
                    (true, true) => format!("({penalties} stage out)"),
                    (false, _) => "(All stages)".to_string(),
                }
            )
        })
        .join("\n")
}

// Display board from given entries
pub fn board(entries: Vec<(&Identifier, [(u8, usize); 25], usize)>) -> String {
    // calculate width for positions
    // the width of the maximum position to be displayed, plus one for ')'
    let width_pos = entries.len().to_string().len();

    // calculate width for names
    // the length of the longest name, plus one for ':'
    let width_name = 1 + entries
        .iter()
        .map(|(id, _scores, _total)| id.name.len())
        .max()
        .unwrap_or_default();

    // calculate width for scores
    // the width of the maximum score, formatted to two decimal places
    let width_score = entries
        .iter()
        .map(|(_id, _scores, total)| total)
        .max()
        .map(|s| 1 + s.to_string().len())
        .unwrap_or_default();

    entries
        .iter()
        .enumerate()
        .map(|(idx, (id, scores, total))| {
            format!(
                "{:>width_pos$}) {:<width_name$} {:>width_score$}  [{}]",
                // idx is zero-based
                idx + 1,
                id.name,
                total,
                scores
                    .iter()
                    .map(|(n_star, _s)| match n_star {
                        0 => " -",
                        1 => " □",
                        2 => " ■",
                        _ => unreachable!(),
                    })
                    .collect::<String>()
            )
        })
        .join("\n")
}

use crate::error::{BotError, BotResult};
use chrono::{naive::NaiveDateTime, DateTime, Duration, TimeZone, Utc};
use itertools::{Itertools, MinMaxResult};
use scraper::{Node, Selector};
use serde::Serialize;
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    fmt,
    iter::Iterator,
    ops::{Deref, DerefMut},
};

static AOC_PUZZLE_UTC_STARTING_HOUR: u32 = 5;
static AOC_MONTH: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub enum ProblemPart {
    FIRST,
    SECOND,
}

// Leaderboard entry parsed from AoC API.
// Year and day fields match corresponding components of DateTime<Utc>.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize)]
pub struct Entry {
    pub timestamp: DateTime<Utc>,
    pub year: i32,
    pub day: u8,
    pub part: ProblemPart,
    pub id: Identifier,
    pub rank: Option<u8>,
}

// unique identifier for a participant on this leaderboard
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Serialize)]
pub struct Identifier {
    pub name: String,
    pub numeric: u64,
}

type Entries = HashSet<Entry>;

#[derive(Debug)]
pub struct Leaderboard(Entries);

#[derive(Debug)]
pub struct ScrapedLeaderboard {
    pub timestamp: chrono::DateTime<Utc>,
    pub leaderboard: Leaderboard,
}

#[derive(Debug)]
pub struct LeaderboardStatistics {
    pub p1_fast: Option<Duration>,
    pub p1_slow: Option<Duration>,
    pub p2_fast: Option<Duration>,
    pub p2_slow: Option<Duration>,
    // (Delta,final rank (part 2))
    pub delta_fast: Option<(Duration, Option<u8>)>,
    pub delta_slow: Option<(Duration, Option<u8>)>,
}

impl fmt::Display for ProblemPart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProblemPart::FIRST => {
                write!(f, "1")
            }
            ProblemPart::SECOND => {
                write!(f, "2")
            }
        }
    }
}

impl ProblemPart {
    pub fn from(input: usize) -> Self {
        match input {
            1 => ProblemPart::FIRST,
            2 => ProblemPart::SECOND,
            // only two parts for each problem
            _ => unreachable!(),
        }
    }
}

impl Entry {
    /// Parsing of global leaderboard HTML nodes.
    pub fn from_html(
        entry: scraper::element_ref::ElementRef,
        year: i32,
        day: u8,
        part: ProblemPart,
    ) -> Option<Self> {
        let rank_selector = Selector::parse(r#".leaderboard-position"#).unwrap();
        let time_selector = Selector::parse(r#".leaderboard-time"#).unwrap();

        let id = entry
            .value()
            .attr("data-user-id")
            .and_then(|id| id.parse::<u64>().ok());

        // Depending on whether users have declared their github, are sponsors, etc ... the name
        // will be nested in different possible DOM hierarchy layouts.
        let name = entry
            .children()
            .filter_map(|node| match node.value() {
                Node::Text(text) => Some(text.trim()),
                Node::Element(el) => match el.name() {
                    // Name wrapped into <a> tags to link to user's github.
                    "a" => {
                        let text = node.last_child().unwrap().value();
                        let text = text.as_text().unwrap().trim();
                        // We ignore <a> tags related to (AoC++) or (Sponsor) labels.
                        match (text.starts_with("("), text.ends_with(")")) {
                            (false, false) => Some(text),
                            (_, _) => None,
                        }
                    }
                    _ => None,
                },
                _ => None,
            })
            .filter(|text| !text.is_empty())
            .last();

        let rank = match entry.select(&rank_selector).next() {
            Some(text) => match text.text().next() {
                Some(t) => t
                    .split(")")
                    .next()
                    .and_then(|rank| rank.trim().parse::<u8>().ok()),
                None => None,
            },
            None => None,
        };

        let timestamp = match entry.select(&time_selector).next() {
            Some(t) => t
                .text()
                .filter_map(|time| {
                    let with_year = format!("{} {}", year, time);
                    let naive_datetime =
                        NaiveDateTime::parse_from_str(&with_year, "%Y %b %d  %H:%M:%S").ok();
                    naive_datetime
                })
                .map(|d| {
                    // Global leaderboard entries are starting at 00:00:00, so we need to offset by
                    // 5 hours to get real UTC time.
                    DateTime::<Utc>::from_utc(d, Utc)
                        + Duration::hours(AOC_PUZZLE_UTC_STARTING_HOUR.into())
                })
                .last(),
            None => None,
        };

        match (id, name, rank, timestamp) {
            (Some(id), _, Some(rank), Some(timestamp)) => Some(Entry {
                id: Identifier {
                    // Name of anonymous user will be None
                    name: name
                        .map_or(format!("anonymous user #{}", id), |n| n.to_string())
                        .to_string(),
                    numeric: id,
                },
                rank: Some(rank),
                part,
                year,
                day,
                timestamp,
            }),
            _ => None,
        }
    }

    /// Time of the release of the corresponding puzzle.
    pub fn puzzle_unlock(year: i32, day: u8) -> BotResult<DateTime<Utc>> {
        // Problems are released at 05:00:00 UTC
        Utc.with_ymd_and_hms(
            year,
            AOC_MONTH,
            day.into(),
            AOC_PUZZLE_UTC_STARTING_HOUR,
            0,
            0,
        )
        .single()
        .ok_or(BotError::Parse)
    }

    /// generate key from entry
    pub fn to_key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.id.numeric,
            self.part,
            self.rank.unwrap_or_default()
        )
    }

    pub fn duration_since_release(&self) -> BotResult<Duration> {
        let release_time = Entry::puzzle_unlock(self.year, self.day)?;
        Ok(self.timestamp - release_time)
    }
}

impl Leaderboard {
    pub fn new() -> Leaderboard {
        Leaderboard(Entries::new())
    }

    fn is_entry_count_equal_to(&self, n: usize) -> bool {
        self.len() == n
    }

    pub fn is_global_complete(&self) -> bool {
        // 100 entries for each part, so completion of global leaderboard
        // for a specific day is 2*100
        self.is_entry_count_equal_to(200)
    }

    /// (year, member) => (unordered) stars
    fn entries_per_year_member(&self) -> HashMap<(i32, &Identifier), Vec<&Entry>> {
        self.iter().into_group_map_by(|a| (a.year, &a.id))
    }

    /// (year, day, member) => (unordered) stars
    pub fn entries_per_year_day_member(
        &self,
        year: i32,
        day: u8,
    ) -> HashMap<(i32, u8, &Identifier), Vec<&Entry>> {
        self.iter()
            .filter(|s| s.year == year && s.day == day)
            .into_group_map_by(|e| (e.year, e.day, &e.id))
    }

    /// (year, day, part) => (unordered) stars
    fn entries_per_year_day_part(&self) -> HashMap<(i32, u8, ProblemPart), Vec<&Entry>> {
        self.iter().into_group_map_by(|a| (a.year, a.day, a.part))
    }

    /// all members ids
    fn members_ids(&self) -> HashSet<u64> {
        self.iter().map(|e| e.id.numeric).collect()
    }

    /// (year, id) => [score per day for that year]
    pub fn daily_scores_per_year_member(&self) -> HashMap<(i32, &Identifier), [usize; 25]> {
        // Max point earned for each star is number of members in leaderboard
        let members_solutions = self.entries_per_year_member();
        let n_members_per_year = members_solutions
            .iter()
            .map(|((y, id), _)| (y, id))
            .into_grouping_map_by(|(y, _)| *y)
            .fold(0, |acc, _key, _val| acc + 1);

        let standings_per_challenge = self.ranked_members_per_year_day_part();
        standings_per_challenge.iter().fold(
            HashMap::new(),
            |mut acc, ((year, day, _part), star_rank)| {
                star_rank
                    .iter()
                    .enumerate()
                    .for_each(|(rank_minus_one, id)| {
                        // unwrap is safe here as we know the year exists
                        let star_score = n_members_per_year.get(&year).unwrap() - rank_minus_one;
                        let day_scores = acc.entry((*year, id)).or_insert([0; 25]);
                        day_scores[(*day - 1) as usize] += star_score;
                    });
                acc
            },
        )
    }

    /// (year, id) => [(n stars, score per day) for that year]
    pub fn daily_stars_scores_per_year_member(
        &self,
    ) -> HashMap<(i32, &Identifier), [(u8, usize); 25]> {
        // Max point earned for each star is number of members in leaderboard
        let members_solutions = self.entries_per_year_member();
        let n_members_per_year = members_solutions
            .iter()
            .map(|((y, id), _)| (y, id))
            .into_grouping_map_by(|(y, _)| *y)
            .fold(0, |acc, _key, _val| acc + 1);

        let standings_per_challenge = self.ranked_members_per_year_day_part();
        standings_per_challenge.iter().fold(
            HashMap::new(),
            |mut acc, ((year, day, _part), star_rank)| {
                star_rank
                    .iter()
                    .enumerate()
                    .for_each(|(rank_minus_one, id)| {
                        // unwrap is safe here as we know the year exists
                        let star_score = n_members_per_year.get(&year).unwrap() - rank_minus_one;
                        let day_stars_scores = acc.entry((*year, id)).or_insert([(0, 0); 25]);
                        day_stars_scores[(*day - 1) as usize].0 += 1;
                        day_stars_scores[(*day - 1) as usize].1 += star_score;
                    });
                acc
            },
        )
    }

    fn local_scores_per_year_member(&self) -> HashMap<(i32, &Identifier), usize> {
        self.daily_scores_per_year_member()
            .iter()
            .map(|((year, id), daily_scores)| ((*year, *id), daily_scores.iter().sum()))
            .collect()
    }

    pub fn get_members_entries_union_with(&self, other: &Leaderboard) -> Vec<&Entry> {
        let other_members_ids = other.members_ids();
        self.iter()
            .filter(|entry| other_members_ids.contains(&entry.id.numeric))
            .collect::<Vec<&Entry>>()
    }

    /// (year, day, part) => [ordered members]
    fn ranked_members_per_year_day_part(
        &self,
    ) -> HashMap<(i32, u8, ProblemPart), Vec<&Identifier>> {
        self.entries_per_year_day_part()
            .into_iter()
            .map(|(challenge, entries)| {
                (
                    challenge,
                    entries
                        .into_iter()
                        // sort solutions chronologically by timestamp
                        .sorted_unstable()
                        // retrieve author of the solution
                        .map(|s| &s.id)
                        .collect(),
                )
            })
            .collect::<HashMap<(i32, u8, ProblemPart), Vec<&Identifier>>>()
    }

    fn min_max_times_for_year_day(
        &self,
        year: i32,
        day: u8,
    ) -> HashMap<ProblemPart, (DateTime<Utc>, DateTime<Utc>)> {
        // Compute max time for each part, in order to infer deltas for members who only scored
        // one part of the global leaderboard that day.
        self.iter()
            .filter(|s| s.year == year && s.day == day)
            .into_group_map_by(|s| s.part)
            .iter()
            .map(
                |(p, solutions)| match solutions.iter().minmax_by_key(|s| s.timestamp) {
                    MinMaxResult::OneElement(s) => (*p, (s.timestamp, s.timestamp)),
                    MinMaxResult::MinMax(s1, s2) => (*p, (s1.timestamp, s2.timestamp)),
                    MinMaxResult::NoElements => unreachable!(),
                },
            )
            .collect::<HashMap<ProblemPart, (DateTime<Utc>, DateTime<Utc>)>>()
    }

    /// year => ordered vec of (name, score)
    pub fn standings_by_local_score_per_year(&self) -> HashMap<i32, Vec<(String, usize)>> {
        let scores = self.local_scores_per_year_member();

        scores
            .into_iter()
            .into_grouping_map_by(|((year, _id), _score)| *year)
            .fold(vec![], |mut acc, _key, ((_year, id), score)| {
                acc.push((id.name.clone(), score));
                acc
            })
            .into_iter()
            .map(|(year, scores)| {
                (
                    year,
                    scores
                        .into_iter()
                        .sorted_by_key(|x| Reverse(x.1))
                        .collect_vec(),
                )
            })
            .collect::<HashMap<i32, Vec<(String, usize)>>>()
    }

    /// year => ordered vec of (name, number of stars)
    pub fn standings_by_number_of_stars_per_year(&self) -> HashMap<i32, Vec<(String, usize)>> {
        let stars = self.entries_per_year_member();

        stars
            .into_iter()
            .into_grouping_map_by(|((year, _id), _stars)| *year)
            .fold(vec![], |mut acc, _key, ((_year, id), stars)| {
                acc.push((id.name.clone(), stars.len()));
                acc
            })
            .into_iter()
            .map(|(year, stars)| {
                (
                    year,
                    stars
                        .into_iter()
                        .sorted_by_key(|x| Reverse(x.1))
                        .collect_vec(),
                )
            })
            .collect::<HashMap<i32, Vec<(String, usize)>>>()
    }

    /// ordered vec of (name, local score)
    pub fn standings_by_local_score_for_year_day(
        &self,
        year: i32,
        day: usize,
    ) -> Vec<(String, usize)> {
        self.daily_scores_per_year_member()
            .iter()
            .filter(|((y, _id), _daily_scores)| y == &year)
            .map(|((_year, id), daily_scores)| (id.name.clone(), daily_scores[day - 1]))
            .filter(|(_, score)| *score > 0)
            .sorted_by_key(|m| Reverse(m.1))
            .collect::<Vec<(String, usize)>>()
    }

    /// ordered vec of (name, duration, final rank)
    pub fn standings_by_delta_for_year_day(
        &self,
        year: i32,
        day: u8,
    ) -> BotResult<Vec<(String, Duration, Option<u8>)>> {
        // We will use max time of part 1 to infer deltas for members who only scored
        // the second part on that day.
        let max_time_first_part = self
            .min_max_times_for_year_day(year, day)
            .get(&ProblemPart::FIRST)
            .and_then(|(_p1_fast, p1_slow)| Some(*p1_slow))
            .ok_or(BotError::Compute(
                "MinMax times could not be computed".to_string(),
            ))?;

        let standings = self
            .entries_per_year_day_member(year, day)
            .into_iter()
            .filter_map(
                |((_year, _day, id), solutions_for_day)| match solutions_for_day.len() {
                    1 => {
                        // unwrap is safe as len == 1
                        let entry = solutions_for_day.last().unwrap();
                        match entry.part {
                            ProblemPart::FIRST => None,
                            ProblemPart::SECOND => {
                                // Overtimed on first part, but came back strong to score second part
                                // Duration is > (part.1, - max first part). We'll substract 1 sec.
                                Some((
                                    id.name.clone(),
                                    entry.timestamp - max_time_first_part - Duration::seconds(1),
                                    entry.rank,
                                ))
                            }
                        }
                    }
                    2 => {
                        let mut ordered_parts =
                            solutions_for_day.iter().sorted_by_key(|s| s.timestamp);
                        // safe unwrap since len == 2
                        let (first, second) =
                            (ordered_parts.next().unwrap(), ordered_parts.next().unwrap());
                        Some((
                            id.name.clone(),
                            second.timestamp - first.timestamp,
                            second.rank,
                        ))
                    }
                    _ => unreachable!(),
                },
            )
            .sorted_by_key(|r| r.1)
            .collect::<Vec<(String, Duration, Option<u8>)>>();
        Ok(standings)
    }

    pub fn statistics_for_year_day(&self, year: i32, day: u8) -> BotResult<LeaderboardStatistics> {
        let minmax_by_part = self.min_max_times_for_year_day(year, day);
        let (p1_fast, p1_slow) =
            minmax_by_part
                .get(&ProblemPart::FIRST)
                .ok_or(BotError::Compute(
                    "Could not retrieve minmax for part 1".to_string(),
                ))?;
        let (p2_fast, p2_slow) =
            minmax_by_part
                .get(&ProblemPart::SECOND)
                .ok_or(BotError::Compute(
                    "Could not retrieve minmax for part 2".to_string(),
                ))?;

        let sorted_deltas = self.standings_by_delta_for_year_day(year, day)?;
        let mut sorted_deltas_iter = sorted_deltas.iter();

        let challenge_start_time = Entry::puzzle_unlock(year, day)?;

        let stats = LeaderboardStatistics {
            p1_fast: Some(*p1_fast - challenge_start_time),
            p1_slow: Some(*p1_slow - challenge_start_time),
            p2_fast: Some(*p2_fast - challenge_start_time),
            p2_slow: Some(*p2_slow - challenge_start_time),
            delta_fast: sorted_deltas_iter
                .next()
                .and_then(|(_name, duration, rank)| Some((*duration, *rank))),
            delta_slow: sorted_deltas_iter
                .last()
                .and_then(|(_name, duration, rank)| Some((*duration, *rank))),
        };
        Ok(stats)
    }

    pub fn show_year(&self, year: i32) -> String {
        let scores = self.daily_stars_scores_per_year_member();
        let entries = scores
            .iter()
            .filter(|((y, _id), _scores)| y == &year)
            .map(|((_y, id), scores)| {
                // we compute total score, and total number of stars
                (
                    id,
                    scores,
                    scores.iter().fold((0, 0), |acc, s| {
                        // (number of stars, score)
                        (acc.0 + s.0, acc.1 + s.1)
                    }),
                )
            })
            // sort by score descending, then by star count descending
            .sorted_unstable_by_key(|entry| (Reverse(entry.2 .1), Reverse(entry.2 .0)))
            .collect::<Vec<_>>();

        // calculate width for positions
        // the width of the maximum position to be displayed, plus one for ')'
        let width_pos = entries.len().to_string().len();

        // calculate width for names
        // the length of the longest name, plus one for ':'
        let width_name = 1 + entries
            .iter()
            .map(|(id, _scores, (_n_stars, _total))| id.name.len())
            .max()
            .unwrap_or_default();

        // calculate width for scores
        // the width of the maximum score, formatted to two decimal places
        let width_score = entries
            .iter()
            .map(|(_id, _scores, (_n_stars, total))| total)
            .max()
            .map(|s| 1 + s.to_string().len())
            .unwrap_or_default();

        entries
            .iter()
            .enumerate()
            .map(|(idx, (id, scores, (_n_stars, total)))| {
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
}

impl Deref for Leaderboard {
    type Target = Entries;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Leaderboard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ScrapedLeaderboard {
    pub fn new() -> ScrapedLeaderboard {
        ScrapedLeaderboard {
            timestamp: Utc::now(),
            leaderboard: Leaderboard::new(),
        }
    }

    pub fn merge_with(&mut self, other: ScrapedLeaderboard) {
        self.timestamp = other.timestamp;
        self.leaderboard
            .extend(other.leaderboard.clone().into_iter());
    }
}

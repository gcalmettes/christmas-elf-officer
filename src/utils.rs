use chrono::{naive::NaiveDateTime, DateTime, Duration, Utc};

pub fn suffix(num: u8) -> &'static str {
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

pub fn challenge_release_time(year: i32, day: u8) -> DateTime<Utc> {
    // Problems are released at 06:00:00 UTC
    DateTime::<Utc>::from_utc(
        NaiveDateTime::parse_from_str(
            format!("{year}-12-{day} 06:00:00").as_str(),
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap(),
        Utc,
    )
}

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.num_seconds() % 60;
    let minutes = (duration.num_seconds() / 60) % 60;
    let hours = (duration.num_seconds() / 60) / 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds,)
}

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

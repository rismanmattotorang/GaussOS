// src/memory/temporal_parse.rs
//! Temporal expression resolution (Phase 1, roadmap #4).
//!
//! Agent conversations mix absolute dates ("March 2024", "2024-03-15") with
//! relative references ("yesterday", "two weeks ago", "last month"). To place a
//! fact correctly on the bi-temporal `valid_at` axis (the edge Zep emphasises),
//! GaussOS resolves both kinds against a reference instant — pure Rust, no LLM
//! required.

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};

/// Resolve the first temporal expression found in `text` to an instant,
/// interpreted relative to `reference` (usually the message timestamp).
/// Returns `None` when no recognised expression is present.
pub fn resolve(text: &str, reference: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let t = text.to_lowercase();

    // --- Keyword relatives ---
    if t.contains("yesterday") {
        return Some(reference - Duration::days(1));
    }
    if t.contains("tomorrow") {
        return Some(reference + Duration::days(1));
    }
    if t.contains("today") || t.contains("right now") || t.contains("just now") {
        return Some(reference);
    }
    if t.contains("last week") {
        return Some(reference - Duration::weeks(1));
    }
    if t.contains("last month") {
        return Some(shift_months(reference, -1));
    }
    if t.contains("last year") {
        return Some(shift_months(reference, -12));
    }
    if t.contains("next week") {
        return Some(reference + Duration::weeks(1));
    }

    // --- "N <unit> ago" / "in N <unit>" ---
    if let Some(dt) = parse_offset(&t, reference) {
        return Some(dt);
    }

    // --- Absolute ISO date: YYYY-MM-DD ---
    if let Some(dt) = parse_iso_date(&t) {
        return Some(dt);
    }

    // --- "<Month> <Year>" e.g. "march 2024" ---
    if let Some(dt) = parse_month_year(&t) {
        return Some(dt);
    }

    None
}

/// Add `months` (which may be negative) to a timestamp, clamping the day.
fn shift_months(dt: DateTime<Utc>, months: i32) -> DateTime<Utc> {
    let total = dt.year() * 12 + (dt.month0() as i32) + months;
    let year = total.div_euclid(12);
    let month0 = total.rem_euclid(12) as u32;
    let day = dt.day().min(days_in_month(year, month0 + 1));
    Utc.with_ymd_and_hms(year, month0 + 1, day, dt.hour(), dt.minute(), dt.second())
        .single()
        .unwrap_or(dt)
}

use chrono::Timelike;

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    let first_this = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    (first_next - first_this).num_days() as u32
}

fn parse_offset(t: &str, reference: DateTime<Utc>) -> Option<DateTime<Utc>> {
    // Matches "<n> <unit> ago" or "in <n> <unit>" with worded small numbers.
    let words = [
        ("a", 1i64), ("an", 1), ("one", 1), ("two", 2), ("three", 3), ("four", 4),
        ("five", 5), ("six", 6), ("seven", 7), ("eight", 8), ("nine", 9), ("ten", 10),
    ];
    let tokens: Vec<&str> = t.split_whitespace().collect();
    for (i, tok) in tokens.iter().enumerate() {
        // amount is either a digit or a small worded number
        let amount: Option<i64> = tok
            .trim_matches(|c: char| !c.is_alphanumeric())
            .parse::<i64>()
            .ok()
            .or_else(|| words.iter().find(|(w, _)| *w == *tok).map(|(_, n)| *n));
        let Some(amount) = amount else { continue };
        let Some(unit) = tokens.get(i + 1) else { continue };
        let ago = tokens.get(i + 2).map(|s| s.starts_with("ago")).unwrap_or(false)
            || tokens.get(i + 3).map(|s| s.starts_with("ago")).unwrap_or(false);
        let forward = i > 0 && tokens[i - 1] == "in";
        let sign = if ago { -1 } else if forward { 1 } else { continue };
        let delta = match *unit {
            u if u.starts_with("day") => Duration::days(amount),
            u if u.starts_with("week") => Duration::weeks(amount),
            u if u.starts_with("hour") => Duration::hours(amount),
            u if u.starts_with("minute") => Duration::minutes(amount),
            u if u.starts_with("month") => return Some(shift_months(reference, (sign as i32) * amount as i32)),
            u if u.starts_with("year") => return Some(shift_months(reference, (sign as i32) * amount as i32 * 12)),
            _ => continue,
        };
        return Some(reference + delta * sign as i32);
    }
    None
}

fn parse_iso_date(t: &str) -> Option<DateTime<Utc>> {
    for token in t.split(|c: char| c == ' ' || c == ',' || c == '.') {
        let token = token.trim();
        if token.len() == 10 && token.as_bytes().get(4) == Some(&b'-') {
            if let Ok(d) = NaiveDate::parse_from_str(token, "%Y-%m-%d") {
                return Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?).into();
            }
        }
    }
    None
}

fn parse_month_year(t: &str) -> Option<DateTime<Utc>> {
    const MONTHS: &[(&str, u32)] = &[
        ("january", 1), ("february", 2), ("march", 3), ("april", 4), ("may", 5),
        ("june", 6), ("july", 7), ("august", 8), ("september", 9), ("october", 10),
        ("november", 11), ("december", 12),
    ];
    let tokens: Vec<&str> = t.split_whitespace().collect();
    for (i, tok) in tokens.iter().enumerate() {
        let clean = tok.trim_matches(|c: char| !c.is_alphabetic());
        if let Some((_, m)) = MONTHS.iter().find(|(name, _)| *name == clean) {
            if let Some(next) = tokens.get(i + 1) {
                if let Ok(year) = next.trim_matches(|c: char| !c.is_numeric()).parse::<i32>() {
                    if (1900..=2200).contains(&year) {
                        let d = NaiveDate::from_ymd_opt(year, *m, 1)?;
                        return Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?).into();
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 31, 12, 0, 0).unwrap()
    }

    #[test]
    fn yesterday_today_tomorrow() {
        let r = reference();
        assert_eq!(resolve("I saw it yesterday", r).unwrap().day(), 30);
        assert_eq!(resolve("today is good", r).unwrap().day(), 31);
        assert_eq!(resolve("see you tomorrow", r).unwrap().day(), 1); // June 1
    }

    #[test]
    fn n_units_ago() {
        let r = reference();
        assert_eq!(resolve("started two weeks ago", r).unwrap().day(), 17);
        assert_eq!(resolve("3 days ago", r).unwrap().day(), 28);
        // a month ago -> April 30 (clamped) or 30
        let m = resolve("a month ago", r).unwrap();
        assert_eq!(m.month(), 4);
    }

    #[test]
    fn last_week_month_year() {
        let r = reference();
        assert_eq!(resolve("last week", r).unwrap().day(), 24);
        assert_eq!(resolve("last month", r).unwrap().month(), 4);
        assert_eq!(resolve("last year", r).unwrap().year(), 2025);
    }

    #[test]
    fn absolute_iso_and_month_year() {
        let r = reference();
        let iso = resolve("on 2024-03-15 we met", r).unwrap();
        assert_eq!((iso.year(), iso.month(), iso.day()), (2024, 3, 15));
        let my = resolve("back in March 2024", r).unwrap();
        assert_eq!((my.year(), my.month()), (2024, 3));
    }

    #[test]
    fn none_when_absent() {
        assert!(resolve("just a normal sentence", reference()).is_none());
    }
}

//! `datetime.fromisoformat` and the subtraction of two of its results, as CPython 3.14 does
//! them.
//!
//! `records.duration_seconds` reads a record's two timestamps with `fromisoformat`, and
//! since 3.11 that accepts far more than what Hydra writes: a week date, a time with no
//! colons, any character at all between the date and the time, an offset with seconds and a
//! fraction. A record is whatever a writer put there, so the port follows the C parser
//! byte for byte, its looseness included, rather than a stricter one that would answer
//! `None` where the Python answered a number. The parser walks UTF-8 bytes and reads a NUL
//! past the end, as the C does, which is why a time may be followed by one stray character
//! before its offset and a fraction of six digits may be followed by anything.
//!
//! `tests/differential.rs` checks this against some 2,200 pairs of timestamps recorded from
//! Python 3.14.7, most of them malformed on purpose.

use hydra_pyjson::json::PyStr;

/// A `datetime`: its wall-clock time as microseconds since 0001-01-01T00:00:00, and its UTC
/// offset in microseconds when it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    local: i128,
    offset: Option<i128>,
}

const DAY: i128 = 86_400_000_000;

fn is_leap(year: i128) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i128, month: i128) -> i128 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn days_before_year(year: i128) -> i128 {
    let y = year - 1;
    y * 365 + y / 4 - y / 100 + y / 400
}

/// The proleptic Gregorian ordinal, 1 for 0001-01-01.
fn ordinal(year: i128, month: i128, day: i128) -> i128 {
    let before_month: i128 = (1..month).map(|m| days_in_month(year, m)).sum();
    days_before_year(year) + before_month + day
}

/// The year, month and day of an ordinal. The year may come back as 0 or 10000, which the
/// range check after it refuses.
fn from_ordinal(ordinal: i128) -> (i128, i128, i128) {
    let mut year = (ordinal - 1).div_euclid(366) + 1;
    while days_before_year(year + 1) < ordinal {
        year += 1;
    }
    let mut day = ordinal - days_before_year(year);
    let mut month = 1;
    while day > days_in_month(year, month) {
        day -= days_in_month(year, month);
        month += 1;
    }
    (year, month, day)
}

/// The byte at `i`, and the NUL a C string ends with past the last one.
fn byte(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// `parse_digits`: exactly `count` ASCII digits from `at`, and where they end.
fn digits(s: &[u8], at: usize, count: usize) -> Option<(i128, usize)> {
    let mut value = 0;
    for i in at..at + count {
        let c = byte(s, i);
        if !c.is_ascii_digit() {
            return None;
        }
        value = value * 10 + i128::from(c - b'0');
    }
    Some((value, at + count))
}

/// `_find_isoformat_datetime_separator`: where the date ends, told from characters 4 and 5
/// and from how many digits follow a week. `None` for the one length the C refuses here.
fn separator(s: &[u8]) -> Option<usize> {
    let len = s.len();
    if len == 7 {
        return Some(7);
    }
    if byte(s, 4) == b'-' {
        if byte(s, 5) != b'W' {
            return Some(10);
        }
        if len > 8 && byte(s, 8) == b'-' {
            if len == 9 {
                return None;
            }
            // `YYYY-Www-##`: a hyphen at 8 as the separator, not a digit at 10.
            if len > 10 && byte(s, 10).is_ascii_digit() {
                return Some(8);
            }
            return Some(10);
        }
        return Some(8);
    }
    if byte(s, 4) != b'W' {
        return Some(8);
    }
    let mut idx = 7;
    while idx < len && byte(s, idx).is_ascii_digit() {
        idx += 1;
    }
    Some(if idx < 9 {
        idx
    } else if idx % 2 == 0 {
        7
    } else {
        8
    })
}

/// `iso_to_ymd`: an ISO year, week and weekday as a calendar date.
fn week_date(year: i128, week: i128, day: i128) -> Option<(i128, i128, i128)> {
    if !(1..=9999).contains(&year) {
        return None;
    }
    // Monday is 0. Ordinal 1 was a Monday.
    let first_weekday = (ordinal(year, 1, 1) + 6) % 7;
    if !(1..53).contains(&week) {
        // A year has a week 53 when it starts on a Thursday, or on a Wednesday and is leap.
        let long_year = first_weekday == 3 || (first_weekday == 2 && is_leap(year));
        if !(week == 53 && long_year) {
            return None;
        }
    }
    if !(1..8).contains(&day) {
        return None;
    }
    let first_day = ordinal(year, 1, 1);
    let mut week1_monday = first_day - first_weekday;
    if first_weekday > 3 {
        week1_monday += 7;
    }
    Some(from_ordinal(week1_monday + (week - 1) * 7 + day - 1))
}

/// `parse_isoformat_date` over the first `len` bytes. It reads past `len` where the C does:
/// only the weekday of a week date looks at it.
fn parse_date(s: &[u8], len: usize) -> Option<(i128, i128, i128)> {
    let (year, mut p) = digits(s, 0, 4)?;
    let dashed = byte(s, p) == b'-';
    if dashed {
        p += 1;
    }
    if byte(s, p) == b'W' {
        let (week, after) = digits(s, p + 1, 2)?;
        p = after;
        let mut day = 1;
        if p < len {
            if dashed {
                if byte(s, p) != b'-' {
                    return None;
                }
                p += 1;
            }
            day = digits(s, p, 1)?.0;
        }
        return week_date(year, week, day);
    }
    let (month, mut p) = digits(s, p, 2)?;
    if dashed {
        if byte(s, p) != b'-' {
            return None;
        }
        p += 1;
    }
    let (day, _) = digits(s, p, 2)?;
    Some((year, month, day))
}

/// Hours, minutes, seconds and microseconds, and whether something other than the end of
/// the string followed them.
struct Clock {
    fields: [i128; 4],
    trailing: bool,
}

/// `parse_hh_mm_ss_ff` over `s[start..end]`: `HH[:MM[:SS[.fff[fff]]]]` with or without the
/// colons. What it lets through is the C's: one character of any kind right before `end`,
/// and anything after six digits of fraction.
fn parse_clock(s: &[u8], start: usize, end: usize) -> Option<Clock> {
    let mut fields = [0; 4];
    let mut p = start;
    let mut colons = true;
    for i in 0..3 {
        (fields[i], p) = digits(s, p, 2)?;
        let c = byte(s, p);
        p += 1;
        if i == 0 {
            colons = c == b':';
        }
        let mark = c == b'.' || c == b',';
        if p >= end {
            // A decimal mark with no digit after it is refused; any other character here
            // only says the string went on.
            return (!mark).then_some(Clock {
                fields,
                trailing: c != 0,
            });
        }
        if colons && c == b':' {
            if i == 2 {
                return None;
            }
        } else if mark {
            if i < 2 {
                return None;
            }
            break;
        } else if !colons {
            p -= 1;
        } else {
            return None;
        }
    }
    let count = (end - p).min(6);
    let (fraction, mut p) = digits(s, p, count)?;
    fields[3] = fraction * 10_i128.pow((6 - count) as u32);
    while byte(s, p).is_ascii_digit() {
        p += 1;
    }
    Some(Clock {
        fields,
        trailing: byte(s, p) != 0,
    })
}

/// `parse_isoformat_time` over `s[start..]`: the clock, and the offset in microseconds when
/// one is written.
fn parse_time(s: &[u8], start: usize) -> Option<([i128; 4], Option<i128>)> {
    let end = s.len();
    // The C tests the first byte before it tests the length, so an empty time looks one
    // byte past the end, finds the NUL and fails on it.
    let mut zone = start;
    loop {
        if matches!(byte(s, zone), b'Z' | b'+' | b'-') {
            break;
        }
        zone += 1;
        if zone >= end {
            break;
        }
    }
    let clock = parse_clock(s, start, zone)?;
    if zone >= end {
        return (!clock.trailing).then_some((clock.fields, None));
    }
    if byte(s, zone) == b'Z' {
        return (byte(s, zone + 1) == 0).then_some((clock.fields, Some(0)));
    }
    let sign = if byte(s, zone) == b'-' { -1 } else { 1 };
    let offset = parse_clock(s, zone + 1, end)?;
    if offset.trailing {
        return None;
    }
    let [hours, minutes, seconds, micros] = offset.fields;
    let offset = sign * ((hours * 3600 + minutes * 60 + seconds) * 1_000_000 + micros);
    // `timezone()` takes an offset strictly inside a day either way, and checks no field.
    (offset.abs() < DAY).then_some((clock.fields, Some(offset)))
}

/// `datetime.fromisoformat(text)`, `None` where it raises `ValueError`.
pub fn fromisoformat(text: &PyStr) -> Option<DateTime> {
    let codes = text.code_points();
    if codes.len() < 7 {
        return None;
    }
    // A surrogate is allowed as the separator, which sits at 7, 8 or 10: the first of those
    // that holds one becomes a `T`. Any other surrogate fails the encoding below.
    let surrogate = [7, 8, 10]
        .into_iter()
        .find(|at| codes.get(*at).is_some_and(|c| (0xd800..0xe000).contains(c)));
    let mut encoded = String::new();
    for (at, code) in codes.iter().enumerate() {
        if Some(at) == surrogate {
            encoded.push('T');
        } else {
            encoded.push(char::from_u32(*code)?);
        }
    }
    let s = encoded.as_bytes();

    let cut = separator(s)?;
    let (mut year, mut month, mut day) = parse_date(s, cut)?;
    let (mut clock, mut offset) = ([0; 4], None);
    if s.len() > cut {
        // The separator is one character, however many bytes it takes.
        let width = match s[cut] {
            lead if lead & 0x80 == 0 => 1,
            lead if lead & 0xf0 == 0xe0 => 3,
            lead if lead & 0xf0 == 0xf0 => 4,
            _ => 2,
        };
        (clock, offset) = parse_time(s, cut + width)?;
    }
    let [mut hour, minute, second, micros] = clock;

    // 24:00:00 is midnight of the next day, and only on a date that exists.
    if hour == 24 && (1..=12).contains(&month) && day <= days_in_month(year, month) {
        if minute != 0 || second != 0 || micros != 0 {
            return None;
        }
        hour = 0;
        day += 1;
        if day > days_in_month(year, month) {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }

    let valid = (1..=9999).contains(&year)
        && (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && (0..24).contains(&hour)
        && (0..60).contains(&minute)
        && (0..60).contains(&second);
    if !valid {
        return None;
    }
    let local = (ordinal(year, month, day) - 1) * DAY
        + ((hour * 60 + minute) * 60 + second) * 1_000_000
        + micros;
    Some(DateTime { local, offset })
}

/// `(ended - started).total_seconds()`. `None` where the subtraction raises `TypeError`:
/// one of the two carries an offset and the other does not.
pub fn seconds_between(started: &DateTime, ended: &DateTime) -> Option<f64> {
    let micros = match (started.offset, ended.offset) {
        (None, None) => ended.local - started.local,
        (Some(from), Some(to)) => (ended.local - to) - (started.local - from),
        _ => return None,
    };
    Some(micros_to_seconds(micros))
}

/// `micros / 10**6` as Python divides two ints: the nearest float to the true quotient.
/// Converting first and dividing after rounds twice once the count passes 2^53, which a
/// span of 286 years does.
fn micros_to_seconds(micros: i128) -> f64 {
    let magnitude = micros.unsigned_abs();
    let nearest = if magnitude < 1 << 53 {
        magnitude as f64 / 1e6
    } else {
        // 63 or more significant bits of the quotient, then round half to even at 53,
        // with the division's remainder breaking what looks like a tie.
        const SHIFT: u32 = 30;
        let scaled = magnitude << SHIFT;
        let (quotient, remainder) = (scaled / 1_000_000, scaled % 1_000_000);
        let extra = (128 - quotient.leading_zeros()) - 53;
        let half = 1_u128 << (extra - 1);
        let low = quotient & ((1 << extra) - 1);
        let mut kept = quotient >> extra;
        if low > half || (low == half && (remainder != 0 || kept & 1 == 1)) {
            kept += 1;
        }
        kept as f64 * 2_f64.powi(extra as i32 - SHIFT as i32)
    };
    if micros < 0 { -nearest } else { nearest }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Option<DateTime> {
        fromisoformat(&PyStr::from(text))
    }

    #[test]
    fn a_run_of_ten_seconds_is_ten_seconds() {
        let started = read("2026-09-24T00:00:00+00:00").unwrap();
        let ended = read("2026-09-24T00:00:10+00:00").unwrap();
        assert_eq!(seconds_between(&started, &ended), Some(10.0));
        assert_eq!(seconds_between(&ended, &started), Some(-10.0));
    }

    #[test]
    fn a_time_with_an_offset_and_one_without_do_not_subtract() {
        let naive = read("2026-09-24T00:00:00").unwrap();
        let aware = read("2026-09-24T00:00:10+00:00").unwrap();
        assert_eq!(seconds_between(&naive, &aware), None);
    }

    #[test]
    fn ordinals_round_trip_over_every_month_end() {
        for year in [1, 4, 100, 400, 1900, 2000, 2024, 2026, 9999] {
            for month in 1..=12 {
                for day in [1, days_in_month(year, month)] {
                    let n = ordinal(year, month, day);
                    assert_eq!(from_ordinal(n), (year, month, day));
                }
            }
        }
        assert_eq!(ordinal(1, 1, 1), 1);
        assert_eq!(ordinal(9999, 12, 31), 3_652_059);
    }
}

//! Absolute-time phrases, e.g. `at 5pm`, `tomorrow at 9am`, `tonight`.
//!
//! The grammar this stage recognizes, after any leading filler:
//!
//! ```text
//! [day] "at" <clock> <message>
//! [day] <daypart>     <message>
//! <strong-clock>      <message>
//! ```
//!
//! `day` is `today`, `tomorrow`, `tonight`, a weekday, or `next <weekday>`.
//! `clock` is `5pm`, `5 pm`, `5:30pm`, `17:30`, `9am`, `noon`, or `midnight`.
//! `daypart` is `morning`, `afternoon`, `evening`, or `night`. Times that only
//! imply an hour (dayparts, a bare `at 5`) lower the confidence rather than
//! being guessed silently.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Weekday};

use crate::parser::lex::{Token, is_filler_prefix, split_leading_connector, tokenize};
use crate::parser::spans::{ParsedSpan, SpanKind};
use crate::parser::{Confidence, ParseError, ParsedReminder};

/// Parse an absolute-time reminder from `input` relative to `now`.
pub fn parse(input: &str, now: DateTime<Local>) -> Result<ParsedReminder, ParseError> {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return Err(ParseError::MissingTime);
    }

    let mut prefix_end = 0;
    while prefix_end < tokens.len() && is_filler_prefix(tokens[prefix_end].text) {
        prefix_end += 1;
    }

    let phrase = parse_time_phrase(&tokens, prefix_end, now).ok_or(ParseError::MissingTime)?;

    let message_tokens = &tokens[phrase.after_index.min(tokens.len())..];
    let (connector, message_tokens) = split_leading_connector(message_tokens);
    let (msg_start, msg_end) = match (message_tokens.first(), message_tokens.last()) {
        (Some(first), Some(last)) => (first.start, last.end),
        _ => return Err(ParseError::MissingMessage),
    };
    let message = input[msg_start..msg_end].to_string();

    let spans = build_spans(&tokens, prefix_end, &phrase, connector, msg_start, msg_end);

    Ok(ParsedReminder {
        original: input.to_string(),
        due_at: phrase.due_at,
        message,
        spans,
        confidence: phrase.confidence,
    })
}

/// The recognized time expression: where it sits, when it resolves to, and how
/// confident we are.
struct TimePhrase {
    time_start: usize,
    time_end: usize,
    after_index: usize,
    due_at: DateTime<Local>,
    confidence: Confidence,
    /// A stand-alone `at` keyword to dim, when it is not already inside the time
    /// span (i.e. no day preceded it).
    at_filler: Option<(usize, usize)>,
}

/// How a resolved date should roll forward if it lands in the past.
enum Roll {
    /// Never roll (an explicit `today`/`tomorrow`).
    None,
    /// Roll to the next day (an implied "today" from a bare clock).
    Day,
    /// Roll to the next week (a weekday that has already passed today).
    Week,
}

/// A parsed day anchor and the tokens it consumed.
struct Day {
    date: NaiveDate,
    had: bool,
    next: usize,
    start: Option<usize>,
    end: Option<usize>,
    implied: Option<Daypart>,
    roll: Roll,
}

fn parse_time_phrase(tokens: &[Token], start: usize, now: DateTime<Local>) -> Option<TimePhrase> {
    let day = parse_day(tokens, start, now.date_naive());
    let idx = day.next;

    let (time, time_end, time_start, after, confidence, at_filler) =
        if let Some(daypart) = day.implied {
            // `tonight` carries its own time of day.
            let (time, precise) = daypart.time();
            (
                time,
                day.end?,
                day.start?,
                idx,
                daypart_confidence(precise),
                None,
            )
        } else if let Some(token) = tokens.get(idx) {
            if token.text.eq_ignore_ascii_case("at") {
                let primary = tokens.get(idx + 1)?;
                let next = tokens.get(idx + 2).map(|t| t.text);
                let (clock, consumed_next) = parse_clock(primary.text, next)?;
                let time = NaiveTime::from_hms_opt(clock.hour, clock.minute, 0)?;
                let end = if consumed_next {
                    tokens[idx + 2].end
                } else {
                    primary.end
                };
                let after = idx + 2 + usize::from(consumed_next);
                let confidence = if clock.strong {
                    Confidence::High
                } else {
                    // A bare `at 5` is genuinely ambiguous about am/pm.
                    Confidence::Low
                };
                let at_filler = if day.start.is_none() {
                    Some((token.start, token.end))
                } else {
                    None
                };
                let time_start = day.start.unwrap_or(primary.start);
                (time, end, time_start, after, confidence, at_filler)
            } else if let Some(daypart) = Daypart::from_word(&token.text.to_ascii_lowercase()) {
                let (time, precise) = daypart.time();
                let time_start = day.start.unwrap_or(token.start);
                (
                    time,
                    token.end,
                    time_start,
                    idx + 1,
                    daypart_confidence(precise),
                    None,
                )
            } else if !day.had {
                // A leading clock with no `at`, e.g. `5pm meeting`. Require it to be
                // unambiguous (has am/pm or a colon) so we do not swallow numbers
                // that are really part of the message.
                let next = tokens.get(idx + 1).map(|t| t.text);
                let (clock, consumed_next) = parse_clock(token.text, next)?;
                if !clock.strong {
                    return None;
                }
                let time = NaiveTime::from_hms_opt(clock.hour, clock.minute, 0)?;
                let end = if consumed_next {
                    tokens[idx + 1].end
                } else {
                    token.end
                };
                let after = idx + 1 + usize::from(consumed_next);
                (time, end, token.start, after, Confidence::High, None)
            } else {
                // A day with no recognizable time of day: refuse to guess.
                return None;
            }
        } else {
            return None;
        };

    let due_at = resolve_due(day.date, time, &day.roll, now)?;

    Some(TimePhrase {
        time_start,
        time_end,
        after_index: after,
        due_at,
        confidence,
        at_filler,
    })
}

/// Parse an optional day anchor at `start`.
fn parse_day(tokens: &[Token], start: usize, today: NaiveDate) -> Day {
    let none = Day {
        date: today,
        had: false,
        next: start,
        start: None,
        end: None,
        implied: None,
        roll: Roll::Day,
    };

    let Some(token) = tokens.get(start) else {
        return none;
    };

    match token.text.to_ascii_lowercase().as_str() {
        "today" => day_at(today, start + 1, token, Roll::None, None),
        "tonight" => day_at(today, start + 1, token, Roll::None, Some(Daypart::Night)),
        "tomorrow" => day_at(
            today + Duration::days(1),
            start + 1,
            token,
            Roll::None,
            None,
        ),
        "next" => match tokens.get(start + 1).and_then(|t| parse_weekday(t.text)) {
            Some(weekday) => {
                let date = next_weekday(today, weekday, true);
                Day {
                    date,
                    had: true,
                    next: start + 2,
                    start: Some(token.start),
                    end: Some(tokens[start + 1].end),
                    implied: None,
                    roll: Roll::Week,
                }
            }
            None => none,
        },
        other => match parse_weekday(other) {
            Some(weekday) => {
                let date = next_weekday(today, weekday, false);
                day_at(date, start + 1, token, Roll::Week, None)
            }
            None => none,
        },
    }
}

/// Build a single-token [`Day`].
fn day_at(
    date: NaiveDate,
    next: usize,
    token: &Token,
    roll: Roll,
    implied: Option<Daypart>,
) -> Day {
    Day {
        date,
        had: true,
        next,
        start: Some(token.start),
        end: Some(token.end),
        implied,
        roll,
    }
}

/// The soonest date on or after `today` whose weekday is `target`. With
/// `force_next`, today itself is skipped (used by `next <weekday>`).
fn next_weekday(today: NaiveDate, target: Weekday, force_next: bool) -> NaiveDate {
    let current = i64::from(today.weekday().num_days_from_monday());
    let wanted = i64::from(target.num_days_from_monday());
    let mut diff = (wanted - current).rem_euclid(7);
    if force_next && diff == 0 {
        diff = 7;
    }
    today + Duration::days(diff)
}

/// Resolve a date and time into a concrete instant, rolling forward if it is
/// already in the past.
fn resolve_due(
    date: NaiveDate,
    time: NaiveTime,
    roll: &Roll,
    now: DateTime<Local>,
) -> Option<DateTime<Local>> {
    let due = local_from(date, time, now)?;
    if due > now {
        return Some(due);
    }

    let bumped = match roll {
        Roll::Day => date + Duration::days(1),
        Roll::Week => date + Duration::days(7),
        Roll::None => return Some(due),
    };
    local_from(bumped, time, now)
}

/// Combine a date and time in the local zone, tolerating DST folds.
fn local_from(date: NaiveDate, time: NaiveTime, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let naive = date.and_time(time);
    match now.timezone().from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) | chrono::LocalResult::Ambiguous(dt, _) => Some(dt),
        chrono::LocalResult::None => None,
    }
}

fn daypart_confidence(precise: bool) -> Confidence {
    if precise {
        Confidence::High
    } else {
        Confidence::Medium
    }
}

/// A parsed clock time.
struct Clock {
    hour: u32,
    minute: u32,
    /// True when the time is unambiguous on its own (had am/pm or a colon, or is
    /// `noon`/`midnight`). Only strong clocks may appear without a leading `at`.
    strong: bool,
}

#[derive(Clone, Copy)]
enum Meridiem {
    Am,
    Pm,
}

/// Parse a clock token, optionally consuming a following `am`/`pm` token.
/// Returns the clock and whether the next token was consumed.
fn parse_clock(primary: &str, next: Option<&str>) -> Option<(Clock, bool)> {
    let lower = primary.to_ascii_lowercase();
    match lower.as_str() {
        "noon" => {
            return Some((
                Clock {
                    hour: 12,
                    minute: 0,
                    strong: true,
                },
                false,
            ));
        }
        "midnight" => {
            return Some((
                Clock {
                    hour: 0,
                    minute: 0,
                    strong: true,
                },
                false,
            ));
        }
        _ => {}
    }

    let (body, meridiem, consumed_next) = if let Some(rest) = lower.strip_suffix("am") {
        (rest.to_string(), Some(Meridiem::Am), false)
    } else if let Some(rest) = lower.strip_suffix("pm") {
        (rest.to_string(), Some(Meridiem::Pm), false)
    } else if matches!(next, Some(n) if n.eq_ignore_ascii_case("am")) {
        (lower.clone(), Some(Meridiem::Am), true)
    } else if matches!(next, Some(n) if n.eq_ignore_ascii_case("pm")) {
        (lower.clone(), Some(Meridiem::Pm), true)
    } else {
        (lower.clone(), None, false)
    };

    let (hour_str, minute_str) = match body.split_once(':') {
        Some((hour, minute)) => (hour, Some(minute)),
        None => (body.as_str(), None),
    };
    if hour_str.is_empty() {
        return None;
    }

    let mut hour: u32 = hour_str.parse().ok()?;
    let minute: u32 = match minute_str {
        Some(minute) => minute.parse().ok()?,
        None => 0,
    };
    let has_colon = minute_str.is_some();

    match meridiem {
        Some(Meridiem::Am) => {
            if hour == 12 {
                hour = 0;
            }
            if hour >= 12 {
                return None;
            }
        }
        Some(Meridiem::Pm) => {
            if hour < 12 {
                hour += 12;
            }
            if hour >= 24 {
                return None;
            }
        }
        None => {}
    }

    if hour >= 24 || minute >= 60 {
        return None;
    }

    let strong = meridiem.is_some() || has_colon;
    Some((
        Clock {
            hour,
            minute,
            strong,
        },
        consumed_next,
    ))
}

/// A rough part of day with a sensible default hour.
#[derive(Clone, Copy)]
enum Daypart {
    Morning,
    Noon,
    Afternoon,
    Evening,
    Night,
    Midnight,
}

impl Daypart {
    fn from_word(word: &str) -> Option<Daypart> {
        Some(match word {
            "morning" => Daypart::Morning,
            "noon" => Daypart::Noon,
            "afternoon" => Daypart::Afternoon,
            "evening" => Daypart::Evening,
            "night" => Daypart::Night,
            "midnight" => Daypart::Midnight,
            _ => return None,
        })
    }

    /// The default time and whether that hour is precise (rather than guessed).
    fn time(self) -> (NaiveTime, bool) {
        let (hour, precise) = match self {
            Daypart::Morning => (9, false),
            Daypart::Noon => (12, true),
            Daypart::Afternoon => (15, false),
            Daypart::Evening => (18, false),
            Daypart::Night => (21, false),
            Daypart::Midnight => (0, true),
        };
        (
            NaiveTime::from_hms_opt(hour, 0, 0).expect("valid constant hour"),
            precise,
        )
    }
}

fn parse_weekday(word: &str) -> Option<Weekday> {
    Some(match word.to_ascii_lowercase().as_str() {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" | "tues" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" | "thurs" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    })
}

/// Assemble the span list, ordered by start offset.
fn build_spans(
    tokens: &[Token],
    prefix_end: usize,
    phrase: &TimePhrase,
    connector: Option<&Token>,
    msg_start: usize,
    msg_end: usize,
) -> Vec<ParsedSpan> {
    let mut spans = Vec::new();

    if prefix_end > 0 {
        spans.push(ParsedSpan::new(
            SpanKind::Filler,
            tokens[0].start,
            tokens[prefix_end - 1].end,
        ));
    }
    if let Some((start, end)) = phrase.at_filler {
        spans.push(ParsedSpan::new(SpanKind::Filler, start, end));
    }
    spans.push(ParsedSpan::new(
        SpanKind::Time,
        phrase.time_start,
        phrase.time_end,
    ));
    if let Some(connector) = connector {
        spans.push(ParsedSpan::new(
            SpanKind::Filler,
            connector.start,
            connector.end,
        ));
    }
    spans.push(ParsedSpan::new(SpanKind::Message, msg_start, msg_end));

    spans.sort_by_key(|span| span.start);
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::spans::SpanKind;

    /// A Saturday, 12:00 local, matching the fixtures' reference time.
    fn now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    fn parse_ok(input: &str) -> ParsedReminder {
        parse(input, now()).expect("expected a successful parse")
    }

    fn time_span(reminder: &ParsedReminder) -> &str {
        reminder
            .spans
            .iter()
            .find(|span| span.kind == SpanKind::Time)
            .and_then(|span| span.text(&reminder.original))
            .expect("a time span")
    }

    #[test]
    fn at_pm_today_or_tomorrow() {
        // 5pm is still ahead of noon, so it is today.
        let r = parse_ok("at 5pm call Sam");
        assert_eq!(
            r.due_at.format("%Y-%m-%d %H:%M").to_string(),
            "2026-07-04 17:00"
        );
        assert_eq!(r.message, "call Sam");
        assert_eq!(r.confidence, Confidence::High);
        assert_eq!(time_span(&r), "5pm");
    }

    #[test]
    fn at_time_already_passed_rolls_to_tomorrow() {
        // 9am is before noon, so it rolls to tomorrow.
        let r = parse_ok("at 9am standup");
        assert_eq!(
            r.due_at.format("%Y-%m-%d %H:%M").to_string(),
            "2026-07-05 09:00"
        );
    }

    #[test]
    fn twenty_four_hour_clock() {
        let r = parse_ok("at 17:30 check oven");
        assert_eq!(
            r.due_at.format("%Y-%m-%d %H:%M").to_string(),
            "2026-07-04 17:30"
        );
        assert_eq!(r.message, "check oven");
    }

    #[test]
    fn tomorrow_at_9am() {
        let r = parse_ok("tomorrow at 9am review PR");
        assert_eq!(
            r.due_at.format("%Y-%m-%d %H:%M").to_string(),
            "2026-07-05 09:00"
        );
        assert_eq!(r.message, "review PR");
        assert_eq!(time_span(&r), "tomorrow at 9am");
    }

    #[test]
    fn spaced_meridiem() {
        let r = parse_ok("at 5 pm call Sam");
        assert_eq!(r.due_at.format("%H:%M").to_string(), "17:00");
        assert_eq!(time_span(&r), "5 pm");
    }

    #[test]
    fn tonight_uses_default_evening_hour() {
        let r = parse_ok("tonight check logs");
        assert_eq!(
            r.due_at.format("%Y-%m-%d %H:%M").to_string(),
            "2026-07-04 21:00"
        );
        assert_eq!(r.message, "check logs");
        assert_eq!(r.confidence, Confidence::Medium);
    }

    #[test]
    fn tomorrow_morning_daypart() {
        let r = parse_ok("tomorrow morning review PR");
        assert_eq!(
            r.due_at.format("%Y-%m-%d %H:%M").to_string(),
            "2026-07-05 09:00"
        );
        assert_eq!(r.confidence, Confidence::Medium);
    }

    #[test]
    fn noon_is_precise() {
        let r = parse_ok("at noon lunch");
        assert_eq!(r.due_at.format("%H:%M").to_string(), "12:00");
        assert_eq!(r.confidence, Confidence::High);
    }

    #[test]
    fn bare_hour_is_low_confidence() {
        let r = parse_ok("at 5 call Sam");
        assert_eq!(r.confidence, Confidence::Low);
    }

    #[test]
    fn next_monday_at_10am() {
        // 2026-07-04 is a Saturday; next Monday is the 6th.
        let r = parse_ok("next monday at 10am follow up");
        assert_eq!(
            r.due_at.format("%Y-%m-%d %H:%M").to_string(),
            "2026-07-06 10:00"
        );
    }

    #[test]
    fn filler_prefix_and_connector() {
        let r = parse_ok("remind me tomorrow at 9am to review PR");
        assert_eq!(r.message, "review PR");
        assert_eq!(r.confidence, Confidence::High);
    }

    #[test]
    fn missing_message_is_reported() {
        assert_eq!(parse("at 5pm", now()), Err(ParseError::MissingMessage));
    }

    #[test]
    fn unrecognized_is_missing_time() {
        assert_eq!(
            parse("check the build", now()),
            Err(ParseError::MissingTime)
        );
        // A day with no time of day should not be guessed.
        assert_eq!(
            parse("tomorrow review PR", now()),
            Err(ParseError::MissingTime)
        );
    }
}

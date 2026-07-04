//! Fixture-backed parser tests.
//!
//! The natural-language corpus in `tests/fixtures/reminders.yaml` is part of the
//! product: every phrase we support should be captured here. This harness runs
//! each fixture through [`parse_reminder`] and reports *all* mismatches at once,
//! so a single run tells you everything that regressed.
//!
//! A fixture describes either a successful parse (`expected:`) or an expected
//! failure (`error:`) — exactly one of the two.

use chrono::{DateTime, Local};
use rapid_reminder::parser::{Confidence, ParseError, ParsedReminder, SpanKind, parse_reminder};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    input: String,
    now: String,
    #[serde(default)]
    expected: Option<Expected>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    offset_seconds: i64,
    message: String,
    time_span: String,
    confidence: String,
}

#[test]
fn all_fixtures_parse_as_expected() {
    let raw = include_str!("fixtures/reminders.yaml");
    let fixtures: Vec<Fixture> = serde_yaml::from_str(raw).expect("fixtures should be valid YAML");

    assert!(
        fixtures.len() >= 50,
        "the corpus should start with at least 50 fixtures, found {}",
        fixtures.len()
    );

    let mut failures = Vec::new();
    for fixture in &fixtures {
        if let Err(reason) = check_fixture(fixture) {
            failures.push(format!("input {:?}: {reason}", fixture.input));
        }
    }

    assert!(
        failures.is_empty(),
        "\n{}\n\n{} of {} fixture(s) failed",
        failures.join("\n"),
        failures.len(),
        fixtures.len()
    );
}

/// Check one fixture, returning a human-readable reason on mismatch.
fn check_fixture(fixture: &Fixture) -> Result<(), String> {
    let now = parse_now(&fixture.now)?;
    let result = parse_reminder(&fixture.input, now);

    match (&fixture.expected, &fixture.error) {
        (Some(expected), None) => check_success(expected, now, result),
        (None, Some(error)) => check_error(error, result),
        _ => Err("fixture must have exactly one of `expected` or `error`".to_string()),
    }
}

/// Parse an RFC 3339 timestamp into local time. The wall-clock zone is
/// irrelevant to the assertions because we compare a duration, not an instant.
fn parse_now(raw: &str) -> Result<DateTime<Local>, String> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|err| format!("invalid `now` {raw:?}: {err}"))
}

fn check_success(
    expected: &Expected,
    now: DateTime<Local>,
    result: Result<ParsedReminder, ParseError>,
) -> Result<(), String> {
    let reminder = result.map_err(|err| format!("expected a parse, got error: {err}"))?;

    let offset = (reminder.due_at - now).num_seconds();
    if offset != expected.offset_seconds {
        return Err(format!(
            "offset_seconds: expected {}, got {offset}",
            expected.offset_seconds
        ));
    }

    if reminder.message != expected.message {
        return Err(format!(
            "message: expected {:?}, got {:?}",
            expected.message, reminder.message
        ));
    }

    let time_span = time_span_text(&reminder).ok_or("no time span in parse result")?;
    if time_span != expected.time_span {
        return Err(format!(
            "time_span: expected {:?}, got {:?}",
            expected.time_span, time_span
        ));
    }

    let confidence = confidence_name(reminder.confidence);
    if confidence != expected.confidence.to_ascii_lowercase() {
        return Err(format!(
            "confidence: expected {:?}, got {:?}",
            expected.confidence, confidence
        ));
    }

    Ok(())
}

fn check_error(expected: &str, result: Result<ParsedReminder, ParseError>) -> Result<(), String> {
    match result {
        Ok(reminder) => Err(format!(
            "expected error {expected:?}, but parsed message {:?}",
            reminder.message
        )),
        Err(err) => {
            let actual = error_name(&err);
            if actual == expected.to_ascii_lowercase() {
                Ok(())
            } else {
                Err(format!("error: expected {expected:?}, got {actual:?}"))
            }
        }
    }
}

/// The text of the single time span in a parsed reminder.
fn time_span_text(reminder: &ParsedReminder) -> Option<&str> {
    reminder
        .spans
        .iter()
        .find(|span| span.kind == SpanKind::Time)
        .and_then(|span| span.text(&reminder.original))
}

fn confidence_name(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "high",
        Confidence::Medium => "medium",
        Confidence::Low => "low",
    }
}

fn error_name(error: &ParseError) -> &'static str {
    match error {
        ParseError::EmptyInput => "empty_input",
        ParseError::MissingTime => "missing_time",
        ParseError::MissingMessage => "missing_message",
        ParseError::TimeOutOfRange => "time_out_of_range",
    }
}

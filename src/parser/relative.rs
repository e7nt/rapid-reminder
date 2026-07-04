//! Relative-time phrases, e.g. `in 15 mins check build`.
//!
//! The grammar this stage recognizes is:
//!
//! ```text
//! [filler] "in" <count> <unit> [connector] <message>
//! ```
//!
//! where `<count>` is a number (or `a`/`an` meaning one), `<unit>` is a word
//! like `mins` or `hours`, filler is text such as `remind me`, and a connector
//! is `to`, `about`, or `that`. The message is preserved as the user typed it.

use chrono::{DateTime, Duration, Local};

use crate::parser::spans::{ParsedSpan, SpanKind};
use crate::parser::{Confidence, ParseError, ParsedReminder};
use crate::time::TimeUnit;

/// A whitespace-delimited token with its byte range in the original input.
struct Token<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

/// The recognized `in <count> <unit>` time expression and where it sits.
struct TimeAnchor {
    /// Index of the `in` keyword token.
    in_index: usize,
    /// Index of the first token after the time expression.
    after_index: usize,
    count: u64,
    unit: TimeUnit,
    /// Byte range covering `<count> <unit>` (the `in` keyword is excluded).
    time_start: usize,
    time_end: usize,
}

/// Parse a relative-time reminder from `input` relative to `now`.
pub fn parse(input: &str, now: DateTime<Local>) -> Result<ParsedReminder, ParseError> {
    let tokens = tokenize(input);

    let anchor = find_time_anchor(&tokens).ok_or(ParseError::MissingTime)?;
    let due_at = due_time(now, &anchor)?;

    let message_tokens = &tokens[anchor.after_index.min(tokens.len())..];
    let (connector, message_tokens) = split_leading_connector(message_tokens);

    let (msg_start, msg_end) = match (message_tokens.first(), message_tokens.last()) {
        (Some(first), Some(last)) => (first.start, last.end),
        _ => return Err(ParseError::MissingMessage),
    };
    let message = input[msg_start..msg_end].to_string();

    let spans = build_spans(&tokens, &anchor, connector, msg_start, msg_end);
    let confidence = confidence_for(&tokens, &anchor);

    Ok(ParsedReminder {
        original: input.to_string(),
        due_at,
        message,
        spans,
        confidence,
    })
}

/// Split `input` into whitespace-delimited tokens, preserving byte offsets.
fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;

    for (i, ch) in input.char_indices() {
        if ch.is_whitespace() {
            if let Some(s) = start.take() {
                tokens.push(Token {
                    text: &input[s..i],
                    start: s,
                    end: i,
                });
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }

    if let Some(s) = start {
        tokens.push(Token {
            text: &input[s..],
            start: s,
            end: input.len(),
        });
    }

    tokens
}

/// Find the first `in <count> <unit>` expression in the token stream.
fn find_time_anchor(tokens: &[Token]) -> Option<TimeAnchor> {
    for (i, token) in tokens.iter().enumerate() {
        if !token.text.eq_ignore_ascii_case("in") {
            continue;
        }
        if let Some(anchor) = parse_duration_after(tokens, i) {
            return Some(anchor);
        }
    }
    None
}

/// Parse the `<count> <unit>` that should follow the `in` at `in_index`.
///
/// Handles both a glued token (`10m`) and a split pair (`15 mins`, `a minute`).
fn parse_duration_after(tokens: &[Token], in_index: usize) -> Option<TimeAnchor> {
    let first = tokens.get(in_index + 1)?;

    if let Some((count, unit)) = split_glued(first.text) {
        return Some(TimeAnchor {
            in_index,
            after_index: in_index + 2,
            count,
            unit,
            time_start: first.start,
            time_end: first.end,
        });
    }

    let count = parse_count(first.text)?;
    let unit_token = tokens.get(in_index + 2)?;
    let unit = TimeUnit::from_word(unit_token.text)?;

    Some(TimeAnchor {
        in_index,
        after_index: in_index + 3,
        count,
        unit,
        time_start: first.start,
        time_end: unit_token.end,
    })
}

/// Parse a count word: a number, or `a`/`an` meaning one.
fn parse_count(word: &str) -> Option<u64> {
    match word.to_ascii_lowercase().as_str() {
        "a" | "an" => Some(1),
        _ => word.parse::<u64>().ok(),
    }
}

/// Split a glued duration token such as `10m` or `30mins` into count and unit.
fn split_glued(word: &str) -> Option<(u64, TimeUnit)> {
    let split = word.find(|c: char| !c.is_ascii_digit())?;
    if split == 0 {
        return None;
    }

    let (digits, rest) = word.split_at(split);
    let count = digits.parse::<u64>().ok()?;
    let unit = TimeUnit::from_word(rest)?;
    Some((count, unit))
}

/// Compute the due time, guarding against overflow.
fn due_time(now: DateTime<Local>, anchor: &TimeAnchor) -> Result<DateTime<Local>, ParseError> {
    let count = i64::try_from(anchor.count).map_err(|_| ParseError::TimeOutOfRange)?;
    let seconds = anchor
        .unit
        .seconds()
        .checked_mul(count)
        .ok_or(ParseError::TimeOutOfRange)?;

    now.checked_add_signed(Duration::seconds(seconds))
        .ok_or(ParseError::TimeOutOfRange)
}

/// If the first message token is a connector (`to`, `about`, `that`), peel it
/// off so it becomes filler and is excluded from the message.
fn split_leading_connector<'a, 'b>(
    tokens: &'b [Token<'a>],
) -> (Option<&'b Token<'a>>, &'b [Token<'a>]) {
    if let Some(first) = tokens.first()
        && is_connector(first.text)
    {
        return (Some(first), &tokens[1..]);
    }
    (None, tokens)
}

fn is_connector(word: &str) -> bool {
    matches!(word.to_ascii_lowercase().as_str(), "to" | "about" | "that")
}

/// Words that count as recognizable structural filler before the time.
fn is_known_filler(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "remind" | "reminder" | "me" | "please" | "ping" | "hey"
    )
}

/// Build the span list, ordered by start offset for predictable rendering.
fn build_spans(
    tokens: &[Token],
    anchor: &TimeAnchor,
    connector: Option<&Token>,
    msg_start: usize,
    msg_end: usize,
) -> Vec<ParsedSpan> {
    let mut spans = Vec::new();

    // Filler prefix before the `in` keyword, collapsed into one span.
    if anchor.in_index > 0 {
        let first = &tokens[0];
        let last = &tokens[anchor.in_index - 1];
        spans.push(ParsedSpan::new(SpanKind::Filler, first.start, last.end));
    }

    // The `in` keyword itself is filler.
    let in_token = &tokens[anchor.in_index];
    spans.push(ParsedSpan::new(
        SpanKind::Filler,
        in_token.start,
        in_token.end,
    ));

    // The time expression.
    spans.push(ParsedSpan::new(
        SpanKind::Time,
        anchor.time_start,
        anchor.time_end,
    ));

    // A connector such as `to` or `about` is filler.
    if let Some(connector) = connector {
        spans.push(ParsedSpan::new(
            SpanKind::Filler,
            connector.start,
            connector.end,
        ));
    }

    // The message.
    spans.push(ParsedSpan::new(SpanKind::Message, msg_start, msg_end));

    spans.sort_by_key(|span| span.start);
    spans
}

/// High confidence when everything before the time is recognized filler;
/// otherwise medium, because we assumed unfamiliar leading words were filler.
fn confidence_for(tokens: &[Token], anchor: &TimeAnchor) -> Confidence {
    let prefix_all_known = tokens[..anchor.in_index]
        .iter()
        .all(|token| is_known_filler(token.text));

    if prefix_all_known {
        Confidence::High
    } else {
        Confidence::Medium
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    fn parse_ok(input: &str) -> ParsedReminder {
        parse(input, now()).expect("expected a successful parse")
    }

    fn span_text<'a>(reminder: &ParsedReminder, kind: SpanKind, input: &'a str) -> Vec<&'a str> {
        reminder
            .spans
            .iter()
            .filter(|span| span.kind == kind)
            .filter_map(|span| span.text(input))
            .collect()
    }

    #[test]
    fn parses_basic_relative_minutes() {
        let input = "in 15 mins check build";
        let parsed = parse_ok(input);

        assert_eq!(parsed.due_at, now() + Duration::seconds(900));
        assert_eq!(parsed.message, "check build");
        assert_eq!(parsed.confidence, Confidence::High);
        assert_eq!(span_text(&parsed, SpanKind::Time, input), vec!["15 mins"]);
        assert_eq!(
            span_text(&parsed, SpanKind::Message, input),
            vec!["check build"]
        );
    }

    #[test]
    fn parses_glued_duration_token() {
        let input = "in 10m check build";
        let parsed = parse_ok(input);

        assert_eq!(parsed.due_at, now() + Duration::seconds(600));
        assert_eq!(parsed.message, "check build");
        assert_eq!(span_text(&parsed, SpanKind::Time, input), vec!["10m"]);
    }

    #[test]
    fn parses_hours() {
        let parsed = parse_ok("in 2 hours stretch");
        assert_eq!(parsed.due_at, now() + Duration::hours(2));
        assert_eq!(parsed.message, "stretch");
    }

    #[test]
    fn strips_leading_filler_and_connector() {
        let input = "remind me in 1 hour to call Sam";
        let parsed = parse_ok(input);

        assert_eq!(parsed.due_at, now() + Duration::hours(1));
        assert_eq!(parsed.message, "call Sam");
        assert_eq!(parsed.confidence, Confidence::High);
        assert_eq!(span_text(&parsed, SpanKind::Time, input), vec!["1 hour"]);
        // "remind me", "in", and "to" are all filler.
        assert_eq!(
            span_text(&parsed, SpanKind::Filler, input),
            vec!["remind me", "in", "to"]
        );
    }

    #[test]
    fn strips_about_connector() {
        let parsed = parse_ok("ping me in 30 minutes about laundry");
        assert_eq!(parsed.due_at, now() + Duration::minutes(30));
        assert_eq!(parsed.message, "laundry");
        assert_eq!(parsed.confidence, Confidence::High);
    }

    #[test]
    fn accepts_a_and_an_as_one() {
        assert_eq!(
            parse_ok("in a minute stretch").due_at,
            now() + Duration::minutes(1)
        );
        assert_eq!(
            parse_ok("in an hour stretch").due_at,
            now() + Duration::hours(1)
        );
    }

    #[test]
    fn preserves_utf8_message_text() {
        let input = "in 5 mins café checké";
        let parsed = parse_ok(input);
        assert_eq!(parsed.message, "café checké");
        // Every span must slice cleanly (on char boundaries).
        for span in &parsed.spans {
            assert!(
                span.text(input).is_some(),
                "span {span:?} not on a boundary"
            );
        }
    }

    #[test]
    fn unknown_leading_words_lower_confidence() {
        let parsed = parse_ok("wibble in 5 mins check build");
        assert_eq!(parsed.confidence, Confidence::Medium);
    }

    #[test]
    fn missing_time_is_reported() {
        assert_eq!(
            parse("remind me later", now()),
            Err(ParseError::MissingTime)
        );
    }

    #[test]
    fn missing_message_is_reported() {
        assert_eq!(parse("in 15 mins", now()), Err(ParseError::MissingMessage));
        assert_eq!(
            parse("in 15 mins to", now()),
            Err(ParseError::MissingMessage)
        );
    }

    #[test]
    fn overflowing_duration_is_out_of_range() {
        let input = format!("in {} days check build", u64::MAX);
        assert_eq!(parse(&input, now()), Err(ParseError::TimeOutOfRange));
    }

    #[test]
    fn spans_are_sorted_and_in_bounds() {
        let input = "remind me in 15 mins to check build";
        let parsed = parse_ok(input);

        let mut previous = 0;
        for span in &parsed.spans {
            assert!(span.start >= previous, "spans should be sorted by start");
            assert!(span.end <= input.len(), "span end out of bounds");
            previous = span.start;
        }
    }
}

//! Deterministic natural-language reminder parser.
//!
//! Text plus a reference time go in; a [`ParsedReminder`] (or a typed
//! [`ParseError`]) comes out. Deterministic parsing comes first — any LLM
//! fallback is optional and kept separate. Span offsets are byte indices into
//! the original input and always land on UTF-8 boundaries.

pub mod absolute;
pub mod errors;
pub mod relative;
pub mod spans;

use chrono::{DateTime, Local};

pub use errors::ParseError;
pub use spans::{ParsedSpan, SpanKind};

/// How confident the parser is in a result.
///
/// Low-confidence parses may later trigger an optional LLM fallback;
/// deterministic parsing itself never depends on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// A fully parsed reminder: when it fires, the message, and which parts of the
/// original text were understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedReminder {
    /// The original input, unchanged.
    pub original: String,
    /// When the reminder should fire.
    pub due_at: DateTime<Local>,
    /// The extracted reminder message, preserving the user's wording.
    pub message: String,
    /// Spans describing the recognized time, message, and filler text.
    pub spans: Vec<ParsedSpan>,
    /// How confident the parser is in this result.
    pub confidence: Confidence,
}

/// Parse `input` into a reminder relative to `now`.
///
/// This stage handles deterministic relative-time phrases such as
/// `in 15 mins check build`. Absolute phrases arrive in a later stage. On
/// ambiguous or unsupported input it returns a typed [`ParseError`] rather than
/// guessing.
pub fn parse_reminder(input: &str, now: DateTime<Local>) -> Result<ParsedReminder, ParseError> {
    if input.trim().is_empty() {
        return Err(ParseError::EmptyInput);
    }

    relative::parse(input, now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    #[test]
    fn empty_input_is_rejected() {
        assert_eq!(parse_reminder("", now()), Err(ParseError::EmptyInput));
        assert_eq!(parse_reminder("   ", now()), Err(ParseError::EmptyInput));
    }

    #[test]
    fn delegates_relative_phrases() {
        let parsed = parse_reminder("in 15 mins check build", now()).unwrap();
        assert_eq!(parsed.message, "check build");
    }
}

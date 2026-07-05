//! Terminal rendering of parsed reminders.
//!
//! Highlighting turns parsing into a visible contract the user can trust. All
//! ANSI/color logic is isolated here and never leaks into the parser. Output
//! must also read correctly without color and must respect `NO_COLOR`.

pub mod color;

use crate::parser::{ParsedSpan, SpanKind};
pub use color::ColorMode;

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";

/// Render `input` with its spans highlighted for the terminal.
///
/// Time spans are green, message spans cyan, and filler dimmed. Text not
/// covered by any span is emitted unchanged, so with color disabled the output
/// is byte-for-byte the original input.
///
/// The renderer never trusts spans blindly: any span that is out of bounds,
/// reversed, off a UTF-8 boundary, or overlapping an already-rendered span is
/// skipped. A malformed span can therefore never panic or corrupt output.
pub fn render_highlighted(input: &str, spans: &[ParsedSpan], color: ColorMode) -> String {
    let colorize = color.colorize();

    let mut ordered: Vec<&ParsedSpan> = spans.iter().collect();
    ordered.sort_by_key(|span| span.start);

    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;

    for span in ordered {
        // Reject reversed or overlapping spans (start must not go backwards).
        if span.end < span.start || span.start < cursor {
            continue;
        }
        // Reject spans that are out of bounds or not on character boundaries.
        let Some(covered) = input.get(span.start..span.end) else {
            continue;
        };

        if let Some(gap) = input.get(cursor..span.start) {
            out.push_str(gap);
        }

        if colorize {
            out.push_str(code_for(span.kind));
            out.push_str(covered);
            out.push_str(RESET);
        } else {
            out.push_str(covered);
        }

        cursor = span.end;
    }

    if let Some(rest) = input.get(cursor..) {
        out.push_str(rest);
    }

    out
}

fn code_for(kind: SpanKind) -> &'static str {
    match kind {
        SpanKind::Time => GREEN,
        SpanKind::Message => CYAN,
        SpanKind::Filler => DIM,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_reminder;
    use chrono::Local;

    /// Spans depend only on the input text, not on `now`, so any reference time
    /// yields the same highlighting.
    fn spans_for(input: &str) -> Vec<ParsedSpan> {
        parse_reminder(input, Local::now()).unwrap().spans
    }

    #[test]
    fn without_color_output_is_unchanged() {
        let input = "remind me in 15 mins to check build";
        let plain = render_highlighted(input, &spans_for(input), ColorMode::Never);
        assert_eq!(plain, input);
    }

    #[test]
    fn with_color_wraps_each_span() {
        let input = "in 15 mins check build";
        let colored = render_highlighted(input, &spans_for(input), ColorMode::Always);
        assert!(colored.contains(GREEN), "time span should be green");
        assert!(colored.contains(CYAN), "message span should be cyan");
        assert!(colored.contains(RESET));
        // Stripping the ANSI codes must recover the original text exactly.
        assert_eq!(strip_ansi(&colored), input);
    }

    #[test]
    fn out_of_bounds_span_is_skipped_not_panicking() {
        let bad = vec![ParsedSpan::new(SpanKind::Time, 100, 200)];
        let out = render_highlighted("short", &bad, ColorMode::Always);
        assert_eq!(out, "short");
    }

    #[test]
    fn span_off_char_boundary_is_skipped() {
        // "café" is 5 bytes; 3..4 lands inside the two-byte 'é'.
        let inside = vec![ParsedSpan::new(SpanKind::Message, 3, 4)];
        let out = render_highlighted("café", &inside, ColorMode::Always);
        assert_eq!(out, "café");
    }

    #[test]
    fn overlapping_span_is_skipped() {
        let spans = vec![
            ParsedSpan::new(SpanKind::Time, 0, 5),
            ParsedSpan::new(SpanKind::Message, 2, 8), // overlaps the first
        ];
        let out = render_highlighted("abcdefgh", &spans, ColorMode::Never);
        assert_eq!(out, "abcdefgh");
    }

    #[test]
    fn snapshot_plain_highlight() {
        let input = "remind me in 15 mins to check build";
        let plain = render_highlighted(input, &spans_for(input), ColorMode::Never);
        insta::assert_snapshot!("highlighted_plain", plain);
    }

    /// Remove ANSI escape sequences so tests can compare against plain text.
    fn strip_ansi(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Skip until the terminating 'm' of the CSI sequence.
                for next in chars.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
            } else {
                out.push(ch);
            }
        }
        out
    }
}

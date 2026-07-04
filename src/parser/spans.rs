//! Byte-offset spans for recognized time, message, and filler text.

/// The kind of text a [`ParsedSpan`] covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// The recognized time expression, e.g. `15 mins`.
    Time,
    /// The reminder message, e.g. `check build`.
    Message,
    /// Structural filler, e.g. `remind me`, `in`, `to`.
    Filler,
}

/// A byte-offset span into the original input.
///
/// `start..end` are byte indices into the original string and always fall on
/// UTF-8 character boundaries, so slicing the original input with them never
/// panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedSpan {
    pub kind: SpanKind,
    pub start: usize,
    pub end: usize,
}

impl ParsedSpan {
    /// Build a span, keeping construction terse at call sites.
    pub fn new(kind: SpanKind, start: usize, end: usize) -> Self {
        Self { kind, start, end }
    }

    /// The slice of `input` this span covers.
    ///
    /// Returns `None` if the span is out of bounds or not on character
    /// boundaries, so callers never panic on a malformed span.
    pub fn text<'a>(&self, input: &'a str) -> Option<&'a str> {
        input.get(self.start..self.end)
    }
}

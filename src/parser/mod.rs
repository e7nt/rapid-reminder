//! Deterministic natural-language reminder parser.
//!
//! Text plus a reference time go in; a `ParsedReminder` (or a typed
//! `ParseError`) comes out. Deterministic parsing comes first — any LLM
//! fallback is optional and kept separate. Span offsets are byte indices into
//! the original input and always land on UTF-8 boundaries.

pub mod absolute;
pub mod errors;
pub mod relative;
pub mod spans;

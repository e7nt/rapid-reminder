//! Typed parse errors surfaced to the CLI with helpful guidance.

use thiserror::Error;

/// Why a reminder could not be parsed.
///
/// Each variant is a specific, actionable failure. The parser never guesses on
/// ambiguous input; it returns one of these instead.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// The input was empty or only whitespace.
    #[error("Input was empty.")]
    EmptyInput,

    /// No recognizable time expression was found.
    #[error("Could not find a specific reminder time.")]
    MissingTime,

    /// A time was found, but there was no message to remind about.
    #[error("Could not find a reminder message.")]
    MissingMessage,

    /// The requested time is so far out it cannot be represented.
    #[error("The requested reminder time is out of range.")]
    TimeOutOfRange,
}

impl ParseError {
    /// A one-line suggestion shown beneath the error message (see SPEC §6.5).
    ///
    /// Returns `None` when no generic example would help the user.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            ParseError::EmptyInput | ParseError::MissingTime | ParseError::MissingMessage => {
                Some("Try: rr in 15 mins check build")
            }
            ParseError::TimeOutOfRange => None,
        }
    }
}

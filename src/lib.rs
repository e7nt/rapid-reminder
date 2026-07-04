//! Rapid Reminder — the reminder engine as a reusable library.
//!
//! The `rr` binary is a thin shell over this crate. Keeping the engine in a
//! library lets the binary and the test suite (including fixture-backed
//! integration tests) drive the same code. Each module owns one concern and
//! stays independent of the others:
//!
//! - [`parser`] — turn natural-language text into a structured reminder.
//! - [`render`] — highlight what was understood for the terminal.
//! - [`storage`] — persist reminders locally.
//! - [`notify`] — deliver notifications behind a trait.
//! - [`daemon`] — fire due reminders.
//! - [`time`] — shared time helpers.
//! - [`ids`] — stable, user-visible reminder identifiers.

pub mod daemon;
pub mod ids;
pub mod notify;
pub mod parser;
pub mod render;
pub mod storage;
pub mod time;

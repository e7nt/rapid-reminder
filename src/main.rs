//! Rapid Reminder (`rr`) — binary entry point.
//!
//! The reminder engine lives in the `rapid_reminder` library crate; this binary
//! only wires the command-line interface to it. See `SPEC.md` for the full
//! specification.

mod cli;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}

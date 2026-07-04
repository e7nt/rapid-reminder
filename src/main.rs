//! Rapid Reminder (`rr`) — terminal-native natural language reminders.
//!
//! See `SPEC.md` for the full specification. This binary only wires the CLI to
//! the parser, renderer, storage, notifier, and daemon modules. Each of those
//! lives behind a clean boundary so parsing, rendering, storage, notification,
//! and scheduling stay independent.

mod cli;
mod daemon;
mod ids;
mod notify;
mod parser;
mod render;
mod storage;
mod time;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}

//! Local persistence for reminders.
//!
//! Backed by SQLite (added in a later stage) with a stable, migratable schema.
//! Reminder ids are stable and user-visible.

pub mod migrations;
pub mod sqlite;

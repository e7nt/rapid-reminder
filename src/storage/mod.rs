//! Local persistence for reminders.
//!
//! Backed by SQLite with a stable, migratable schema. Reminder ids are stable
//! and user-visible. Timestamps are stored as RFC 3339 text so the database is
//! readable and portable.

pub mod migrations;
pub mod sqlite;

use chrono::{DateTime, Local};
use thiserror::Error;

pub use sqlite::Store;

/// A stable, user-visible reminder identifier (the SQLite row id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReminderId(pub i64);

impl std::fmt::Display for ReminderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle state of a stored reminder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReminderStatus {
    /// Not yet fired.
    Pending,
    /// Successfully notified.
    Fired,
    /// Cancelled by the user before firing.
    Cancelled,
}

impl ReminderStatus {
    /// The stable string stored in the database.
    fn as_str(self) -> &'static str {
        match self {
            ReminderStatus::Pending => "pending",
            ReminderStatus::Fired => "fired",
            ReminderStatus::Cancelled => "cancelled",
        }
    }

    /// Parse a status back from its stored string.
    fn from_str(text: &str) -> Option<ReminderStatus> {
        match text {
            "pending" => Some(ReminderStatus::Pending),
            "fired" => Some(ReminderStatus::Fired),
            "cancelled" => Some(ReminderStatus::Cancelled),
            _ => None,
        }
    }
}

/// A stored reminder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    pub id: ReminderId,
    pub due_at: DateTime<Local>,
    pub message: String,
    pub created_at: DateTime<Local>,
    pub status: ReminderStatus,
}

/// Errors from the storage layer.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The underlying SQLite call failed.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// The database directory could not be created.
    #[error("could not access the database directory: {0}")]
    Io(#[from] std::io::Error),

    /// No platform data directory could be determined.
    #[error("could not determine a data directory for the reminder database")]
    NoDataDir,

    /// A stored row held a value the code cannot interpret (corruption or a
    /// database written by an incompatible version).
    #[error("stored reminder is corrupt: {0}")]
    Corrupt(String),
}

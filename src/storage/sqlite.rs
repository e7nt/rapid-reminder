//! SQLite-backed storage implementation.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use directories::ProjectDirs;
use rusqlite::{Connection, Row};

use crate::storage::migrations;
use crate::storage::{Reminder, ReminderId, ReminderStatus, StorageError};

/// A handle to the local reminder database.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open the database at the default platform data location, creating it and
    /// the containing directory if needed.
    pub fn open_default() -> Result<Store, StorageError> {
        Self::open_at(&default_db_path()?)
    }

    /// Open (or create) the database at `path`.
    pub fn open_at(path: &Path) -> Result<Store, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// Open a throwaway in-memory database, used by tests.
    pub fn open_in_memory() -> Result<Store, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Store, StorageError> {
        migrations::apply(&conn)?;
        Ok(Store { conn })
    }

    /// Insert a new pending reminder and return its assigned id.
    pub fn insert(
        &self,
        due_at: DateTime<Local>,
        message: &str,
        created_at: DateTime<Local>,
    ) -> Result<ReminderId, StorageError> {
        self.conn.execute(
            "INSERT INTO reminders (due_at, message, created_at, status)
             VALUES (?1, ?2, ?3, ?4)",
            (
                due_at.to_rfc3339(),
                message,
                created_at.to_rfc3339(),
                ReminderStatus::Pending.as_str(),
            ),
        )?;
        Ok(ReminderId(self.conn.last_insert_rowid()))
    }

    /// All pending reminders, soonest first.
    pub fn list_pending(&self) -> Result<Vec<Reminder>, StorageError> {
        let mut statement = self.conn.prepare(
            "SELECT id, due_at, message, created_at, status
             FROM reminders
             WHERE status = ?1
             ORDER BY due_at ASC, id ASC",
        )?;
        let rows = statement.query_map((ReminderStatus::Pending.as_str(),), row_to_reminder)?;

        let mut reminders = Vec::new();
        for row in rows {
            reminders.push(row?);
        }
        Ok(reminders)
    }

    /// Fetch a single reminder by id, if it exists.
    pub fn get(&self, id: ReminderId) -> Result<Option<Reminder>, StorageError> {
        let mut statement = self.conn.prepare(
            "SELECT id, due_at, message, created_at, status
             FROM reminders
             WHERE id = ?1",
        )?;
        let mut rows = statement.query_map((id.0,), row_to_reminder)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Cancel a pending reminder. Returns `true` if a pending reminder with this
    /// id existed and was cancelled.
    pub fn cancel(&self, id: ReminderId) -> Result<bool, StorageError> {
        self.set_status_from_pending(id, ReminderStatus::Cancelled)
    }

    /// Mark a pending reminder as fired. Returns `true` if a pending reminder
    /// with this id existed and was updated. Transitioning only from `pending`
    /// prevents a reminder from firing twice.
    pub fn mark_fired(&self, id: ReminderId) -> Result<bool, StorageError> {
        self.set_status_from_pending(id, ReminderStatus::Fired)
    }

    fn set_status_from_pending(
        &self,
        id: ReminderId,
        status: ReminderStatus,
    ) -> Result<bool, StorageError> {
        let changed = self.conn.execute(
            "UPDATE reminders SET status = ?1 WHERE id = ?2 AND status = ?3",
            (status.as_str(), id.0, ReminderStatus::Pending.as_str()),
        )?;
        Ok(changed > 0)
    }
}

/// Build the default database path: `<data dir>/reminders.db`.
fn default_db_path() -> Result<PathBuf, StorageError> {
    let dirs = ProjectDirs::from("", "", "rapid-reminder").ok_or(StorageError::NoDataDir)?;
    Ok(dirs.data_dir().join("reminders.db"))
}

/// Map a database row into a [`Reminder`].
fn row_to_reminder(row: &Row) -> rusqlite::Result<Reminder> {
    let id: i64 = row.get(0)?;
    let due_at: String = row.get(1)?;
    let message: String = row.get(2)?;
    let created_at: String = row.get(3)?;
    let status: String = row.get(4)?;

    Ok(Reminder {
        id: ReminderId(id),
        due_at: parse_stored_time(&due_at, 1)?,
        message,
        created_at: parse_stored_time(&created_at, 3)?,
        status: parse_stored_status(&status)?,
    })
}

/// Parse an RFC 3339 timestamp stored in the database into local time.
fn parse_stored_time(raw: &str, column: usize) -> rusqlite::Result<DateTime<Local>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                column,
                rusqlite::types::Type::Text,
                err.into(),
            )
        })
}

fn parse_stored_status(raw: &str) -> rusqlite::Result<ReminderStatus> {
    ReminderStatus::from_str(raw).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            format!("unknown reminder status {raw:?}").into(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32, minute: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 7, 4, hour, minute, 0).unwrap()
    }

    #[test]
    fn insert_then_get_round_trips() {
        let store = Store::open_in_memory().unwrap();
        let id = store.insert(at(14, 45), "check build", at(12, 0)).unwrap();

        let reminder = store.get(id).unwrap().expect("reminder should exist");
        assert_eq!(reminder.id, id);
        assert_eq!(reminder.message, "check build");
        assert_eq!(reminder.due_at, at(14, 45));
        assert_eq!(reminder.created_at, at(12, 0));
        assert_eq!(reminder.status, ReminderStatus::Pending);
    }

    #[test]
    fn get_missing_returns_none() {
        let store = Store::open_in_memory().unwrap();
        assert!(store.get(ReminderId(999)).unwrap().is_none());
    }

    #[test]
    fn list_pending_is_sorted_by_due_time() {
        let store = Store::open_in_memory().unwrap();
        store.insert(at(16, 0), "later", at(12, 0)).unwrap();
        store.insert(at(14, 0), "sooner", at(12, 0)).unwrap();

        let pending = store.list_pending().unwrap();
        let messages: Vec<&str> = pending.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["sooner", "later"]);
    }

    #[test]
    fn cancel_removes_from_pending() {
        let store = Store::open_in_memory().unwrap();
        let id = store.insert(at(14, 0), "drop me", at(12, 0)).unwrap();

        assert!(store.cancel(id).unwrap());
        assert!(store.list_pending().unwrap().is_empty());
        assert_eq!(
            store.get(id).unwrap().unwrap().status,
            ReminderStatus::Cancelled
        );

        // Cancelling again is a no-op because it is no longer pending.
        assert!(!store.cancel(id).unwrap());
    }

    #[test]
    fn mark_fired_only_once() {
        let store = Store::open_in_memory().unwrap();
        let id = store.insert(at(14, 0), "ping", at(12, 0)).unwrap();

        assert!(store.mark_fired(id).unwrap());
        assert_eq!(
            store.get(id).unwrap().unwrap().status,
            ReminderStatus::Fired
        );
        // A second attempt reports no change, preventing duplicate fires.
        assert!(!store.mark_fired(id).unwrap());
    }

    #[test]
    fn persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reminders.db");

        let id = {
            let store = Store::open_at(&path).unwrap();
            store
                .insert(at(14, 0), "survive restart", at(12, 0))
                .unwrap()
        };

        let store = Store::open_at(&path).unwrap();
        let reminder = store.get(id).unwrap().expect("reminder should persist");
        assert_eq!(reminder.message, "survive restart");
    }
}

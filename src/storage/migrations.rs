//! Forward-only schema migrations for the reminder database.
//!
//! The schema version is tracked with SQLite's `user_version` pragma. Each
//! migration bumps the version by one and is applied only when the database is
//! older than it. Migrations never rewrite existing rows destructively.

use rusqlite::Connection;

use crate::storage::StorageError;

/// The highest schema version this build knows how to produce.
const LATEST_VERSION: i64 = 1;

/// Bring `conn` up to [`LATEST_VERSION`], applying only the missing migrations.
pub fn apply(conn: &Connection) -> Result<(), StorageError> {
    let version = current_version(conn)?;

    if version >= LATEST_VERSION {
        return Ok(());
    }

    if version < 1 {
        conn.execute_batch(V1)?;
    }

    set_version(conn, LATEST_VERSION)?;
    Ok(())
}

/// The schema version currently recorded in the database (0 for a fresh file).
fn current_version(conn: &Connection) -> Result<i64, StorageError> {
    let version = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version)
}

/// Record the schema version. `user_version` does not accept bound parameters,
/// so the trusted internal constant is formatted directly.
fn set_version(conn: &Connection, version: i64) -> Result<(), StorageError> {
    conn.pragma_update(None, "user_version", version)?;
    Ok(())
}

/// Version 1: the initial reminders table.
const V1: &str = "
CREATE TABLE reminders (
    id         INTEGER PRIMARY KEY,
    due_at     TEXT NOT NULL,
    message    TEXT NOT NULL,
    created_at TEXT NOT NULL,
    status     TEXT NOT NULL
);
CREATE INDEX idx_reminders_status ON reminders(status);
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply(&conn).unwrap();
        // Applying again must not error or duplicate objects.
        apply(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), LATEST_VERSION);
    }

    #[test]
    fn creates_reminders_table() {
        let conn = Connection::open_in_memory().unwrap();
        apply(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='reminders'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}

//! Background daemon that polls pending reminders and fires them when due.
//!
//! The scheduling logic ([`run_once`]) is pure with respect to time and the
//! notifier: it takes the current time and a [`Notifier`] as arguments, so it is
//! fully testable with a fixed clock and a fake notifier. The polling loop
//! ([`run`]) layers real time and interruptible sleeping on top.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, Local};
use thiserror::Error;

use crate::notify::Notifier;
use crate::storage::{StorageError, Store};

/// The title shown on fired reminder notifications.
const NOTIFICATION_TITLE: &str = "Rapid Reminder";

/// How often the loop wakes to check for due reminders, and the granularity at
/// which it re-checks the shutdown flag while sleeping.
const SLEEP_STEP: Duration = Duration::from_millis(200);

/// Errors that stop the daemon.
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Fire every reminder due at or before `now`, exactly once.
///
/// A reminder is marked fired only after its notification succeeds, so a
/// notification failure leaves it pending to retry on the next tick. Because
/// [`Store::mark_fired`] only transitions a reminder out of `pending`, a
/// reminder can never fire twice. Returns how many reminders fired.
pub fn run_once<N: Notifier>(
    store: &Store,
    notifier: &N,
    now: DateTime<Local>,
) -> Result<usize, DaemonError> {
    let mut fired = 0;

    // `list_pending` is sorted by due time, so the first not-yet-due reminder
    // means everything after it is also in the future.
    for reminder in store.list_pending()? {
        if reminder.due_at > now {
            break;
        }

        match notifier.notify(NOTIFICATION_TITLE, &reminder.message) {
            Ok(()) => {
                if store.mark_fired(reminder.id)? {
                    fired += 1;
                }
            }
            Err(err) => {
                eprintln!(
                    "rr daemon: could not notify reminder {}: {err}; will retry",
                    reminder.id
                );
            }
        }
    }

    Ok(fired)
}

/// Run the polling loop until `shutdown` is set, checking for due reminders
/// every `poll` interval.
pub fn run<N: Notifier>(
    store: &Store,
    notifier: &N,
    poll: Duration,
    shutdown: &Arc<AtomicBool>,
) -> Result<(), DaemonError> {
    while !shutdown.load(Ordering::Relaxed) {
        run_once(store, notifier, Local::now())?;
        interruptible_sleep(poll, shutdown);
    }
    Ok(())
}

/// Sleep for up to `total`, waking early if `shutdown` is set, so the daemon
/// responds to SIGINT/SIGTERM promptly instead of after a full poll interval.
fn interruptible_sleep(total: Duration, shutdown: &AtomicBool) {
    let mut slept = Duration::ZERO;
    while slept < total && !shutdown.load(Ordering::Relaxed) {
        let nap = (total - slept).min(SLEEP_STEP);
        std::thread::sleep(nap);
        slept += nap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::FakeNotifier;
    use crate::storage::ReminderStatus;
    use chrono::TimeZone;

    fn now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    #[test]
    fn fires_due_reminders_and_marks_them() {
        let store = Store::open_in_memory().unwrap();
        let notifier = FakeNotifier::new();
        let id = store
            .insert(now() - chrono::Duration::seconds(1), "check build", now())
            .unwrap();

        let fired = run_once(&store, &notifier, now()).unwrap();

        assert_eq!(fired, 1);
        assert_eq!(notifier.count(), 1);
        assert_eq!(notifier.sent()[0].body, "check build");
        assert_eq!(
            store.get(id).unwrap().unwrap().status,
            ReminderStatus::Fired
        );
    }

    #[test]
    fn does_not_fire_future_reminders() {
        let store = Store::open_in_memory().unwrap();
        let notifier = FakeNotifier::new();
        store
            .insert(now() + chrono::Duration::hours(1), "later", now())
            .unwrap();

        let fired = run_once(&store, &notifier, now()).unwrap();

        assert_eq!(fired, 0);
        assert_eq!(notifier.count(), 0);
        assert_eq!(store.list_pending().unwrap().len(), 1);
    }

    #[test]
    fn does_not_fire_twice() {
        let store = Store::open_in_memory().unwrap();
        let notifier = FakeNotifier::new();
        store
            .insert(now() - chrono::Duration::seconds(1), "once", now())
            .unwrap();

        assert_eq!(run_once(&store, &notifier, now()).unwrap(), 1);
        assert_eq!(run_once(&store, &notifier, now()).unwrap(), 0);
        assert_eq!(notifier.count(), 1);
    }

    #[test]
    fn failed_notification_leaves_reminder_pending() {
        let store = Store::open_in_memory().unwrap();
        let notifier = FakeNotifier::failing();
        let id = store
            .insert(now() - chrono::Duration::seconds(1), "retry me", now())
            .unwrap();

        let fired = run_once(&store, &notifier, now()).unwrap();

        assert_eq!(fired, 0);
        assert_eq!(
            store.get(id).unwrap().unwrap().status,
            ReminderStatus::Pending
        );
    }

    #[test]
    fn fires_only_the_due_ones() {
        let store = Store::open_in_memory().unwrap();
        let notifier = FakeNotifier::new();
        store
            .insert(now() - chrono::Duration::minutes(5), "due a", now())
            .unwrap();
        store
            .insert(now() - chrono::Duration::minutes(1), "due b", now())
            .unwrap();
        store
            .insert(now() + chrono::Duration::minutes(1), "future", now())
            .unwrap();

        let fired = run_once(&store, &notifier, now()).unwrap();

        assert_eq!(fired, 2);
        assert_eq!(store.list_pending().unwrap().len(), 1);
    }
}

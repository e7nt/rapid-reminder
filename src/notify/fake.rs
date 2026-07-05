//! In-memory fake notifier that records calls, for tests.

use std::sync::Mutex;

use crate::notify::{Notifier, NotifyError};

/// A single recorded notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentNotification {
    pub title: String,
    pub body: String,
}

/// Records notifications instead of showing them, so tests can assert on what
/// would have been delivered without needing a desktop session.
///
/// Uses a [`Mutex`] rather than a `RefCell` so it is `Sync` and can be shared
/// with the daemon in tests.
#[derive(Debug, Default)]
pub struct FakeNotifier {
    sent: Mutex<Vec<SentNotification>>,
    always_fail: bool,
}

impl FakeNotifier {
    /// A notifier that records every notification and always succeeds.
    pub fn new() -> Self {
        Self::default()
    }

    /// A notifier that always fails, for exercising error handling.
    pub fn failing() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            always_fail: true,
        }
    }

    /// A snapshot of the notifications recorded so far.
    pub fn sent(&self) -> Vec<SentNotification> {
        self.sent.lock().expect("notifier mutex poisoned").clone()
    }

    /// How many notifications have been recorded.
    pub fn count(&self) -> usize {
        self.sent.lock().expect("notifier mutex poisoned").len()
    }
}

impl Notifier for FakeNotifier {
    fn notify(&self, title: &str, body: &str) -> Result<(), NotifyError> {
        if self.always_fail {
            return Err(NotifyError::Send(
                "fake notifier configured to fail".to_string(),
            ));
        }

        self.sent
            .lock()
            .expect("notifier mutex poisoned")
            .push(SentNotification {
                title: title.to_string(),
                body: body.to_string(),
            });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_each_notification() {
        let notifier = FakeNotifier::new();
        notifier.notify("Reminder", "check build").unwrap();
        notifier.notify("Reminder", "review PR").unwrap();

        let sent = notifier.sent();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].title, "Reminder");
        assert_eq!(sent[0].body, "check build");
        assert_eq!(sent[1].body, "review PR");
    }

    #[test]
    fn failing_notifier_records_nothing_and_errors() {
        let notifier = FakeNotifier::failing();
        assert!(notifier.notify("Reminder", "check build").is_err());
        assert_eq!(notifier.count(), 0);
    }
}

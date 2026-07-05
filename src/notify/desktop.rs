//! Native desktop notifications via `notify-rust`.

use notify_rust::Notification;

use crate::notify::{Notifier, NotifyError};

/// Sends real desktop notifications. Stateless, so it is cheap to clone and
/// share.
#[derive(Debug, Default, Clone, Copy)]
pub struct DesktopNotifier;

impl Notifier for DesktopNotifier {
    fn notify(&self, title: &str, body: &str) -> Result<(), NotifyError> {
        Notification::new()
            .summary(title)
            .body(body)
            .show()
            .map(|_handle| ())
            .map_err(|err| NotifyError::Send(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sends a real desktop notification. Ignored by default because it needs a
    /// desktop session; run manually with:
    /// `cargo test --lib notify::desktop -- --ignored --nocapture`.
    #[test]
    #[ignore = "requires a desktop notification session"]
    fn sends_a_real_notification() {
        DesktopNotifier
            .notify("Rapid Reminder", "manual notification test")
            .expect("notification should be delivered");
    }
}

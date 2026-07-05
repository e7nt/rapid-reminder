//! Notification delivery, kept behind a trait so tests can use a fake notifier
//! and never require a graphical desktop session.

pub mod desktop;
pub mod fake;

use thiserror::Error;

pub use desktop::DesktopNotifier;
pub use fake::FakeNotifier;

/// Something that can deliver a notification to the user.
///
/// Keeping this behind a trait lets the daemon fire notifications in production
/// while tests substitute a [`FakeNotifier`] that records calls instead.
pub trait Notifier {
    /// Deliver a notification with the given title and body.
    fn notify(&self, title: &str, body: &str) -> Result<(), NotifyError>;
}

/// Why a notification could not be delivered.
#[derive(Debug, Error)]
pub enum NotifyError {
    /// The underlying notification backend rejected or failed the request.
    #[error("failed to send notification: {0}")]
    Send(String),
}

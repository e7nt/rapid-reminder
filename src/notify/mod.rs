//! Notification delivery, kept behind a trait so tests can use a fake notifier
//! and never require a graphical desktop session.

pub mod desktop;
pub mod fake;

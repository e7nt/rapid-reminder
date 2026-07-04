//! Terminal rendering of parsed reminders.
//!
//! Highlighting turns parsing into a visible contract the user can trust. All
//! ANSI/color logic is isolated here and never leaks into the parser. Output
//! must also read correctly without color and must respect `NO_COLOR`.

pub mod color;

//! Command-line interface for `rr`.
//!
//! Argument parsing lives here and stays independent of the reminder parser and
//! of scheduling logic. Some subcommands are still stubs; later stages wire them
//! to real behavior.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Local};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use rapid_reminder::daemon;
use rapid_reminder::notify::{DesktopNotifier, Notifier};
use rapid_reminder::parser::{Confidence, ParsedReminder, parse_reminder};
use rapid_reminder::render::{ColorMode, render_highlighted};
use rapid_reminder::storage::{Reminder, ReminderId, Store};

/// How often the daemon checks for due reminders.
const DAEMON_POLL: Duration = Duration::from_secs(5);

/// Set reliable reminders from messy human text without breaking terminal flow.
#[derive(Debug, Parser)]
#[command(name = "rr", version, about, after_help = AFTER_HELP)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// When to colorize output.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto, global = true)]
    color: ColorArg,
}

/// Footer shown under `rr --help`. The primary action is setting a reminder from
/// free text, which has no subcommand of its own, so it is explained here.
const AFTER_HELP: &str = "\
Set a reminder by typing it naturally — no subcommand needed:
  rr in 15 mins check build
  rr remind me in 2 hours to stretch
  rr tomorrow at 9am review PR

Rapid Reminder shows what it understood and sends a desktop notification when the
reminder is due, so you can capture a thought and stay in your terminal flow.
Run `rr daemon` in the background for notifications to fire.";

#[derive(Debug, Subcommand)]
enum Command {
    /// List pending reminders.
    List,
    /// Cancel a reminder by id.
    Cancel {
        /// The user-visible reminder id shown by `rr list`.
        id: u64,
    },
    /// Run the background daemon that fires due reminders.
    Daemon,
    /// Check notification support, storage access, and daemon status.
    Doctor,
    /// Set a reminder from natural language (the default action).
    ///
    /// This catch-all captures free text such as `rr in 15 mins check build`,
    /// so a reminder can be typed without naming a subcommand.
    #[command(external_subcommand)]
    Remind(Vec<String>),
}

/// CLI-facing mirror of [`ColorMode`], kept here so the library stays free of a
/// `clap` dependency.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

impl From<ColorArg> for ColorMode {
    fn from(arg: ColorArg) -> Self {
        match arg {
            ColorArg::Auto => ColorMode::Auto,
            ColorArg::Always => ColorMode::Always,
            ColorArg::Never => ColorMode::Never,
        }
    }
}

/// Parse arguments and dispatch to the matching handler.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let color = ColorMode::from(cli.color);

    match cli.command {
        Some(Command::List) => cmd_list(),
        Some(Command::Cancel { id }) => cmd_cancel(id),
        Some(Command::Daemon) => cmd_daemon(),
        Some(Command::Doctor) => cmd_doctor(),
        Some(Command::Remind(words)) => cmd_remind(&words, color),
        None => {
            // No subcommand and no free text: show help.
            let _ = Cli::command().print_help();
            Ok(())
        }
    }
}

/// `rr daemon` — poll for due reminders and fire them until interrupted.
fn cmd_daemon() -> Result<()> {
    let store = Store::open_default()?;
    let notifier = DesktopNotifier;

    let shutdown = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&shutdown))?;

    println!(
        "rr daemon started (checking every {}s). Press Ctrl-C to stop.",
        DAEMON_POLL.as_secs()
    );
    daemon::run(&store, &notifier, DAEMON_POLL, &shutdown)?;
    println!("rr daemon stopped.");
    Ok(())
}

/// `rr list` — print the pending reminders as a table.
fn cmd_list() -> Result<()> {
    let store = Store::open_default()?;
    let pending = store.list_pending()?;
    print!("{}", format_pending_table(&pending, Local::now()));
    Ok(())
}

/// `rr cancel <id>` — cancel a pending reminder.
fn cmd_cancel(id: u64) -> Result<()> {
    let store = Store::open_default()?;

    let Ok(raw) = i64::try_from(id) else {
        eprintln!("No pending reminder with id {id}.");
        std::process::exit(1);
    };
    let reminder_id = ReminderId(raw);

    if store.cancel(reminder_id)? {
        println!("✓ Cancelled reminder {reminder_id}");
        Ok(())
    } else {
        eprintln!("No pending reminder with id {reminder_id}.");
        std::process::exit(1);
    }
}

/// Format pending reminders as an aligned table, or a friendly note if empty.
fn format_pending_table(reminders: &[Reminder], now: DateTime<Local>) -> String {
    if reminders.is_empty() {
        return String::from("No pending reminders.\n");
    }

    let rows: Vec<(String, String, &str)> = reminders
        .iter()
        .map(|reminder| {
            (
                reminder.id.to_string(),
                format_due(reminder.due_at, now),
                reminder.message.as_str(),
            )
        })
        .collect();

    let id_width = column_width("ID", rows.iter().map(|(id, _, _)| id.as_str()));
    let due_width = column_width("Due", rows.iter().map(|(_, due, _)| due.as_str()));

    let mut out = String::new();
    out.push_str(&format!(
        "{:<id_width$}  {:<due_width$}  {}\n",
        "ID", "Due", "Message"
    ));
    for (id, due, message) in &rows {
        out.push_str(&format!("{id:<id_width$}  {due:<due_width$}  {message}\n"));
    }
    out
}

/// The display width of a column: the widest of its header and values.
fn column_width<'a>(header: &str, values: impl Iterator<Item = &'a str>) -> usize {
    values
        .map(str::len)
        .chain(std::iter::once(header.len()))
        .max()
        .unwrap_or(header.len())
}

/// Render a due time relative to `now`: `Today HH:MM`, `Tomorrow HH:MM`, or an
/// abbreviated date for anything further out.
fn format_due(due: DateTime<Local>, now: DateTime<Local>) -> String {
    let days = (due.date_naive() - now.date_naive()).num_days();
    let time = due.format("%H:%M");
    match days {
        0 => format!("Today {time}"),
        1 => format!("Tomorrow {time}"),
        _ => due.format("%b %d %H:%M").to_string(),
    }
}

/// `rr <text>` — parse the free-form reminder text and store it.
fn cmd_remind(words: &[String], color: ColorMode) -> Result<()> {
    let text = words.join(" ");
    let reminder = match parse_reminder(&text, Local::now()) {
        Ok(reminder) => reminder,
        Err(error) => {
            eprintln!("{error}");
            if let Some(hint) = error.hint() {
                eprintln!("{hint}");
            }
            std::process::exit(1);
        }
    };

    let store = Store::open_default()?;
    let id = store.insert(reminder.due_at, &reminder.message, Local::now())?;
    print_set(&reminder, id, color);
    Ok(())
}

/// Confirm a stored reminder, with the recognized parts highlighted.
fn print_set(reminder: &ParsedReminder, id: ReminderId, color: ColorMode) {
    let highlighted = render_highlighted(&reminder.original, &reminder.spans, color);

    println!("✓ Reminder set (#{id})");
    println!();
    println!("Understood:");
    println!("  {highlighted}");
    println!();
    println!("Due:");
    println!("  {}", reminder.due_at.format("%a %Y-%m-%d %H:%M"));
    println!("Message:");
    println!("  {}", reminder.message);

    if reminder.confidence != Confidence::High {
        println!();
        println!("(lower confidence — double-check the time and message)");
    }
}

/// `rr doctor` — check that storage and notifications work.
///
/// Sends a real test notification so the whole delivery path is exercised.
/// Exits non-zero if any check fails, so it is usable in scripts.
fn cmd_doctor() -> Result<()> {
    println!("rr doctor");
    println!();

    let mut healthy = true;

    match Store::open_default() {
        Ok(store) => {
            let pending = store.list_pending().map(|list| list.len());
            let path = rapid_reminder::storage::default_db_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "<unknown>".to_string());
            match pending {
                Ok(count) => println!("  [ok]   storage: {path} ({count} pending)"),
                Err(err) => {
                    healthy = false;
                    println!("  [FAIL] storage: opened {path} but could not read reminders: {err}");
                }
            }
        }
        Err(err) => {
            healthy = false;
            println!("  [FAIL] storage: {err}");
        }
    }

    match DesktopNotifier.notify("Rapid Reminder", "notifications are working") {
        Ok(()) => println!("  [ok]   notifications: sent a test notification"),
        Err(err) => {
            healthy = false;
            println!("  [FAIL] notifications: {err}");
        }
    }

    // The daemon has no persisted status yet; point the user at how to run it.
    println!("  [--]   daemon: status is not tracked; start it with `rr daemon`");

    println!();
    if healthy {
        println!("All checks passed.");
        Ok(())
    } else {
        eprintln!("Some checks failed.");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rapid_reminder::storage::ReminderStatus;

    fn at(day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 7, day, hour, minute, 0)
            .unwrap()
    }

    fn reminder(id: i64, due: DateTime<Local>, message: &str) -> Reminder {
        Reminder {
            id: ReminderId(id),
            due_at: due,
            message: message.to_string(),
            created_at: at(4, 12, 0),
            status: ReminderStatus::Pending,
        }
    }

    #[test]
    fn empty_table_is_a_friendly_note() {
        assert_eq!(
            format_pending_table(&[], at(4, 12, 0)),
            "No pending reminders.\n"
        );
    }

    #[test]
    fn due_time_is_humanized() {
        let now = at(4, 12, 0);
        assert_eq!(format_due(at(4, 14, 45), now), "Today 14:45");
        assert_eq!(format_due(at(5, 9, 0), now), "Tomorrow 09:00");
        assert_eq!(format_due(at(7, 9, 0), now), "Jul 07 09:00");
    }

    #[test]
    fn table_snapshot() {
        let now = at(4, 12, 0);
        let reminders = vec![
            reminder(1, at(4, 14, 45), "check build"),
            reminder(2, at(5, 9, 0), "review PR"),
        ];
        insta::assert_snapshot!("pending_table", format_pending_table(&reminders, now));
    }
}

//! Command-line interface for `rr`.
//!
//! Argument parsing lives here and stays independent of the reminder parser and
//! of scheduling logic. In this stage the subcommands are stubs; later stages
//! wire them to real behavior.

use anyhow::Result;
use chrono::Local;
use clap::{CommandFactory, Parser, Subcommand};
use rapid_reminder::parser::{ParsedReminder, parse_reminder};

/// Set reliable reminders from messy human text without breaking terminal flow.
#[derive(Debug, Parser)]
#[command(name = "rr", version, about, args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Natural-language reminder text, e.g. `rr in 15 mins check build`.
    #[arg(trailing_var_arg = true)]
    words: Vec<String>,
}

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
}

/// Parse arguments and dispatch to the matching handler.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::List) => not_yet("list"),
        Some(Command::Cancel { .. }) => not_yet("cancel"),
        Some(Command::Daemon) => not_yet("daemon"),
        Some(Command::Doctor) => not_yet("doctor"),
        None => run_default(&cli.words),
    }
}

/// Handle `rr` with no subcommand: either free-form reminder text or bare help.
fn run_default(words: &[String]) -> Result<()> {
    if words.is_empty() {
        // A closed pipe (e.g. `rr | head`) is not an error worth surfacing, so
        // the write result is deliberately ignored here.
        let _ = Cli::command().print_help();
        return Ok(());
    }

    let text = words.join(" ");
    match parse_reminder(&text, Local::now()) {
        Ok(reminder) => {
            print_understood(&reminder);
            Ok(())
        }
        Err(error) => {
            eprintln!("{error}");
            if let Some(hint) = error.hint() {
                eprintln!("{hint}");
            }
            std::process::exit(1);
        }
    }
}

/// Show what the parser understood. Storage and colored highlighting arrive in
/// later stages; for now this is an honest plain-text preview.
fn print_understood(reminder: &ParsedReminder) {
    println!("Understood:");
    println!("  message:    {}", reminder.message);
    println!(
        "  due:        {}",
        reminder.due_at.format("%a %Y-%m-%d %H:%M")
    );
    println!("  confidence: {:?}", reminder.confidence);
}

/// Placeholder for subcommands that later stages implement.
fn not_yet(command: &str) -> Result<()> {
    println!("rr: `{command}` is not implemented yet (Stage 0 skeleton).");
    Ok(())
}

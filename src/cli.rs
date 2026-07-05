//! Command-line interface for `rr`.
//!
//! Argument parsing lives here and stays independent of the reminder parser and
//! of scheduling logic. Some subcommands are still stubs; later stages wire them
//! to real behavior.

use anyhow::Result;
use chrono::Local;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use rapid_reminder::parser::{Confidence, ParsedReminder, parse_reminder};
use rapid_reminder::render::{ColorMode, render_highlighted};

/// Set reliable reminders from messy human text without breaking terminal flow.
#[derive(Debug, Parser)]
#[command(name = "rr", version, about, args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// When to colorize output.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto, global = true)]
    color: ColorArg,

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
        Some(Command::List) => not_yet("list"),
        Some(Command::Cancel { .. }) => not_yet("cancel"),
        Some(Command::Daemon) => not_yet("daemon"),
        Some(Command::Doctor) => not_yet("doctor"),
        None => run_default(&cli.words, color),
    }
}

/// Handle `rr` with no subcommand: either free-form reminder text or bare help.
fn run_default(words: &[String], color: ColorMode) -> Result<()> {
    if words.is_empty() {
        // A closed pipe (e.g. `rr | head`) is not an error worth surfacing, so
        // the write result is deliberately ignored here.
        let _ = Cli::command().print_help();
        return Ok(());
    }

    let text = words.join(" ");
    match parse_reminder(&text, Local::now()) {
        Ok(reminder) => {
            print_understood(&reminder, color);
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

/// Show what the parser understood, with the recognized parts highlighted.
///
/// Storage is not wired yet, so this deliberately says "Understood" rather than
/// "Reminder set" — that promise is made once persistence lands.
fn print_understood(reminder: &ParsedReminder, color: ColorMode) {
    let highlighted = render_highlighted(&reminder.original, &reminder.spans, color);

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

/// Placeholder for subcommands that later stages implement.
fn not_yet(command: &str) -> Result<()> {
    println!("rr: `{command}` is not implemented yet.");
    Ok(())
}

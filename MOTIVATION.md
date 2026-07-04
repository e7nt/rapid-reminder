# MOTIVATION.md

## Why this exists

Rapid Reminder exists because context switching is expensive.

People who work in terminals often need to temporarily leave one task while waiting on something else:

- A build is running.
- Tests are executing.
- A deployment is rolling out.
- A script is processing data.
- A download is finishing.
- A teammate needs a follow-up later.
- A short break should not become a lost thread.

In these moments, the user does not want to open a calendar app, create a task, choose a time from a picker, or switch to a GUI workflow. They want to stay in flow and type something natural:

```bash
rr in 15 mins check the build
rr remind me at 5pm to follow up on staging
rr tomorrow morning review the PR
```

The reminder should be created immediately, and the user should trust that the system understood the important details.

## The problem we are solving

The core problem is not simply “setting reminders.” Existing tools already do that.

The problem is:

> When users are deep in terminal work and need to switch context, setting a reminder should be almost frictionless.

Current options are often too slow or awkward:

- Calendar apps require leaving the terminal.
- Todo apps often capture work but do not reliably notify at the right time.
- `cron` and `at` are powerful but not natural for quick human reminders.
- Shell hacks like `sleep 900 && notify-send ...` work but are clunky.
- Reminder apps are usually GUI-first and interrupt flow.

Rapid Reminder aims to make this interaction feel instant, local, and trustworthy.

## Why terminal-first matters

For developers, operators, researchers, and power users, the terminal is not just a place to run commands. It is the workspace.

A terminal-first reminder tool respects that workflow:

- No app switching.
- No mouse interaction.
- No account required.
- No cloud dependency by default.
- No heavy interface for a lightweight thought.

The fastest reminder is the one that can be created exactly where the thought occurs.

## Why natural language matters

Users should not need to remember rigid syntax for common reminders.

Instead of forcing:

```bash
rr --after 15m --message "check build"
```

Rapid Reminder should accept:

```bash
rr remind me in 15 mins to check build
```

Natural language input reduces friction. It lets users express intent quickly, even with filler words or slightly messy phrasing.

The parser should be deterministic first, with optional LLM fallback only when needed. Reliability matters more than novelty.

## Why highlighting matters

Natural language tools can feel uncertain. Users often wonder:

> Did it understand me correctly?

Rapid Reminder should answer that immediately by highlighting what it parsed:

```text
Understood:
  remind me in 15 mins to check build
             └─ time ─┘    └ message ┘
```

This makes the tool transparent. The user can see the time expression and reminder message before trusting the reminder.

Highlighting turns parsing from a black box into a visible contract.

## What success looks like

The project succeeds if users can reliably type a quick reminder in the terminal and move on with confidence.

A successful experience feels like this:

1. The user has a thought they do not want to lose.
2. They type one short command.
3. The tool shows what it understood.
4. The reminder is stored locally.
5. The notification arrives at the right time.
6. The user returns to the original task without mental overhead.

## What we are not trying to build

Rapid Reminder is not trying to become a full productivity suite.

It is not primarily:

- A calendar replacement.
- A project management tool.
- A cloud reminders platform.
- A general AI assistant.
- A complex task database.

The first version should stay focused on doing one thing well:

> Quickly set reliable reminders from the terminal using natural language.

## Open-source philosophy

This project should be useful, understandable, and welcoming to contributors.

The codebase should be:

- Readable.
- Well-tested.
- Modular.
- Safe to change.
- Easy to extend with new phrase examples.

The natural language fixture corpus is part of the product. Every new phrase we support should make the tool more useful for everyone.

## Guiding principle

When deciding whether a feature belongs, ask:

> Does this make it faster, clearer, or more reliable for someone to set a reminder without leaving the terminal?

If yes, it may belong.

If no, it should probably wait.

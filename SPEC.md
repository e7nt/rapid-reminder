# Rapid Reminder — Project Specification

## 1. Summary

Rapid Reminder is a terminal-native natural language reminder tool written in Rust.

It lets users quickly set reminders without leaving the terminal:

```bash
rr remind me in 15 mins to check the build
rr tomorrow at 9am review the PR
rr in 2h switch laundry
```

The tool parses the user's text, highlights the important parts it understood, stores the reminder locally, and sends a native desktop notification when the reminder is due.

The core product promise is:

> Set reliable reminders from messy human text without breaking terminal flow.

---

## 2. Goals

### Product goals

- Provide a fast terminal-only reminder experience.
- Accept natural language reminder text.
- Highlight parsed time and message spans so users can verify interpretation.
- Be reliable enough for daily use.
- Work locally by default.
- Be easy to install, test, maintain, and contribute to.

### Engineering goals

- Rust-first implementation.
- Clear, readable, well-tested code.
- Deterministic parser before any LLM fallback.
- Large fixture-backed natural language test suite.
- Pre-commit checks to prevent low-quality changes.
- Modular architecture so parsing, storage, notification, and CLI code remain separate.

---

## 3. Non-goals for v1

- Full calendar replacement.
- Cloud sync.
- User accounts.
- Team collaboration.
- Complex recurring reminders.
- GUI application.
- Mandatory LLM dependency.

LLM support may be added later as an optional fallback, but the first versions should be useful without it.

---

## 4. Primary use cases

### 4.1 Developer waiting on a command

```bash
rr in 10 mins check build
```

The user is waiting on a build, deployment, test suite, or script and wants to move to another task temporarily.

### 4.2 Quick follow-up reminder

```bash
rr remind me at 5pm to follow up on the staging bug
```

The user wants a low-friction reminder without opening a calendar or GUI app.

### 4.3 Messy terminal input

```bash
rr remindr me in 15 mins to chek docker logs
```

The parser should tolerate common filler words and minor command phrasing differences. It does not need to fix spelling in the reminder message.

---

## 5. CLI interface

The binary name is tentatively `rr`.

### 5.1 Set reminder

```bash
rr <natural language text>
```

Examples:

```bash
rr in 15 mins check build
rr remind me in 2 hours to stretch
rr ping me tomorrow at 9am about reviewing the PR
```

Expected output:

```text
✓ Reminder set

Understood:
  remind me in 15 mins to check build
             └─ time ─┘    └ message ┘

Due:
  Today at 14:45

Message:
  check build
```

In color-capable terminals:

- Time span: green
- Message span: cyan
- Filler text: dimmed, optional
- Warnings/ambiguity: yellow
- Errors: red

### 5.2 List reminders

```bash
rr list
```

Expected output:

```text
ID  Due                 Message
1   Today 14:45         check build
2   Tomorrow 09:00      review PR
```

### 5.3 Cancel reminder

```bash
rr cancel <id>
```

Expected output:

```text
✓ Cancelled reminder 2
```

### 5.4 Run daemon

```bash
rr daemon
```

The daemon checks stored reminders and sends notifications when they are due.

Implementation may later support systemd, launchd, or other OS-native scheduling.

### 5.5 Version and diagnostics

```bash
rr --version
rr doctor
```

`rr doctor` should check notification support, storage accessibility, and daemon status where possible.

---

## 6. Parsing requirements

### 6.1 Parser contract

The parser accepts raw text and a reference time.

It returns either a parsed reminder or a clear parse error.

```rust
pub fn parse_reminder(input: &str, now: DateTime<Local>) -> Result<ParsedReminder, ParseError>
```

### 6.2 Parsed reminder model

```rust
pub struct ParsedReminder {
    pub original: String,
    pub due_at: DateTime<Local>,
    pub message: String,
    pub spans: Vec<ParsedSpan>,
    pub confidence: Confidence,
}

pub struct ParsedSpan {
    pub kind: SpanKind,
    pub start: usize,
    pub end: usize,
}

pub enum SpanKind {
    Time,
    Message,
    Filler,
}

pub enum Confidence {
    High,
    Medium,
    Low,
}
```

Span indexes must be byte offsets into `original` and must always fall on UTF-8 character boundaries.

### 6.3 v1 supported input patterns

Relative time:

```text
in 15 mins check build
in 10m check build
in 2 hours stretch
remind me in 1 hour to call Sam
ping me in 30 minutes about laundry
```

Absolute time:

```text
at 5pm call Sam
at 17:30 check oven
tomorrow at 9am review PR
```

Simple day phrases:

```text
tomorrow morning review PR
tonight check logs
next monday at 10am follow up
```

### 6.4 Parser behavior

- Prefer deterministic parsing over LLM calls.
- Return clear errors for unsupported input.
- Do not silently guess when the time is ambiguous.
- Preserve the user's message text as much as possible.
- Extract filler words only when they are clearly structural, such as `remind me`, `to`, `about`, or `please`.
- Keep parser functions small and focused.

### 6.5 Error examples

Input:

```text
remind me later
```

Output:

```text
Could not find a specific reminder time.
Try: rr in 15 mins check build
```

Input:

```text
in 15 mins
```

Output:

```text
Could not find a reminder message.
Try: rr in 15 mins check build
```

---

## 7. Highlighting requirements

The renderer receives the original input and parsed spans.

```rust
pub fn render_highlighted(input: &str, spans: &[ParsedSpan], color: ColorMode) -> String
```

Requirements:

- Never mutate parser output.
- Do not panic on invalid spans; return a render error or skip invalid spans safely.
- Support color modes:
  - `auto`
  - `always`
  - `never`
- Respect `NO_COLOR`.
- Keep ANSI/color logic isolated from parser logic.
- Snapshot-test rendered output with color disabled.

---

## 8. Storage requirements

Initial storage should be local and simple.

Recommended path:

```text
~/.local/share/rapid-reminder/reminders.db
```

On non-Linux platforms, follow platform-specific data directory conventions.

Recommended implementation:

- SQLite via `rusqlite`.
- Stable schema migrations.
- Reminder IDs are stable and user-visible.

Reminder fields:

```rust
pub struct Reminder {
    pub id: ReminderId,
    pub due_at: DateTime<Local>,
    pub message: String,
    pub created_at: DateTime<Local>,
    pub status: ReminderStatus,
}

pub enum ReminderStatus {
    Pending,
    Fired,
    Cancelled,
}
```

---

## 9. Notification requirements

v1 should use native desktop notifications where possible.

Recommended crate:

```toml
notify-rust = "4"
```

Requirements:

- Notification sending must be abstracted behind a trait.
- Tests should use a fake notifier.
- Failed notifications must be logged and surfaced by `rr doctor` where possible.

```rust
pub trait Notifier {
    fn notify(&self, title: &str, body: &str) -> Result<(), NotifyError>;
}
```

---

## 10. Daemon requirements

The daemon is responsible for firing due reminders.

Requirements:

- Poll pending reminders at a reasonable interval.
- Mark reminders as fired only after successful notification, or record failure explicitly.
- Avoid duplicate notifications.
- Shut down cleanly on SIGINT/SIGTERM.
- Keep scheduling code independent from CLI parsing.

---

## 11. Test strategy

Testing is a first-class requirement.

### 11.1 Unit tests

Unit-test small parsing functions, duration conversion, span extraction, renderer behavior, storage functions, and notification abstractions.

### 11.2 Fixture-backed parser tests

Natural language examples should live in:

```text
tests/fixtures/reminders.yaml
```

Example:

```yaml
- input: "remind me in 15 mins to check build"
  now: "2026-07-04T12:00:00+05:30"
  expected:
    offset_seconds: 900
    message: "check build"
    time_span: "15 mins"
    confidence: "high"
```

The project should start with 50–100 examples and grow toward 1,000+.

### 11.3 Snapshot tests

Use snapshot tests for CLI and renderer output.

Recommended crate:

```toml
insta = "1"
```

### 11.4 Property tests

Use property tests for generated relative-time inputs.

Recommended crate:

```toml
proptest = "1"
```

---

## 12. Code quality standards

Code must be written for maintainers and contributors, not just for the compiler.

### 12.1 Function style

- Use clear function names.
- Keep functions short.
- Prefer early guards over deeply nested conditionals.
- Avoid cleverness when simple code is clearer.
- Return typed errors instead of stringly-typed failures.
- Keep side effects explicit.

Good style:

```rust
fn parse_non_empty_input(input: &str) -> Result<&str, ParseError> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    Ok(trimmed)
}
```

Avoid:

```rust
fn parse(input: &str) -> Option<_> {
    if !input.trim().is_empty() {
        // many nested branches
    } else {
        None
    }
}
```

### 12.2 Module boundaries

Suggested layout:

```text
src/
  main.rs
  cli.rs
  parser/
    mod.rs
    relative.rs
    absolute.rs
    spans.rs
    errors.rs
  render/
    mod.rs
    color.rs
  storage/
    mod.rs
    sqlite.rs
    migrations.rs
  notify/
    mod.rs
    desktop.rs
    fake.rs
  daemon.rs
  time.rs
  ids.rs
```

### 12.3 Error handling

Recommended crates:

```toml
thiserror = "1"
anyhow = "1"
```

Use `thiserror` for library/domain errors and `anyhow` at the CLI boundary.

### 12.4 Documentation

- Public functions need doc comments when their behavior is not obvious.
- Parser decisions should be documented with examples.
- New fixture categories should include comments explaining the category.

---

## 13. Pre-commit and CI requirements

The repository must include pre-commit hooks.

Required local checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo deny check
```

Recommended extra checks:

```bash
cargo audit
cargo machete
```

CI should run the same checks on pull requests.

No pull request should be merged unless formatting, linting, tests, and dependency checks pass.

---

## 14. Contribution model

Contributions should be easy and safe.

Good first contributions:

- Add natural language parser fixtures.
- Improve parser coverage for a specific phrase category.
- Add notification support for a platform.
- Improve docs and examples.
- Add tests for edge cases.

Contributor expectations:

- Add tests for behavior changes.
- Keep functions readable.
- Avoid large unrelated refactors.
- Explain parser behavior changes in PR descriptions.

---

## 15. Future ideas

- Optional LLM fallback for low-confidence parses.
- Local model support.
- Shell completions.
- Snooze command.
- Recurring reminders.
- Command completion reminders, e.g. notify when a long command finishes.
- Integrations with `systemd`, `launchd`, or Windows Task Scheduler.
- Import/export reminders.

---

## 16. Recommended initial milestones

### Milestone 1: MVP parser and CLI

- `rr <text>` parses relative reminders.
- Highlight understood time/message spans.
- Store reminder in SQLite.
- Add fixture tests.

### Milestone 2: Notifications and daemon

- `rr daemon` fires pending reminders.
- Native desktop notifications.
- `rr list` and `rr cancel`.

### Milestone 3: Hardening

- 100+ parser fixtures.
- Pre-commit and CI.
- `rr doctor`.
- Cross-platform notification validation.

### Milestone 4: Open-source polish

- README with demos.
- CONTRIBUTING guide.
- Installation instructions.
- Release workflow.

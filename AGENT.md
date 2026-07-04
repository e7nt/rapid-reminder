# AGENT.md

This file gives coding agents project-specific instructions for Rapid Reminder.

## Project intent

Rapid Reminder is a Rust CLI for setting reliable natural language reminders from the terminal.

The codebase should feel like it was written by a careful open-source maintainer: readable, tested, boring where possible, and easy for new contributors to understand.

## Read first

Before making substantial changes, read:

1. `SPEC.md`
2. `CONTRIBUTING.md` if present
3. Existing parser fixtures under `tests/fixtures/` if present

## Engineering principles

- Optimize for clarity over cleverness.
- Prefer deterministic parsing before LLM-based interpretation.
- Keep parser, rendering, storage, notification, and daemon code separate.
- Use typed errors.
- Use early guards to avoid deeply nested control flow.
- Make invalid states hard to represent.
- Keep functions small and named after what they do.
- Add or update tests for every behavior change.
- Do not introduce global mutable state unless there is a strong reason.

## Rust style

Use idiomatic, maintainable Rust.

Required before handing off code:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

If dependency tooling is installed, also run:

```bash
cargo deny check
cargo audit
cargo machete
```

## Function style

Prefer early returns:

```rust
fn require_message(message: &str) -> Result<&str, ParseError> {
    let message = message.trim();

    if message.is_empty() {
        return Err(ParseError::MissingMessage);
    }

    Ok(message)
}
```

Avoid large nested functions and multi-purpose helpers.

## Parser changes

When changing parser behavior:

1. Add fixture tests first or in the same change.
2. Preserve span correctness.
3. Do not silently guess ambiguous times.
4. Keep the original reminder message as close to user input as possible.
5. Explain any new phrase category in tests or comments.

Parser output must include spans for important recognized text so the CLI can highlight what was understood.

## Highlighting changes

Rendering must not affect parsing.

- Keep ANSI/color logic isolated.
- Respect `NO_COLOR`.
- Support non-colored snapshot tests.
- Do not panic on malformed spans.

## Storage changes

- Use migrations for schema changes.
- Do not break existing reminder databases without an explicit migration.
- Tests should use temporary databases.

## Notification changes

- Keep notification behavior behind a trait.
- Unit tests should use a fake notifier.
- Do not require a graphical desktop session for tests.

## Dependency policy

Before adding a dependency, consider:

- Is it actively maintained?
- Is the license compatible with open source distribution?
- Is the dependency necessary?
- Can this be implemented clearly without it?

Prefer small, well-maintained crates.

## Pull request expectations

A good change should include:

- Focused implementation.
- Tests or fixture updates.
- Clear error handling.
- No unrelated formatting churn.
- Updated docs when user behavior changes.

## Do not do

- Do not add mandatory cloud or account-based features.
- Do not make LLM calls required for core parsing.
- Do not mix parser logic with terminal rendering.
- Do not ignore clippy warnings.
- Do not add broad refactors while fixing small bugs.

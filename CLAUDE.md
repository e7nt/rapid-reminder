# CLAUDE.md

## Role

You are helping maintain Rapid Reminder, a Rust terminal application for natural language reminders.

Your job is to make changes that are easy to read, easy to test, and easy for future contributors to modify.

## Core product behavior

Users should be able to type commands like:

```bash
rr remind me in 15 mins to check the build
rr tomorrow at 9am review PR
rr in 2h move laundry
```

The app should:

1. Parse the reminder time.
2. Extract the reminder message.
3. Highlight the text it understood.
4. Store the reminder locally.
5. Notify the user when due.

## Coding rules

- Write Rust code like a proud open-source maintainer.
- Prefer boring, explicit, readable code.
- Use early guards instead of deep nesting.
- Keep functions focused.
- Use typed errors.
- Add tests for behavior changes.
- Preserve clean module boundaries.

## Required checks

Run these before finalizing code changes:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

If available, also run:

```bash
pre-commit run --all-files
```

## Parser guidance

The parser is the heart of the project.

When working on parsing:

- Add fixture examples.
- Keep span offsets correct.
- Avoid guessing ambiguous input.
- Return helpful parse errors.
- Keep LLM fallback optional and separate from deterministic parsing.

## Terminal output guidance

Highlighted output exists to build user trust.

- Time spans should be visually distinct.
- Message spans should be visually distinct.
- Output must also work without color.
- Respect `NO_COLOR`.

## Contribution mindset

Assume future contributors will read your code to learn how the project works. Leave the codebase simpler than you found it.

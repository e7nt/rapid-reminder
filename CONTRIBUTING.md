# Contributing to Rapid Reminder

Thanks for helping make terminal reminders better! Contributions are meant to be
easy and safe. This project values readable, well-tested, boring-where-possible
code that new contributors can learn from.

## Getting started

You need a [Rust toolchain](https://rustup.rs) (stable).

```console
$ git clone <your fork>
$ cd rapid-reminder
$ cargo test --all-features
```

Before opening a pull request, run the same checks CI runs:

```console
$ cargo fmt --all
$ cargo clippy --all-targets --all-features -- -D warnings
$ cargo test --all-features
```

If you have them installed, `cargo deny check`, `cargo audit`, and
`cargo machete` are run by the optional pre-commit hooks
(`pre-commit run --all-files`).

## Great first contributions

- **Add natural-language fixtures.** The corpus is part of the product.
- Improve parser coverage for a specific phrase category.
- Add notification support or testing for another platform.
- Improve docs and examples.

## Adding parser fixtures

Fixtures live in [`tests/fixtures/reminders.yaml`](./tests/fixtures/reminders.yaml)
and run through the real parser. Each entry is either a success case or an
expected error.

**Relative phrases** use `now` (an RFC 3339 instant) and assert `offset_seconds`,
which is independent of the machine's timezone:

```yaml
- input: "remind me in 15 mins to check build"
  now: "2026-07-04T12:00:00+05:30"
  expected:
    offset_seconds: 900
    message: "check build"
    time_span: "15 mins"
    confidence: "high"
```

**Absolute phrases** use `now_local` (a `YYYY-MM-DD HH:MM` local wall clock) and
assert `due_local`, so times like `5pm` resolve deterministically:

```yaml
- input: "tomorrow at 9am review PR"
  now_local: "2026-07-04 08:00"
  expected:
    due_local: "2026-07-05 09:00"
    message: "review PR"
    time_span: "tomorrow at 9am"
    confidence: "high"
```

**Error cases** name the expected `ParseError` in snake_case:

```yaml
- input: "remind me later"
  now: "2026-07-04T12:00:00+05:30"
  error: "missing_time"
```

- `time_span` is the exact substring the parser marks as the time, preserving
  the user's original casing.
- `confidence` is `high`, `medium`, or `low`.
- Group new phrases under a commented category so their intent is clear.

Run just the corpus with:

```console
$ cargo test --test parser_fixtures
```

## Code guidelines

The full guidance lives in [`AGENT.md`](./AGENT.md) and [`CLAUDE.md`](./CLAUDE.md).
In short:

- Optimize for clarity over cleverness; prefer early guards to deep nesting.
- Use typed errors; keep functions small and named after what they do.
- Keep parser / render / storage / notify / daemon boundaries clean — never mix
  rendering into parsing.
- Parsing stays deterministic; any LLM support must remain optional.
- Every behavior change ships with a test or fixture.

## Pull request expectations

- Focused change with tests or fixture updates.
- No unrelated formatting churn or broad refactors bundled with small fixes.
- Explain any parser behavior change in the PR description.

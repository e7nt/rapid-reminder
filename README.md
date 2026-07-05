# Rapid Reminder (`rr`)

> Set reliable reminders from messy human text without breaking terminal flow.

Rapid Reminder is a terminal-native reminder tool written in Rust. Type a
reminder in plain language, see exactly what it understood, and get a native
desktop notification when it is due — all without leaving the terminal.

```console
$ rr remind me in 15 mins to check build
✓ Reminder set (#1)

Understood:
  remind me in 15 mins to check build

Due:
  Sun 2026-07-05 14:54
Message:
  check build
```

In a color-capable terminal the time is green, the message is cyan, and filler
(`remind me`, `in`, `to`) is dimmed — so you can trust what was parsed at a
glance.

## Why

When you are deep in terminal work — a build running, a deploy rolling out, a
follow-up you can't lose — opening a calendar app breaks your flow. `rr` makes
capturing a reminder as fast as the thought, then reliably notifies you at the
right time. It works locally, needs no account, and has no cloud dependency.

## Install

From source (requires a [Rust toolchain](https://rustup.rs)):

```console
$ cargo install --path .
```

This installs the `rr` binary into `~/.cargo/bin`. SQLite is bundled, so there
are no system library dependencies.

## Usage

Set a reminder using natural language:

```console
$ rr in 15 mins check build
$ rr remind me in 2 hours to stretch
$ rr tomorrow at 9am review PR
$ rr tonight check logs
$ rr next monday at 10am follow up
```

List, cancel, and run the notifier daemon:

```console
$ rr list
ID  Due             Message
1   Today 14:45     check build
2   Tomorrow 09:00  review PR

$ rr cancel 2
✓ Cancelled reminder 2

$ rr daemon        # fires due reminders until you stop it (Ctrl-C)
$ rr doctor        # check storage + notifications work
```

Control color with `--color auto|always|never` (or set `NO_COLOR`):

```console
$ rr --color never in 15 mins check build
```

### Supported time phrases

- **Relative:** `in 15 mins`, `in 10m`, `in 2 hours`, `in 1h`, `in 30 seconds`,
  `in 2 days`, `in a minute`, `in an hour`.
- **Absolute:** `at 5pm`, `at 5 pm`, `at 17:30`, `at 5:30pm`, `at noon`,
  `at midnight`.
- **Day + time:** `today at 5pm`, `tomorrow at 9am`, `monday at 9am`,
  `next monday at 10am`.
- **Dayparts:** `tonight`, `tomorrow morning`, `tomorrow evening`,
  `sunday morning`.

Filler such as `remind me`, `ping me`, `please`, and connectors like `to`,
`about`, `that` are recognized and stripped from the message.

## How it works

- **Deterministic first.** Parsing is rule-based and predictable; there is no
  required LLM. On ambiguous input `rr` returns a clear error instead of
  guessing, and lowers its confidence rather than inventing an exact time.
- **Transparent.** The parser reports byte-offset spans for the time, message,
  and filler, which the renderer highlights so you can verify interpretation.
- **Local + durable.** Reminders live in a SQLite database under your platform
  data directory (`~/.local/share/rapid-reminder/reminders.db` on Linux), with
  migrations and stable, user-visible ids.
- **Reliable delivery.** The daemon fires a reminder only after its notification
  succeeds and never fires it twice; it shuts down cleanly on SIGINT/SIGTERM.

The codebase keeps parsing, rendering, storage, notification, and scheduling in
separate modules. See [`SPEC.md`](./SPEC.md) for the full specification and
[`MOTIVATION.md`](./MOTIVATION.md) for the product rationale.

## Development

```console
$ cargo test --all-features
$ cargo fmt --all
$ cargo clippy --all-targets --all-features -- -D warnings
```

The natural-language corpus in [`tests/fixtures/reminders.yaml`](./tests/fixtures/reminders.yaml)
is part of the product — adding a phrase there is one of the easiest and most
valuable contributions. See [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or
[MIT license](./LICENSE-MIT) at your option.

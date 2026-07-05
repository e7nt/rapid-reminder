# Rapid Reminder — Implementation Roadmap

This is the build checklist for `rr`. Work proceeds in **stages**. Each stage is
small, self-contained, and ends in a **testable deliverable** — something you can
run and verify before moving on. No stage depends on unfinished work in a later
stage.

Legend: `[ ]` todo · `[~]` in progress · `[x]` done

Related docs: [`SPEC.md`](./SPEC.md) · [`MOTIVATION.md`](./MOTIVATION.md) · [`AGENT.md`](./AGENT.md) · [`CLAUDE.md`](./CLAUDE.md)

---

## Stage 0 — Project skeleton & toolchain

Get a compiling, testable project with the right structure before any real logic.

- [x] `cargo init --name rapid-reminder` (binary `rr` via `[[bin]]`)
- [x] Add dependencies: `clap` (derive), `chrono`, `thiserror`, `anyhow`
- [x] Add dev-deps: `insta`, `proptest`, `serde` + `serde_yaml`
- [x] Create module skeleton from SPEC §12.2 (empty stubs that compile):
      `cli`, `parser/`, `render/`, `storage/`, `notify/`, `daemon`, `time`, `ids`
- [x] `rr --version` and `rr --help` wired through `clap`
- [x] `.gitignore` (`/target`, `*.db`), commit `.pre-commit-config.yaml`
- [x] First commit

**Deliverable (testable):**
```bash
cargo build          # compiles clean
cargo test           # runs (0 tests OK)
./target/debug/rr --version
./target/debug/rr --help
```

---

## Stage 1 — Relative-time parser (deterministic core)

The heart of the product. Text in → structured reminder + byte-offset spans out.

- [x] Define types: `ParsedReminder`, `ParsedSpan`, `SpanKind`, `Confidence` (SPEC §6.2)
- [x] Define `ParseError` with `thiserror` (`EmptyInput`, `MissingTime`, `MissingMessage`, `TimeOutOfRange`)
- [x] `parse_reminder(input, now) -> Result<ParsedReminder, ParseError>`
- [x] Relative patterns: `in 15 mins`, `in 10m`, `in 2 hours`, `in 1h`, `in 30 minutes` (+ seconds/days, `a`/`an`)
- [x] Filler stripping: `remind me`, `to`, `about`, `please`, `ping me`
- [x] Spans are correct byte offsets on UTF-8 boundaries
- [x] Error cases return helpful messages (SPEC §6.5)
- [x] Unit tests for duration parsing, span extraction, error paths

> Notes: split into a **library crate + thin `rr` binary** so tests (and Stage 2
> fixtures) can drive the engine directly. `Confidence::Low` is defined per SPEC
> but not yet produced — it arrives with ambiguous absolute parsing (Stage 8).
> `rr <text>` now shows an honest plain-text preview; colored highlighting is Stage 3.

**Deliverable (testable):**
```bash
cargo test parser
# Unit tests prove: "remind me in 15 mins to check build" ->
#   due_at = now + 900s, message = "check build", time_span = "15 mins"
```

---

## Stage 2 — Fixture-backed parser test harness

Make the natural-language corpus a first-class, growable asset.

- [x] `tests/fixtures/reminders.yaml` schema (input, now, expected offset/message/span/confidence; plus `error:` cases)
- [x] Test loader that runs every fixture through `parse_reminder`
- [x] Seed **50** initial fixtures across relative-time phrasings (50 success + 10 error = 60)
- [x] Clear failure output showing input + expected vs actual (aggregated, all failures at once)

**Deliverable (testable):**
```bash
cargo test fixtures   # all 50 fixtures pass; adding a bad fixture fails loudly
```

---

## Stage 3 — Highlighted terminal output

Turn parsing into a visible contract so users trust it.

- [x] `render_highlighted(input, spans, color) -> String`
- [x] `ColorMode { Auto, Always, Never }`; respect `NO_COLOR` and `--color`
- [x] Time span green, message cyan, filler dimmed (SPEC §7)
- [x] Never panics on malformed spans (skip/err safely)
- [x] Snapshot tests (`insta`) with color disabled

> Also wired the `Understood:` block + `--color` flag into `rr <text>`; malformed
> spans (out-of-bounds, off-boundary, reversed, overlapping) are all skipped.

**Deliverable (testable):**
```bash
cargo test render
NO_COLOR=1 ./target/debug/rr "in 15 mins check build"   # plain highlighted output
./target/debug/rr "in 15 mins check build"              # colored "Understood:" block
```

---

## Stage 4 — Local SQLite storage

Persist reminders reliably with a stable, migratable schema.

- [x] `rusqlite` dependency (bundled SQLite); DB at platform data dir via `directories`
- [x] `Reminder`, `ReminderId`, `ReminderStatus` types (SPEC §8)
- [x] Schema migration v1 (tracked via `user_version` pragma); stable user-visible IDs
- [x] `insert`, `list_pending`, `get`, `cancel`, `mark_fired`
- [x] Tests use in-memory + `tempfile` DBs (no touching real data)
- [x] Wire `rr <text>` to actually store a reminder

**Deliverable (testable):**
```bash
cargo test storage
./target/debug/rr "in 15 mins check build"   # prints "✓ Reminder set" and persists
```

---

## Stage 5 — `list` and `cancel` commands

Close the manual lifecycle before automating firing.

- [x] `rr list` — table of pending reminders (ID / Due / Message), humanized `Today`/`Tomorrow`
- [x] `rr cancel <id>` — marks cancelled, confirms; missing id errors with exit 1
- [x] Snapshot test for the table + unit tests for due humanizing

**Deliverable (testable):**
```bash
./target/debug/rr "in 15 mins check build"
./target/debug/rr list      # shows the reminder with an ID
./target/debug/rr cancel 1  # "✓ Cancelled reminder 1"; list no longer shows it
```

---

## Stage 6 — Notifications behind a trait

Native desktop notifications, fully testable without a GUI.

- [x] `Notifier` trait: `notify(title, body) -> Result<(), NotifyError>`
- [x] `DesktopNotifier` via `notify-rust` (verified: delivers a real notification)
- [x] `FakeNotifier` recording calls (Mutex-backed so it is `Sync` for the daemon)
- [x] `NotifyError` typed; failures surfaced to callers (daemon/doctor use it)

**Deliverable (testable):**
```bash
cargo test notify          # fake notifier asserts title/body
# Manual: a tiny `rr test-notify` (or unit harness) pops a real desktop notification
```

---

## Stage 7 — Daemon (fires due reminders)

The piece that makes reminders actually arrive.

- [x] `rr daemon` polls `list_pending` every 5s
- [x] Fires only when due; `mark_fired` only after success; no duplicates (verified live)
- [x] Clean shutdown on SIGINT/SIGTERM (signal-hook + interruptible sleep)
- [x] `run_once(store, notifier, now)` is pure over time+notifier; 5 tests with fake clock + fake notifier

**Deliverable (testable):**
```bash
cargo test daemon
./target/debug/rr "in 1 min check build" && ./target/debug/rr daemon
# ~1 min later: a real desktop notification fires exactly once
```

---

## Stage 8 — Absolute & day-phrase parsing

Broaden the parser once the pipeline is proven end-to-end.

- [x] Absolute: `at 5pm`, `at 5 pm`, `at 17:30`, `at 5:30pm`, `at noon/midnight`, `tomorrow at 9am`
- [x] Day phrases: `tomorrow morning`, `tonight`, `next monday at 10am`, weekdays, dayparts
- [x] Ambiguity handling: daypart → Medium, bare `at 5` → Low; a day with no time errors (no guessing)
- [x] Grew fixtures to **106** (relative + absolute); added `proptest` for generated relative durations

> Also: extracted a shared `lex` module; `parse_reminder` tries relative then
> falls back to absolute; fixed a CLI arg-parsing bug (`rr --color never list`)
> by making the free-text reminder an `external_subcommand`.

**Deliverable (testable):**
```bash
cargo test              # 100+ fixtures + property tests green
./target/debug/rr "tomorrow at 9am review PR"   # correct due date, highlighted
```

---

## Stage 9 — `doctor` & hardening

- [x] `rr doctor` checks notification support (sends a test), storage access + path, daemon note
- [x] Pre-commit config present (`fmt`, `clippy -D warnings`, `test`, `deny`/`audit`/`machete` if installed)
- [x] GitHub Actions CI running fmt + clippy + test on stable
- [x] `cargo clippy --all-targets --all-features -- -D warnings` clean (throughout)

**Deliverable (testable):**
```bash
./target/debug/rr doctor       # reports OK/failing subsystems
pre-commit run --all-files     # all hooks pass
```

---

## Stage 10 — Open-source polish

- [ ] `README.md` with animated/asciinema demo and quick start
- [ ] `CONTRIBUTING.md` (esp. how to add fixtures)
- [ ] Install instructions (`cargo install`, prebuilt binaries)
- [ ] Release workflow (tags → built binaries)

**Deliverable (testable):**
```bash
cargo install --path .   # installs `rr` to ~/.cargo/bin
rr "in 5 mins ship it"   # works from a clean shell
```

---

## Future (post-v1, from SPEC §15)

- [ ] Optional LLM fallback for low-confidence parses (never required)
- [ ] Snooze command · recurring reminders · shell completions
- [ ] "Notify when this long command finishes" integration
- [ ] systemd / launchd / Task Scheduler integration · import/export

---

### Working agreement
- Deterministic parser before any LLM. Reliability over novelty.
- Every behavior change ships with a test or fixture.
- Keep parser / render / storage / notify / daemon boundaries clean.
- Run `cargo fmt && cargo clippy -D warnings && cargo test` before each commit.

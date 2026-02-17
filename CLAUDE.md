# CLAUDE.md — editor-dojo

This file provides guidance for AI assistants working in this repository.

## Project Overview

**editor-dojo** is a gamified, terminal-based training tool for mastering text editors (currently Helix, with plans for Vim, Neovim, and Emacs). Users complete editing challenges — transforming a "starting" buffer into a "target" buffer as efficiently as possible — while the tool tracks their time, keystroke count, and session recording.

Built in **Rust (2021 edition)** using a **Clean Architecture** (Hexagonal/Onion) pattern.

---

## Build & Development Commands

```bash
# Build
cargo build           # debug build
cargo build --release # release build

# Run
cargo run             # run from source

# Tests
cargo test            # run all unit tests

# Linting / Formatting
cargo fmt             # format code
cargo clippy          # lint (all warnings should be resolved before merging)
```

There is no CI configuration in the repo, but `cargo test`, `cargo fmt`, and `cargo clippy` are the expected quality gates.

---

## Architecture

The project strictly follows **Clean Architecture** with a unidirectional dependency rule:

```
domain  ←  application  ←  infrastructure
                ↑
               ui
```

- **domain** has zero external crate dependencies (pure Rust std only).
- **application** depends only on `domain`.
- **infrastructure** implements application traits using external crates.
- **ui** depends on `application` and `domain`; it uses `ratatui`/`crossterm` for TUI rendering.

### Layer Summary

| Layer | Path | Responsibility |
|---|---|---|
| Domain | `src/domain/` | Core entities and value objects |
| Application | `src/application/` | Business logic, orchestration, trait definitions |
| Infrastructure | `src/infrastructure/` | Concrete implementations (editor, file I/O, recorder) |
| UI | `src/ui/` | TUI screens rendered with ratatui |
| Entry point | `src/main.rs` | Dependency injection wiring, main loop |

---

## Source Structure

```
src/
├── main.rs                              # Entry point; wires DI and drives the main menu loop
│
├── domain/
│   ├── challenge.rs                     # Challenge entity (id, title, start/target content, hints, difficulty, tags)
│   ├── solution.rs                      # Solution value object (completed?, elapsed time, recording)
│   ├── recording.rs                     # Recording value object (path to .cast file, KeySequence)
│   ├── key_sequence.rs                  # Key sequence value object (Vec<String> of human-readable key names)
│   ├── challenge_stats.rs               # Per-challenge stats (best time, best keystrokes, attempt count)
│   ├── mastery_tier.rs                  # MasteryTier enum: Bronze / Silver / Gold (calculated from time + keystrokes)
│   ├── progress.rs                      # User-wide Progress entity (all challenge stats, streaks, achievements)
│   ├── achievement.rs                   # Achievement definitions and AchievementId enum
│   └── mod.rs
│
├── application/
│   ├── challenge_runner.rs              # ChallengeRunner<E,W,F>: orchestrates file → editor → validate flow
│   │                                   # Defines EditorSpawner, FileWatcher, FileSystem traits
│   ├── validator.rs                     # SolutionValidator: whitespace-normalized content comparison
│   ├── progress_repository.rs           # ProgressRepository trait (load/save)
│   ├── progress_tracker.rs              # ProgressTracker: thread-safe wrapper, records solutions, checks achievements
│   ├── achievement_checker.rs           # AchievementChecker: evaluates all AchievementId conditions against Progress
│   └── mod.rs
│
├── infrastructure/
│   ├── editor.rs                        # HelixEditor: implements EditorSpawner via `hx` process
│   ├── watcher.rs                       # FileChangeWatcher: implements FileWatcher via `notify` crate
│   ├── filesystem.rs                    # LocalFileSystem: implements FileSystem (tempfile, read, cleanup)
│   ├── recorder.rs                      # Recorder trait + AsciinemaRecorder implementation
│   ├── cast_parser.rs                   # CastParser: parses asciinema .cast files → KeySequence
│   ├── challenge_loader.rs              # ChallengeLoader trait + TomlChallengeLoader implementation
│   ├── json_progress_repository.rs      # JsonProgressRepository: implements ProgressRepository with JSON persistence
│   └── mod.rs
│
└── ui/
    ├── main_menu_screen.rs              # Main menu (Start Training, View Progress, Browse Challenges, Settings, Quit)
    ├── challenge_list_screen.rs         # Scrollable list of all challenges with completion/tier badges
    ├── challenge_screen.rs              # Challenge brief (description, start/target, hint, practice/challenge mode toggle)
    ├── results_screen.rs                # Post-challenge results (time, keystrokes, key sequence, achievements)
    ├── progress_screen.rs               # Overall progress dashboard (stats, streaks, achievements)
    └── mod.rs
```

---

## Key Domain Concepts

### Challenge
Defined by a TOML file. Fields:
- `id` — unique string identifier (e.g. `"delete-word-01"`)
- `title` / `description`
- `starting_content` / `target_content` — the text transformation task
- `hint` — editor-specific hint (prefers `[hints].helix`, falls back to `[hints].generic`)
- `difficulty` — optional string (`"beginner"`, etc.)
- `tags` — optional string list
- `progressive_hints` — up to 3 ordered hints (`hint_1`, `hint_2`, `hint_3`)
- `optimal_solution` / `optimal_keystrokes` — optional reference solution

### MasteryTier
Calculated from `(time, keystrokes)`:
- **Gold**: `< 15s` AND `< 30 keystrokes`
- **Silver**: `< 30s` AND `< 50 keystrokes`
- **Bronze**: completed (any time/keystrokes)

Without asciinema (no keystroke count), completion always earns Bronze.

### Validation
`SolutionValidator` normalizes content before comparing:
1. Trim leading/trailing whitespace from each line
2. Remove empty lines
3. Join with `\n`

This means trailing newlines and extra blank lines in the file don't cause false failures.

### Challenge Flow (ChallengeRunner)
1. Create a temp file with `starting_content`
2. Set up a file-change watcher on it
3. Optionally start asciinema recording
4. Spawn editor (or asciinema wrapping editor)
5. Poll: when a file-change event fires, read the file and validate
6. On success: kill editor, stop watcher, finalize recording
7. Return `Solution` with elapsed time and optional `Recording`

### Progress Persistence
Progress is saved as JSON to:
- Linux/Mac: `~/.local/share/editor-dojo/progress.json`
- Windows: `%APPDATA%/editor-dojo/progress.json`

On parse failure, the corrupted file is backed up to `.json.bak` and a fresh `Progress` is started.

### Recordings
Stored at `~/.local/share/editor-dojo/recordings/challenge-{id}-{timestamp}.cast`.

The `.cast` file format is asciinema v2 (newline-delimited JSON). Line 0 is a header; subsequent lines are `[timestamp, event_type, data]` arrays. `CastParser` extracts `"i"` (input) events.

---

## Challenge File Format (TOML)

All challenges live under `challenges/helix/` and are loaded in alphabetical order. Files are named `NN-slug.toml`.

```toml
[metadata]
id = "delete-word-01"          # unique, stable identifier
title = "Delete the word REMOVE"
description = "Remove the word 'REMOVE' from the text"
difficulty = "beginner"        # optional
tags = ["movement", "deletion"] # optional

[hints]
generic = "Move to the word, then delete it"      # fallback hint
helix = "Use 'w' to move by words, 'wd' to delete word"  # editor-specific (preferred)
hint_1 = "Word motions in Helix: 'w' moves to next word" # optional progressive hints
hint_2 = "Deletion: 'd' deletes the selection"
hint_3 = "Combine: 'w' to reach REMOVE, then 'wd' to delete"
optimal_solution = "wwd"       # optional, shown to user
optimal_keystrokes = 3         # required if optimal_solution is set

[content]
starting = "Hello REMOVE world"
target = "Hello world"
```

When adding a new challenge, follow the existing numbered naming convention and ensure `optimal_solution` and `optimal_keystrokes` are set together (or both absent).

---

## Achievements

Defined in `src/domain/achievement.rs` and evaluated by `AchievementChecker`:

| Achievement | Condition |
|---|---|
| First Steps | Complete 1 challenge |
| Lightning Fast | Complete any challenge in < 5s |
| Speed Demon | Complete 10 challenges in < 10s |
| Perfectionist | Complete any challenge with < 20 keystrokes |
| Efficiency Expert | Average keystrokes across all completions < 40 |
| Consistent Learner | Practice streak ≥ 7 days |
| Dedicated Practitioner | Practice streak ≥ 30 days |
| Halfway There | Complete ≥ 50% of all challenges |
| Challenge Master | Gold tier on ≥ 25 challenges |
| Gold Rush | Gold tier on 10 consecutive challenges |
| Completionist | Complete all challenges |
| Century Club | Complete 100 challenges total |

---

## UI Navigation

The TUI uses `ratatui` with `crossterm`. All screens follow the same pattern: create terminal → draw loop → restore terminal on exit.

Common key bindings across screens:
- `Esc` / `q` — go back / quit
- `Enter` — confirm / start

Challenge screen extras:
- `p` — toggle Practice Mode (results not recorded)
- `h` — toggle progressive hints overlay (only when hints exist)

---

## Key External Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` 0.28 | TUI rendering |
| `crossterm` 0.28 | Terminal input/output |
| `anyhow` 1.0 | Error handling |
| `notify` 6.1 | File system watcher |
| `tempfile` 3.13 | Temp file management |
| `serde` / `serde_json` 1.0 | JSON serialization for progress |
| `toml` 0.8 | TOML parsing for challenge files |
| `chrono` 0.4 | Timestamps and date arithmetic |
| `dirs` 5.0 | OS-appropriate data directories |
| `rand` 0.8 | Random number generation (reserved for future use) |

---

## Testing Conventions

- Unit tests live in `#[cfg(test)]` modules at the bottom of each source file.
- Infrastructure tests that need file I/O use `tempfile::TempDir` for isolation.
- Application-layer tests use inline mock structs implementing the relevant traits.
- No integration test framework is wired yet (the `test_integration.sh` script is a manual shell runner).
- Run all tests with `cargo test`.

---

## Adding New Features

### New challenge set (editor)
1. Create a new directory under `challenges/` (e.g. `challenges/vim/`)
2. Add `.toml` files following the format above
3. Update `main.rs` to load from the new directory, or make the path configurable

### New editor backend
1. Implement `EditorSpawner` for the new editor binary
2. Optionally implement a `Recorder` if asciinema wrapping is insufficient
3. Wire via dependency injection in `main.rs`

### New achievement
1. Add a variant to `AchievementId` in `src/domain/achievement.rs`
2. Add a match arm in `Achievement::get()` with name, description, badge
3. Add a match arm in `AchievementChecker::check_achievement()` with the logic
4. Add a test in the `#[cfg(test)]` block of `achievement_checker.rs`

### New UI screen
1. Create `src/ui/your_screen.rs` implementing a struct with a `show()` method
2. Follow the `ratatui::init()` / `ratatui::restore()` pattern
3. Export from `src/ui/mod.rs`
4. Wire from the appropriate menu action in `main.rs`

---

## Conventions & Style

- **Error handling**: use `anyhow::Result` everywhere; propagate with `?` and add context with `.context("...")`.
- **Getters**: domain entities expose data through `&self` methods; no public fields.
- **Builder pattern**: `Challenge` and `ChallengeRunner` use `with_*` methods for optional fields, keeping `new()` minimal.
- **Immutability**: `ChallengeStats::record_attempt` returns a new value rather than mutating in-place.
- **Trait objects vs generics**: `ChallengeRunner` uses generic type parameters (monomorphized); `Recorder` uses `Box<dyn Recorder>` (runtime dispatch) since it's optional.
- **No `unwrap()` in production paths**: use `expect()` only when a panic truly indicates a programming error, and prefer `?` / `ok_or` otherwise.
- **`#[allow(dead_code)]`**: used sparingly with a comment explaining why (e.g., constructed via serde, used in tests).

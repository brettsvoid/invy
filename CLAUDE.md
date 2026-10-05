# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**invy** is a Rust CLI tool for home inventory management with hierarchical places. Items are organized in a tree structure (e.g., garage -> toolbox -> hammer). Single SQLite database at `~/.local/share/invy/invy.db` (`$XDG_DATA_HOME` respected).

## Build Commands

```bash
cargo build                 # Debug build
cargo build --release       # Release build
cargo test                  # Run all integration tests
cargo test test_name        # Run specific test
cargo test -- --nocapture   # Show test output
cargo clippy                # Lint
cargo fmt                   # Format
```

## Architecture

```
src/
├── main.rs           # Entry point, routes commands
├── cli.rs            # Clap argument definitions
├── config.rs         # Config file (~/.config/invy/config.toml)
├── model.rs          # Data structures (Item, ItemWithPath, ListItem)
├── db.rs             # SQLite operations, migrations, queries
├── inventory.rs      # Rules every change follows: names, moves, destinations
├── search.rs         # fzf-syntax search (nucleo-matcher) for find, show and the TUI
├── output.rs         # Output formatting (human/JSON/CSV)
├── commands/         # Command implementations (add, find, list, show, mv, rm, edit)
└── tui/              # Interactive terminal UI (ratatui)
    ├── mod.rs        # Terminal setup and event loop
    ├── app.rs        # State, tree flattening, key handling, actions
    ├── ui.rs         # Rendering
    └── input.rs      # Single-line text input for prompts
```

**Flow:** CLI parsing (cli.rs) → Command handler (commands/*) → Rules (inventory.rs) → Database (db.rs) → Output formatting (output.rs)

**TUI flow:** `invy tui` → tui::run → App holds one Connection and calls inventory.rs for changes and db.rs for reads. It does not go through commands/* or output.rs.

Any rule about what a change may do (name checks, move checks, what a destination means) belongs in `inventory.rs`, never in a command or the TUI, so every front end enforces it the same way. A change that may auto-create a place runs in a transaction, so a refusal leaves nothing behind.

## Database Schema

Single `items` table with self-referential `place_id` foreign key. ON DELETE SET NULL orphans children when parent is deleted. Names can repeat, even in one place: an item is one physical thing, and identical items are duplicates (see `CONTEXT.md` and `docs/adr/0001-no-item-quantities.md`). `inventory::resolve` turns a name, path or `@id` into items, and treats matching duplicates as interchangeable.

Each item also has a `kind`: `room`, `furniture`, `box` or `thing` (the default). The set is a fixed enum in `model.rs`, not user-extensible. It is descriptive only — nothing restricts what can go where.

`GlyphSet` (`nerd` by default, `unicode`, `ascii`) decides kind icons, fold markers, tree branches and the `×` in duplicate counts. It is chosen by `--glyphs` / `INVY_GLYPHS` and stored in a `OnceLock` in `model.rs`. `Kind::glyph()` reads it; `Kind::glyph_in(set)` is the pure version to test against.

Every glyph must be one display column, or the TUI tree stops lining up. `every_glyph_occupies_a_single_column` in `model.rs` enforces this — keep it passing when adding a glyph.

Nerd Font codepoints were picked by reading the `post` table of an installed patched font and rendering candidates, not from memory. Do the same before changing them: the `post` table maps glyph ids to names like `md-dresser`, so you can search by name rather than guess.

### Migrations

`PRAGMA user_version` tracks the schema version, and `db::migrate` upgrades a file on open. To add a step: bump `SCHEMA_VERSION`, add an `if version < N` block, and add a test in `db.rs` that builds a fixture at version N-1. Never edit an earlier block — existing files have already run it.

## Testing

- Integration tests in `tests/` directory using `assert_cmd` and `predicates`
- `TestEnv` harness in `tests/common/mod.rs` creates isolated temporary databases
- Each command has dedicated test file (e.g., `tests/add_test.rs`)
- `TestEnv` points `HOME`, `XDG_CONFIG_HOME` and `XDG_DATA_HOME` into its temp dir, so a real config never leaks in. Use `cmd_without_db()` and `write_config()` to test config behaviour
- TUI logic is covered by unit tests in `src/tui/app.rs`. They drive `App::on_key` over a temporary database. Run with `cargo test --bin invy`

## Key Behaviors

- Paths use `/` separator (e.g., `garage/toolbox/hammer`)
- `--json`, `--csv` flags for output format
- `--db <path>` overrides default database location
- `~/.config/invy/config.toml` (`$XDG_CONFIG_HOME` respected) can set `db`. Precedence: `--db`, then config, then `~/.local/share/invy/invy.db`. Unknown keys are an error
- See SPEC.md for complete behavioral specification

## Commit Style

Use conventional commits: `type: description`

- `feat:` new feature
- `fix:` bug fix
- `refactor:` code change that neither fixes a bug nor adds a feature
- `docs:` documentation only
- `test:` adding or updating tests

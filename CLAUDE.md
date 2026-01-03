# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**invy** is a Rust CLI tool for home inventory management with hierarchical containers. Items are organized in a tree structure (e.g., garage -> toolbox -> hammer). Single SQLite database at `~/.invy.db`.

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
├── model.rs          # Data structures (Item, ItemWithPath, ListItem)
├── db.rs             # SQLite operations, migrations, queries
├── output.rs         # Output formatting (human/JSON/CSV)
└── commands/         # Command implementations (add, find, list, show, mv, rm, edit)
```

**Flow:** CLI parsing (cli.rs) → Command handler (commands/*) → Database (db.rs) → Output formatting (output.rs)

## Database Schema

Single `items` table with self-referential `container_id` foreign key. ON DELETE SET NULL orphans children when parent is deleted. Unique constraint on (name, container_id) prevents duplicate names within same container.

## Testing

- Integration tests in `tests/` directory using `assert_cmd` and `predicates`
- `TestEnv` harness in `tests/common/mod.rs` creates isolated temporary databases
- Each command has dedicated test file (e.g., `tests/add_test.rs`)

## Key Behaviors

- Paths use `/` separator (e.g., `garage/toolbox/hammer`)
- `--json`, `--csv` flags for output format
- `--db <path>` overrides default database location
- See SPEC.md for complete behavioral specification

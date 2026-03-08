# Workspace Reorganization Design

**Date:** 2026-03-07
**Branch:** `jcoletaylor/workspace-reorganization`
**Related:** TAS-361, TAS-362 (tasker-core equivalents)

## Problem

The storyteller workspace has grown to 6 Rust crates, 2 Python packages, and extensive build tooling — all sitting at the repository root alongside docs, config, and infrastructure. The flat structure makes it hard to distinguish crates from infrastructure at a glance and doesn't match the idiomatic Rust workspace convention used by mature projects (Bevy, Ratatui, Nushell).

## Target Structure

```
storyteller/
├── .cargo/config.toml          # Aliases, incremental=true for dev
├── .github/                    # CI workflows (paths updated)
├── Cargo.toml                  # Pure workspace manifest (no [package])
├── Cargo.lock
├── Makefile.toml               # cargo-make entry (extends tools/cargo-make/)
├── config/                     # Runtime configuration
├── docker/                     # PostgreSQL + AGE infrastructure
├── docs/                       # Design documentation
├── tests/                      # Workspace-level integration tests
│   └── fixtures/               # Test fixture data (moved from test_inputs/)
│
├── crates/
│   ├── storyteller/             # Root crate (integration test coordinator)
│   ├── storyteller-core/        # Types, traits, errors, DB
│   ├── storyteller-storykeeper/ # Persistence layer + migrations
│   ├── storyteller-engine/      # Bevy ECS runtime + agents
│   ├── storyteller-api/         # Axum HTTP layer
│   ├── storyteller-cli/         # CLI entry point + bin/
│   └── storyteller-ml/          # ML feature pipeline
│
├── tools/
│   ├── cargo-make/              # Task runner configs + scripts
│   ├── doc-tools/               # Python: Scrivener/DOCX extraction
│   └── training/                # Python: character prediction + event classifier
│
└── [root files: .env, .gitignore, LICENSE, README.md, Brewfile, etc.]
```

## Key Decisions

### Crates under `crates/`

All 7 Rust crates (including the root `storyteller` coordinator crate) move into `crates/`. The root `Cargo.toml` becomes a pure workspace manifest with no `[package]` section. This is the idiomatic pattern used by Bevy, Ratatui, and Nushell.

### Tools under `tools/`

Build tooling (`cargo-make/`) and Python packages (`doc-tools/`, `training/`) are all development utilities. Grouping them under `tools/` is accurate and avoids premature hierarchy like `packages/python/`.

### `test_inputs/` → `tests/fixtures/`

The `test_inputs/` directory contains dummy data from prototype play sessions. Moving it into `tests/fixtures/` gives it a proper home within the test infrastructure.

### `.cargo/config.toml` cleanup

- Enable `incremental = true` for dev builds (was incorrectly set to `false`)
- Remove commented-out mold linker configuration (mold is incompatible on macOS; the Xcode 15+ native `ld` linker is faster than the deprecated `sold` fork)

### What stays at root

- `docker/`, `config/`, `docs/`, `tests/` — shared resources, not owned by a single crate
- `Makefile.toml` — must be at root for cargo-make discovery
- All dotfiles/dotdirs (`.cargo/`, `.github/`, `.env`, etc.)

## Path Update Inventory

### Cargo.toml

- **Root `Cargo.toml`**: workspace `members` change to `crates/*` listing. Remove `[package]`, `[lib]`, `[dependencies]`, `[dev-dependencies]`, `[features]`, `[lints]` sections.
- **`crates/storyteller/Cargo.toml`**: new file with `[package]`, `[lib]`, `[features]`, `[dev-dependencies]`, `[lints]` from the old root.
- **Each crate's `Cargo.toml`**: inter-crate `path` dependencies verified (siblings under `crates/` use `path = "../storyteller-core"` etc.).

### cargo-make

- `Makefile.toml` at root: `extend` path changes from `cargo-make/main.toml` → `tools/cargo-make/main.toml`.
- Any `SCRIPTS_DIR` or relative path references inside cargo-make configs.

### CI workflows (.github/)

- Path filters, working directories, and any hardcoded crate paths.

### Documentation

- `CLAUDE.md`, `AGENTS.md` symlink, `README.md` — path references throughout.
- `.claude/` skill files if they reference crate paths.
- Memory files.

### Other

- `.gitignore` — check for crate-specific entries.
- Any `test_inputs` references in code → `tests/fixtures`.

## Risk Assessment

This is lower risk than tasker-core's equivalent reorganization (TAS-361) because:

- **No published crates** — no symlink or manifest verification needed.
- **No FFI workers** — no cross-language build complexity.
- **No `.sqlx/` cache** — migrations live in storyteller-storykeeper with crate-relative paths.
- **Fewer cargo-make tasks and CI workflows** — smaller surface area for breakage.
- **No cross-repo consumers** — tasker-contrib doesn't reference storyteller paths.

**Primary risk area:** cargo-make path references — most likely source of breakage.

## Out of Scope

- Crate renaming (names stay as-is)
- Any functional code changes
- Mold linker (incompatible on macOS)

## Acceptance Criteria

- All crates live under `crates/`
- Build tooling and Python packages live under `tools/`
- `test_inputs/` content moved to `tests/fixtures/`
- Root `Cargo.toml` is a pure workspace manifest
- `cargo make check` passes
- `cargo make test` passes
- CI pipeline passes
- All documentation updated (CLAUDE.md, AGENTS.md, README.md, skill files, memory files)

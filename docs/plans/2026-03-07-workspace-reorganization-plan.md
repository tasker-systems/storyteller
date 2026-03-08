# Workspace Reorganization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reorganize the storyteller workspace so Rust crates live under `crates/` and tooling lives under `tools/`, matching idiomatic Rust workspace conventions.

**Architecture:** Move 7 Rust crates into `crates/`, move cargo-make + Python packages into `tools/`, split root Cargo.toml into pure workspace manifest + coordinator crate. Update all path references in cargo-make, CI, and documentation.

**Tech Stack:** Rust workspace (cargo), cargo-make, GitHub Actions, Python (uv)

---

## Pre-flight: Understand What We're Changing

The current flat layout has 7 crates and 2 Python packages at the repo root. After this plan:

```
crates/{storyteller,storyteller-core,storyteller-engine,storyteller-api,storyteller-cli,storyteller-ml,storyteller-storykeeper}
tools/{cargo-make,doc-tools,training}
tests/fixtures/  (from test_inputs/)
```

Inter-crate path dependencies already use `../storyteller-*` and will continue to work since all crates remain siblings under `crates/`.

---

### Task 1: Move Rust Crates into `crates/`

**Files:**
- Create: `crates/` directory
- Move: `storyteller-core/`, `storyteller-engine/`, `storyteller-api/`, `storyteller-cli/`, `storyteller-ml/`, `storyteller-storykeeper/` into `crates/`
- Move: `src/` into `crates/storyteller/`

**Step 1: Create directory and move crates**

```bash
mkdir -p crates
git mv storyteller-core crates/
git mv storyteller-engine crates/
git mv storyteller-api crates/
git mv storyteller-cli crates/
git mv storyteller-ml crates/
git mv storyteller-storykeeper crates/
mkdir -p crates/storyteller
git mv src crates/storyteller/
```

**Step 2: Verify moves**

```bash
ls crates/
# Expected: storyteller/ storyteller-api/ storyteller-cli/ storyteller-core/ storyteller-engine/ storyteller-ml/ storyteller-storykeeper/
ls crates/storyteller/src/lib.rs
# Expected: file exists
```

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move Rust crates into crates/ directory"
```

---

### Task 2: Move Tooling into `tools/`

**Files:**
- Create: `tools/` directory
- Move: `cargo-make/`, `doc-tools/`, `training/` into `tools/`

**Step 1: Create directory and move tooling**

```bash
mkdir -p tools
git mv cargo-make tools/
git mv doc-tools tools/
git mv training tools/
```

**Step 2: Verify moves**

```bash
ls tools/
# Expected: cargo-make/ doc-tools/ training/
```

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move cargo-make and Python packages into tools/"
```

---

### Task 3: Move `test_inputs/` to `tests/fixtures/`

**Files:**
- Move: `test_inputs/` contents into `tests/fixtures/`

**Step 1: Move test data**

```bash
mkdir -p tests/fixtures
git mv test_inputs/* tests/fixtures/
rmdir test_inputs
```

**Step 2: Verify**

```bash
ls tests/fixtures/
# Expected: dedup_eval.txt
```

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move test_inputs/ to tests/fixtures/"
```

---

### Task 4: Split Root Cargo.toml — Create Coordinator Crate Manifest

**Files:**
- Create: `crates/storyteller/Cargo.toml`

**Step 1: Create the coordinator crate's Cargo.toml**

Create `crates/storyteller/Cargo.toml` with the `[package]`, `[lib]`, `[features]`, `[dev-dependencies]`, and `[lints]` sections extracted from the root:

```toml
[package]
name = "storyteller"
version = "0.1.0"
edition = "2021"
description = "Multi-agent storytelling engine — narrative gravity, character tensors, and collaborative world-building"
readme = "../../README.md"
license = "MIT"
repository = "https://github.com/tasker-systems/storyteller"
keywords = ["storytelling", "narrative", "agents", "bevy", "ecs"]
categories = ["game-engines", "simulation"]

[features]
test-ml-model = ["storyteller-engine/test-ml-model"]
test-llm = ["storyteller-engine/test-llm"]

[lib]
crate-type = ["rlib"]
name = "storyteller"
path = "src/lib.rs"

[dependencies]

[dev-dependencies]
storyteller-core = { path = "../storyteller-core" }
storyteller-engine = { path = "../storyteller-engine" }
storyteller-api = { path = "../storyteller-api" }
tokio = { workspace = true }

[lints]
workspace = true
```

Note: `readme` path changes to `../../README.md` since the crate is now two levels deep.

**Step 2: Verify file exists**

```bash
cat crates/storyteller/Cargo.toml
```

**Step 3: Commit**

```bash
git add crates/storyteller/Cargo.toml
git commit -m "refactor: create coordinator crate manifest at crates/storyteller/"
```

---

### Task 5: Update Root Cargo.toml — Pure Workspace Manifest

**Files:**
- Modify: `Cargo.toml` (workspace root)

**Step 1: Update root Cargo.toml**

Remove the `[package]`, `[features]`, `[lib]`, `[dependencies]`, `[dev-dependencies]`, and `[lints]` sections. Update `[workspace]` members to point to `crates/`:

```toml
[workspace]
members = [
    "crates/storyteller",
    "crates/storyteller-core",
    "crates/storyteller-storykeeper",
    "crates/storyteller-engine",
    "crates/storyteller-api",
    "crates/storyteller-cli",
    "crates/storyteller-ml",
]

# Everything below this line stays exactly as-is:
# [workspace.dependencies] ...
# [workspace.lints.clippy] ...
# [workspace.lints.rust] ...
# [profile.dev] ...
# [profile.release] ...
# [profile.coverage] ...
# [profile.profiling] ...
```

The `[workspace.dependencies]`, `[workspace.lints.*]`, and `[profile.*]` sections remain unchanged.

**Step 2: Verify cargo can find all crates**

```bash
cargo metadata --format-version=1 --no-deps | python3 -c "import sys,json; pkgs=json.load(sys.stdin)['packages']; [print(p['name']) for p in pkgs]"
```

Expected: all 7 crate names listed.

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "refactor: convert root Cargo.toml to pure workspace manifest"
```

---

### Task 6: Update Inter-Crate Path Dependencies

**Files:**
- Verify (likely no changes): `crates/storyteller-engine/Cargo.toml`, `crates/storyteller-api/Cargo.toml`, `crates/storyteller-cli/Cargo.toml`, `crates/storyteller-ml/Cargo.toml`, `crates/storyteller-storykeeper/Cargo.toml`

**Step 1: Verify path deps still resolve**

All crates already use `path = "../storyteller-*"` which remains correct since they're all siblings under `crates/`. Verify:

```bash
grep -rn 'path = ' crates/*/Cargo.toml
```

Expected: all paths are `"../storyteller-*"` — no changes needed since relative sibling paths are preserved.

**Step 2: Run cargo check**

```bash
cargo check --all-features 2>&1 | tail -20
```

Expected: successful compilation (warnings OK at this stage).

**Step 3: Commit (only if changes were needed)**

```bash
# Only if any Cargo.toml files were modified
git add crates/*/Cargo.toml
git commit -m "fix: update inter-crate path dependencies for crates/ layout"
```

---

### Task 7: Update Makefile.toml Extend Path

**Files:**
- Modify: `Makefile.toml` (line 15)

**Step 1: Update extend path**

Change:
```toml
extend = "./cargo-make/main.toml"
```
To:
```toml
extend = "./tools/cargo-make/main.toml"
```

**Step 2: Verify cargo-make can load**

```bash
cargo make --list-all-steps 2>&1 | head -5
```

Expected: task list displays without errors.

**Step 3: Commit**

```bash
git add Makefile.toml
git commit -m "fix: update Makefile.toml extend path for tools/cargo-make/"
```

---

### Task 8: Update cargo-make Internal Paths

**Files:**
- Modify: `tools/cargo-make/main.toml` (lines 83-95, 101-105, 145)
- Modify: `tools/cargo-make/scripts/setup-env.sh` (line 24)
- Modify: `tools/cargo-make/scripts/generate-db-schema.sh` (line 22, 24)

**Step 1: Update Python package paths in main.toml**

The `check-python` and `test-python` tasks reference `doc-tools` and `training` relative to workspace root. Update:

```toml
[tasks.check-python]
description = "Lint all Python packages"
script = [
    "cd tools/doc-tools && uv run ruff check .",
    "cd tools/training && uv run ruff check src/ tests/",
    "cd tools/training/event_classifier && uv run ruff check .",
]

[tasks.test-python]
description = "Test all Python packages"
script = [
    "cd tools/doc-tools && uv run pytest",
    "cd tools/training && uv run pytest tests/",
    "cd tools/training/event_classifier && uv run pytest",
]
```

**Step 2: Update setup-env.sh and generate-db-schema.sh script paths**

In `tools/cargo-make/main.toml`, update script references:

```toml
[tasks.setup-env]
script = ["bash tools/cargo-make/scripts/setup-env.sh --mode=test"]

[tasks.setup-env-ml]
script = ["bash tools/cargo-make/scripts/setup-env.sh --mode=test-ml"]

[tasks.setup-env-dev]
script = ["bash tools/cargo-make/scripts/setup-env.sh --mode=dev"]

[tasks.generate-db-schema]
script = ["bash tools/cargo-make/scripts/generate-db-schema.sh"]
```

**Step 3: Update shell script REPO_ROOT derivation**

Both scripts use `SCRIPT_DIR/../..` to find the repo root. After moving from `cargo-make/scripts/` (depth 2) to `tools/cargo-make/scripts/` (depth 3), update:

In `tools/cargo-make/scripts/setup-env.sh` (line 24):
```bash
WORKSPACE_PATH="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
```

In `tools/cargo-make/scripts/generate-db-schema.sh` (line 22):
```bash
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
```

**Step 4: Update generate-db-schema.sh migrations path**

In `tools/cargo-make/scripts/generate-db-schema.sh` (line 24):
```bash
MIGRATIONS_DIR="${REPO_ROOT}/crates/storyteller-storykeeper/migrations"
```

**Step 5: Verify cargo-make tasks resolve**

```bash
cargo make --list-all-steps 2>&1 | head -20
```

**Step 6: Commit**

```bash
git add Makefile.toml tools/cargo-make/
git commit -m "fix: update cargo-make paths for tools/ and crates/ layout"
```

---

### Task 9: Update .cargo/config.toml

**Files:**
- Modify: `.cargo/config.toml`

**Step 1: Enable incremental compilation and clean up dead config**

Replace the full file contents with:

```toml
[alias]
b = "build"
r = "run"
t = "test"
c = "check"
lint = "clippy --all-targets --all-features -- -D warnings"

[env]
WORKSPACE_PATH = { value = ".", relative = true }

[profile.dev]
incremental = true
split-debuginfo = "unpacked"  # Reduces debug artifact size on macOS

[profile.release]
incremental = false
```

Changes: `incremental = false` → `true` for dev, removed commented-out mold linker config.

**Step 2: Commit**

```bash
git add .cargo/config.toml
git commit -m "fix: enable incremental compilation, remove dead mold linker config"
```

---

### Task 10: Update CI Workflows

**Files:**
- Modify: `.github/workflows/test-python.yml` (working-directory references)

**Step 1: Update Python test workflow paths**

All `working-directory` references need `tools/` prefix:

- `doc-tools` → `tools/doc-tools`
- `training` → `tools/training`
- `training/event_classifier` → `tools/training/event_classifier`

**Step 2: Check other workflows for path references**

The Rust CI workflows (`code-quality.yml`, `test-rust.yml`) operate at workspace root using `cargo` commands — these should not need changes. Verify that `test-rust.yml` STORYTELLER_MODEL_PATH (`${{ github.workspace }}/tests/fixtures/models`) is already correct (tests/fixtures/ is where models live).

**Step 3: Commit**

```bash
git add .github/
git commit -m "fix: update CI workflow paths for tools/ layout"
```

---

### Task 11: Smoke Test — cargo make check

**Step 1: Run full quality check**

```bash
cargo make check
```

Expected: clippy, fmt check, and doc build all pass.

**Step 2: Run tests**

```bash
cargo make test
```

Expected: all unit tests pass.

**Step 3: If failures, fix path issues and re-run**

Common failure points:
- Cargo.toml path resolution errors → check `path = ` in crate manifests
- cargo-make script path errors → check `SCRIPT_DIR`/`REPO_ROOT` derivation
- Missing files → check git mv completeness

---

### Task 12: Update Documentation

**Files:**
- Modify: `CLAUDE.md` (crate paths, directory listing, command examples)
- Modify: `README.md` (directory structure if present)
- Modify: `tools/doc-tools/CLAUDE.md` (if it references parent paths)
- Verify: `AGENTS.md` symlink still works

**Step 1: Update CLAUDE.md**

Key sections to update:
- Workspace Architecture section: all `storyteller-*/src/` paths become `crates/storyteller-*/src/`
- Directory listings showing crate structure
- Any references to `cargo-make/` → `tools/cargo-make/`
- Any references to `doc-tools/` → `tools/doc-tools/`
- Any references to `training/` → `tools/training/`
- Python commands section: `cd doc-tools` → `cd tools/doc-tools`, `cd training` → `cd tools/training`

**Step 2: Verify AGENTS.md symlink**

```bash
ls -la AGENTS.md
# Expected: AGENTS.md -> CLAUDE.md
```

**Step 3: Update memory files**

Update `/Users/petetaylor/.claude/projects/-Users-petetaylor-projects-tasker-systems-storyteller/memory/MEMORY.md` with new directory paths.

**Step 4: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: update documentation for crates/ and tools/ workspace layout"
```

---

### Task 13: Final Verification and Cleanup

**Step 1: Full cargo check**

```bash
cargo make check
```

**Step 2: Full test suite**

```bash
cargo make test
```

**Step 3: Verify no stale references**

```bash
# Search for old top-level crate paths in non-Rust files
grep -rn "storyteller-core/" --include="*.md" --include="*.yml" --include="*.toml" --include="*.sh" . | grep -v "crates/storyteller-core" | grep -v target/ | grep -v ".git/"
```

Expected: no hits outside of `crates/` paths (except possibly in docs referencing module paths like `storyteller_core::` which are crate names, not file paths).

**Step 4: Verify workspace root is clean**

```bash
ls -d */ | sort
# Expected: crates/ config/ docker/ docs/ tests/ tools/ target/
# (no storyteller-* directories at root)
```

**Step 5: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore: final cleanup after workspace reorganization"
```

---

## Summary of All Path Changes

| Location | Old Path | New Path |
|----------|----------|----------|
| `Cargo.toml` members | `".", "storyteller-core", ...` | `"crates/storyteller", "crates/storyteller-core", ...` |
| `Makefile.toml` extend | `./cargo-make/main.toml` | `./tools/cargo-make/main.toml` |
| `main.toml` Python tasks | `cd doc-tools`, `cd training` | `cd tools/doc-tools`, `cd tools/training` |
| `main.toml` script tasks | `bash cargo-make/scripts/...` | `bash tools/cargo-make/scripts/...` |
| `setup-env.sh` WORKSPACE_PATH | `SCRIPT_DIR/../..` | `SCRIPT_DIR/../../..` |
| `generate-db-schema.sh` REPO_ROOT | `SCRIPT_DIR/../..` | `SCRIPT_DIR/../../..` |
| `generate-db-schema.sh` MIGRATIONS | `storyteller-storykeeper/migrations` | `crates/storyteller-storykeeper/migrations` |
| `.github/workflows/test-python.yml` | `working-directory: doc-tools` | `working-directory: tools/doc-tools` |
| `.github/workflows/test-python.yml` | `working-directory: training` | `working-directory: tools/training` |
| `.cargo/config.toml` | `incremental = false` | `incremental = true` |
| `CLAUDE.md` | `storyteller-*/src/` paths | `crates/storyteller-*/src/` paths |

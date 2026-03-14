# Engine Server Phase 1: Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone gRPC engine server that can compose scenes, run turn pipelines, and persist sessions — testable independently of the Tauri workshop.

**Architecture:** Extract scene composition into `storyteller-composer` crate. Add tonic gRPC service to `storyteller-api`. Server manages sessions via `EngineStateManager` (ArcSwap SWMR), persists events to `composition.json` + `events.jsonl` + `turns.jsonl`. All orchestration logic extracted from `commands.rs` into server-side service implementations.

**Tech Stack:** Rust, tonic (gRPC), prost (protobuf), arc-swap, dashmap, tokio, serde_json

**Design spec:** `docs/plans/2026-03-13-engine-server-and-playtest-harness-design.md`

---

## Chunk 1: Descriptor Migration + Composer Extraction

### Task 1: Descriptor UUIDv7 Migration Script

Add `entity_id` (UUIDv7) fields to all descriptor objects. Cross-reference arrays (`valid_archetypes`, `valid_dynamics`, `valid_profiles`) remain as string slugs — the gRPC layer does slug→UUID resolution at the boundary.

**Files:**
- Create: `tools/descriptor-migration/migrate_to_uuidv7.py`
- Modify: `storyteller-data/training-data/descriptors/genres.json`
- Modify: `storyteller-data/training-data/descriptors/archetypes.json`
- Modify: `storyteller-data/training-data/descriptors/profiles.json`
- Modify: `storyteller-data/training-data/descriptors/dynamics.json`
- Modify: `storyteller-data/training-data/descriptors/goals.json`
- Modify: `storyteller-data/training-data/descriptors/settings.json`
- Modify: `storyteller-data/training-data/descriptors/names.json`

**Context:**
- Descriptor files live in `storyteller-data/training-data/descriptors/` (separate repo, path via `STORYTELLER_DATA_PATH`)
- Current format: each object has `"id": "string_slug"`, cross-references use string slug arrays (`"valid_archetypes": ["wandering_artist", ...]`)
- Target format: each object adds `"entity_id": "019..."` (UUIDv7). Cross-reference arrays stay as string slugs (matching the `id` field they reference). Original `"id"` slug is retained for human readability and runtime lookup.
- UUIDv7 is time-ordered — use `uuid` Python package with `uuid7()`.
- Goals' internal `lexicon[].dimensional_context` references also stay as slugs — they're matched against descriptor `id` fields at runtime.

- [ ] **Step 1: Write the migration script**

```python
#!/usr/bin/env python3
"""Migrate descriptor files to use UUIDv7 entity identifiers.

Adds entity_id to every descriptor object. Cross-reference arrays
(valid_archetypes, valid_dynamics, valid_profiles) remain as string slugs
since runtime code matches against the 'id' field. Original string 'id'
fields are preserved as human-readable slugs.

Usage:
    python migrate_to_uuidv7.py /path/to/storyteller-data/training-data/descriptors
"""

import json
import sys
from pathlib import Path

# uuid7 requires uuid-utils or similar — fall back to uuid6 package
try:
    from uuid_utils import uuid7
except ImportError:
    try:
        from uuid6 import uuid7
    except ImportError:
        print("Install uuid-utils: pip install uuid-utils")
        sys.exit(1)


def build_slug_to_uuid(descriptors_dir: Path) -> dict[str, str]:
    """First pass: assign a UUIDv7 to every slug across all descriptor files."""
    slug_map: dict[str, str] = {}

    # Files with top-level arrays of objects with "id" fields
    array_files = {
        "genres.json": "genres",
        "archetypes.json": "archetypes",
        "profiles.json": "profiles",
        "dynamics.json": "dynamics",
        "goals.json": "goals",
    }

    for filename, key in array_files.items():
        path = descriptors_dir / filename
        if not path.exists():
            continue
        data = json.loads(path.read_text())
        for item in data.get(key, []):
            slug = item["id"]
            if slug not in slug_map:
                slug_map[slug] = str(uuid7())

    # Settings: keyed by genre slug, nested profile_settings keyed by profile slug
    # Settings don't have their own id fields — they're keyed by genre/profile slugs
    # which are already in the slug_map from genres.json and profiles.json

    # Names: keyed by genre slug — no separate id fields
    # Genre slugs already in slug_map

    return slug_map


def migrate_file(path: Path, key: str, slug_map: dict[str, str]) -> None:
    """Add entity_id to each descriptor object. Cross-ref arrays stay as slugs."""
    data = json.loads(path.read_text())
    items = data.get(key, [])

    for item in items:
        slug = item["id"]
        item["entity_id"] = slug_map[slug]

    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    print(f"  Migrated {path.name}: {len(items)} items")


def migrate_settings(path: Path, slug_map: dict[str, str]) -> None:
    """Settings are keyed by genre slug — add entity_id to setting objects."""
    if not path.exists():
        return
    data = json.loads(path.read_text())

    # Settings structure varies — handle both flat and nested formats
    # The key insight: settings are indexed by genre_id string, not by their own id
    # We don't need to add entity_id to settings themselves, just ensure
    # the genre keys are consistent (they remain as slugs for now since
    # settings are looked up by genre_id)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    print(f"  Settings: reformatted (keyed by genre slug, no migration needed)")


def migrate_names(path: Path, slug_map: dict[str, str]) -> None:
    """Names are keyed by genre slug — no entity_id needed on name entries."""
    if not path.exists():
        return
    data = json.loads(path.read_text())
    # Names are genre_id → {names: [...]} — no migration needed
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    print(f"  Names: reformatted (keyed by genre slug, no migration needed)")


def main():
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <descriptors-dir>")
        sys.exit(1)

    descriptors_dir = Path(sys.argv[1])
    if not descriptors_dir.is_dir():
        print(f"Not a directory: {descriptors_dir}")
        sys.exit(1)

    print("Pass 1: Building slug → UUIDv7 mapping...")
    slug_map = build_slug_to_uuid(descriptors_dir)
    print(f"  {len(slug_map)} slugs mapped")

    # Write slug map for reference / debugging
    map_path = descriptors_dir / "slug_to_uuid.json"
    map_path.write_text(json.dumps(slug_map, indent=2) + "\n")
    print(f"  Slug map written to {map_path.name}")

    print("\nPass 2: Migrating descriptor files...")

    migrate_file(descriptors_dir / "genres.json", "genres", slug_map)
    migrate_file(descriptors_dir / "archetypes.json", "archetypes", slug_map)
    migrate_file(descriptors_dir / "profiles.json", "profiles", slug_map)
    migrate_file(descriptors_dir / "dynamics.json", "dynamics", slug_map)
    migrate_file(descriptors_dir / "goals.json", "goals", slug_map)

    migrate_settings(descriptors_dir / "settings.json", slug_map)
    migrate_names(descriptors_dir / "names.json", slug_map)

    print("\nDone! Review changes and commit both repos.")
    print(f"Slug map saved to: {map_path}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run the migration**

```bash
# Install uuid-utils
pip install uuid-utils

# Run against descriptors directory
python tools/descriptor-migration/migrate_to_uuidv7.py \
    "$STORYTELLER_DATA_PATH/training-data/descriptors"
```

Verify: spot-check `genres.json` — each genre should have `entity_id` field, `valid_archetypes` should contain UUIDs, `id` slug preserved.

- [ ] **Step 3: Update descriptor Rust types to include entity_id**

Modify: `crates/storyteller-engine/src/scene_composer/descriptors.rs`

Add `entity_id: String` field to every descriptor struct that has an `id` field: `Genre`, `Archetype`, `Profile`, `Dynamic`, `Goal`. The field is a UUIDv7 string. Use `#[serde(default)]` for backward compat during transition.

```rust
// In each struct (Genre, Archetype, Profile, Dynamic, Goal):
/// UUIDv7 entity identifier for database and gRPC references.
#[serde(default)]
pub entity_id: String,
```

- [ ] **Step 4: Update SceneComposer lookups to support both slug and entity_id**

Add a `find_by_entity_id` variant or make `find_genre`/`find_archetype`/etc. check both `id` and `entity_id`. This ensures backward compatibility during migration.

```rust
// In catalog.rs, update find_genre etc:
pub(crate) fn find_genre(&self, id: &str) -> Option<&Genre> {
    self.descriptors.genres.iter().find(|g| g.id == id || g.entity_id == id)
}
```

Apply the same pattern to `find_archetype`, `find_profile`, `find_dynamic`.

- [ ] **Step 5: Verify existing tests still pass**

Run: `cargo test -p storyteller-engine --all-features`
Expected: All scene_composer tests pass (they use string slugs, which still work).

- [ ] **Step 6: Commit**

```bash
# In storyteller repo
git add tools/descriptor-migration/ crates/storyteller-engine/src/scene_composer/descriptors.rs \
    crates/storyteller-engine/src/scene_composer/catalog.rs
git commit -m "feat: add UUIDv7 entity_id to descriptors and dual-lookup in composer"

# In storyteller-data repo (separate commit)
cd "$STORYTELLER_DATA_PATH" && git add . && git commit -m "feat: add UUIDv7 entity_id to all descriptors"
```

---

### Task 2: Create storyteller-composer Crate

Extract scene composition logic from `storyteller-engine` into a new `storyteller-composer` crate.

**Files:**
- Create: `crates/storyteller-composer/Cargo.toml`
- Create: `crates/storyteller-composer/src/lib.rs`
- Move: `crates/storyteller-engine/src/scene_composer/` → `crates/storyteller-composer/src/`
- Modify: `Cargo.toml` (workspace members)
- Modify: `crates/storyteller-engine/Cargo.toml` (add storyteller-composer dep)
- Modify: `crates/storyteller-engine/src/lib.rs` (re-export from composer)

**Context:**
- `scene_composer/` contains: `mod.rs`, `catalog.rs`, `compose.rs`, `descriptors.rs`, `goals.rs`, `likeness.rs`, `names.rs`
- These modules depend on `storyteller-core` types (SceneData, CharacterSheet, EntityId, etc.) and `PlutchikWestern` grammar
- `storyteller-engine` uses `SceneComposer`, `SceneSelections`, `ComposedScene`, `ComposedGoals` from these modules
- After extraction, `storyteller-engine` depends on `storyteller-composer` for these types

- [ ] **Step 1: Create crate scaffold**

`crates/storyteller-composer/Cargo.toml`:
```toml
[package]
name = "storyteller-composer"
version = "0.1.0"
edition = "2021"
description = "Scene composition and descriptor catalog for the storyteller engine"
readme = "../README.md"
license = "MIT"
repository = "https://github.com/tasker-systems/storyteller"
keywords = ["storytelling", "narrative", "composition"]
categories = ["game-engines"]

[dependencies]
storyteller-core = { path = "../storyteller-core", version = "=0.1.0" }

serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
uuid = { workspace = true }
rand = { workspace = true }

[dev-dependencies]
serde_json = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Move scene_composer modules**

```bash
# Create source directory
mkdir -p crates/storyteller-composer/src

# Copy modules (we'll fix imports after)
cp crates/storyteller-engine/src/scene_composer/catalog.rs crates/storyteller-composer/src/
cp crates/storyteller-engine/src/scene_composer/compose.rs crates/storyteller-composer/src/
cp crates/storyteller-engine/src/scene_composer/descriptors.rs crates/storyteller-composer/src/
cp crates/storyteller-engine/src/scene_composer/goals.rs crates/storyteller-composer/src/
cp crates/storyteller-engine/src/scene_composer/likeness.rs crates/storyteller-composer/src/
cp crates/storyteller-engine/src/scene_composer/names.rs crates/storyteller-composer/src/
```

- [ ] **Step 3: Write lib.rs for storyteller-composer**

```rust
//! Scene composition and descriptor catalog for the storyteller engine.
//!
//! This crate owns the creative assembly layer: descriptor catalogs,
//! genre/archetype/setting/dynamics selection, character generation,
//! scene composition, and goal intersection. Hydrated from JSON descriptor
//! files today, eventually database-backed.

pub mod catalog;
pub mod compose;
pub mod descriptors;
pub mod goals;
pub mod likeness;
pub mod names;

pub use catalog::SceneComposer;
pub use compose::{CastSelection, ComposedScene, DynamicSelection, SceneSelections};
pub use descriptors::DescriptorSet;
pub use goals::{CastMember, CharacterGoal, ComposedGoals, GoalVisibility, SceneGoal};
```

- [ ] **Step 4: Fix imports in moved modules**

In all moved files, update `super::` references to use the new crate structure:
- `super::catalog::SceneComposer` → `crate::catalog::SceneComposer`
- `super::descriptors::*` → `crate::descriptors::*`
- `super::compose::*` → `crate::compose::*`
- `super::goals::*` → `crate::goals::*`
- `super::names::*` → `crate::names::*`

Key files to update:
- `catalog.rs`: `use super::descriptors::` → `use crate::descriptors::`
- `compose.rs`: `use super::catalog::SceneComposer` → `use crate::catalog::SceneComposer`, `use super::descriptors::` → `use crate::descriptors::`, `use super::names::` → `use crate::names::`
- `goals.rs`: `use super::descriptors::` → `use crate::descriptors::` AND in test module: `use crate::scene_composer::descriptors::*` → `use crate::descriptors::*`
- `likeness.rs`: `super::` → `crate::` AND in test module: `use super::super::descriptors::DimensionalContext` → `use crate::descriptors::DimensionalContext`

- [ ] **Step 5: Add storyteller-composer to workspace**

In root `Cargo.toml`, add to members:
```toml
members = [
    "crates/storyteller",
    "crates/storyteller-core",
    "crates/storyteller-storykeeper",
    "crates/storyteller-engine",
    "crates/storyteller-composer",  # NEW
    "crates/storyteller-api",
    "crates/storyteller-cli",
    "crates/storyteller-ml",
    "crates/storyteller-workshop/src-tauri",
]
```

- [ ] **Step 6: Update storyteller-engine to depend on storyteller-composer**

In `crates/storyteller-engine/Cargo.toml`:
```toml
storyteller-composer = { path = "../storyteller-composer", version = "=0.1.0" }
```

Replace the `scene_composer/` module directory in engine with a thin re-export:

`crates/storyteller-engine/src/scene_composer.rs` (replace the directory):
```rust
//! Re-exports from storyteller-composer for backward compatibility.
//!
//! Scene composition logic now lives in the `storyteller-composer` crate.
//! This module re-exports the public API so existing consumers
//! (workshop, tests) don't need to change their imports immediately.

pub use storyteller_composer::*;
```

Delete the `crates/storyteller-engine/src/scene_composer/` directory after confirming the re-export compiles.

- [ ] **Step 7: Verify compilation and tests**

```bash
cargo check --workspace --all-features
cargo test -p storyteller-composer --all-features
cargo test -p storyteller-engine --all-features
```

All existing tests should pass — the re-export ensures backward compatibility.

- [ ] **Step 8: Commit**

```bash
git add crates/storyteller-composer/ Cargo.toml \
    crates/storyteller-engine/Cargo.toml \
    crates/storyteller-engine/src/scene_composer.rs
git rm -r crates/storyteller-engine/src/scene_composer/
git commit -m "refactor: extract storyteller-composer crate from engine"
```

---

## Chunk 2: Proto Definitions + gRPC Build Infrastructure

### Task 3: Create Protobuf Definitions

Define the `StorytellerEngine` gRPC service with all RPCs and message types.

**Files:**
- Create: `proto/storyteller/v1/engine.proto`
- Create: `proto/storyteller/v1/composer.proto`
- Create: `proto/storyteller/v1/common.proto`

**Context:**
- See design spec section "gRPC Service Definition" for the full RPC list
- See design spec section "EngineEvent Envelope" for the event message shape
- Proto uses UUIDv7 strings for all entity identifiers
- Common types (timestamps, UUIDs) shared across services go in `common.proto`

- [ ] **Step 1: Create common.proto**

```protobuf
syntax = "proto3";
package storyteller.v1;

import "google/protobuf/empty.proto";

// Shared types used across storyteller services.
// google.protobuf.Empty is used for parameterless RPCs.
```

- [ ] **Step 2: Create engine.proto**

```protobuf
syntax = "proto3";
package storyteller.v1;

import "storyteller/v1/common.proto";
import "google/protobuf/empty.proto";

// Core engine service for gameplay and session management.
// Named StorytellerEngine to match the design spec.
service StorytellerEngine {
  // Gameplay RPCs (server-streaming)
  rpc ComposeScene(ComposeSceneRequest) returns (stream EngineEvent);
  rpc SubmitInput(SubmitInputRequest) returns (stream EngineEvent);
  rpc ResumeSession(ResumeSessionRequest) returns (stream EngineEvent);

  // Query RPCs (unary)
  rpc ListSessions(google.protobuf.Empty) returns (SessionList);
  rpc GetSceneState(GetSceneStateRequest) returns (SceneState);
  rpc CheckLlmStatus(google.protobuf.Empty) returns (LlmStatus);
  rpc GetPredictionHistory(PredictionHistoryRequest) returns (PredictionHistoryResponse);

  // Event replay (server-streaming)
  rpc GetSessionEvents(SessionEventsRequest) returns (stream StoredEvent);

  // Continuous push (server-streaming)
  rpc StreamLogs(LogFilter) returns (stream LogEntry);
}

// --- Requests ---

message ComposeSceneRequest {
  string genre_id = 1;       // UUIDv7 or slug
  string profile_id = 2;     // UUIDv7 or slug
  repeated CastMember cast = 3;
  repeated DynamicPairing dynamics = 4;
  optional string title_override = 5;
  optional string setting_override = 6;
  optional uint64 seed = 7;
}

message CastMember {
  string archetype_id = 1;   // UUIDv7 or slug
  optional string name = 2;
  string role = 3;
}

message DynamicPairing {
  string dynamic_id = 1;     // UUIDv7 or slug
  uint32 cast_index_a = 2;
  uint32 cast_index_b = 3;
}

message SubmitInputRequest {
  string session_id = 1;
  string input = 2;
}

message ResumeSessionRequest {
  string session_id = 1;
}

message GetSceneStateRequest {
  string session_id = 1;
}

message PredictionHistoryRequest {
  string session_id = 1;
  optional uint32 from_turn = 2;
  optional uint32 to_turn = 3;
}

message SessionEventsRequest {
  string session_id = 1;
  repeated string event_types = 2;  // optional filter
}

message LogFilter {
  optional string level = 1;     // trace, debug, info, warn, error
  optional string target = 2;    // module path filter
}

// --- EngineEvent envelope ---

message EngineEvent {
  string event_id = 1;       // UUIDv7
  string session_id = 2;
  optional uint32 turn = 3;
  string timestamp = 4;      // RFC3339

  oneof payload {
    PhaseStarted phase_started = 10;
    DecompositionComplete decomposition = 11;
    PredictionComplete prediction = 12;
    ArbitrationComplete arbitration = 13;
    IntentSynthesisComplete intent_synthesis = 14;
    ContextAssembled context = 15;
    NarratorToken narrator_token = 16;
    NarratorComplete narrator_complete = 17;
    SceneComposed scene_composed = 18;
    GoalsGenerated goals = 19;
    TurnComplete turn_complete = 20;
    ErrorOccurred error = 30;
  }
}

message PhaseStarted {
  string phase = 1;
}

message DecompositionComplete {
  string raw_json = 1;       // serialized EventDecomposition
}

message PredictionComplete {
  string raw_json = 1;       // serialized prediction data
}

message ArbitrationComplete {
  string verdict = 1;        // Permitted, Impossible, Ambiguous
  string details = 2;
}

message IntentSynthesisComplete {
  string intent_statements = 1;
}

message ContextAssembled {
  uint32 preamble_tokens = 1;
  uint32 journal_tokens = 2;
  uint32 retrieved_tokens = 3;
  uint32 total_tokens = 4;
}

message NarratorToken {
  string token = 1;
}

message NarratorComplete {
  string prose = 1;
  uint64 generation_ms = 2;
}

message SceneComposed {
  string title = 1;
  string setting_description = 2;
  repeated string cast_names = 3;
  string composition_json = 4;  // full composition for persistence
}

message GoalsGenerated {
  repeated string scene_goals = 1;
  repeated string character_goals = 2;
  optional string scene_direction = 3;
  optional string character_drives = 4;
  optional string player_context = 5;
  uint64 timing_ms = 6;
}

message TurnComplete {
  uint32 turn = 1;
  uint64 total_ms = 2;
}

message ErrorOccurred {
  string phase = 1;
  string message = 2;
}

// --- Query responses ---

message SessionList {
  repeated SessionSummary sessions = 1;
}

message SessionSummary {
  string session_id = 1;
  string genre = 2;
  string profile = 3;
  string title = 4;
  repeated string cast_names = 5;
  uint32 turn_count = 6;
  string created_at = 7;
}

message SceneState {
  string session_id = 1;
  string title = 2;
  string setting_description = 3;
  repeated CharacterState characters = 4;
  optional string scene_goals_json = 5;
  optional string intentions_json = 6;
  uint32 current_turn = 7;
}

message CharacterState {
  string entity_id = 1;
  string name = 2;
  string role = 3;
  string performance_notes = 4;
}

message LlmStatus {
  bool narrator_available = 1;
  string narrator_model = 2;
  bool decomposition_available = 3;
  string decomposition_model = 4;
  bool intent_available = 5;
  string intent_model = 6;
  bool predictor_available = 7;
}

message PredictionHistoryResponse {
  string raw_json = 1;       // serialized PredictionHistory
}

message StoredEvent {
  string event_id = 1;
  string event_type = 2;
  string payload_json = 3;
  string timestamp = 4;
}

message LogEntry {
  string level = 1;
  string target = 2;
  string message = 3;
  string timestamp = 4;
}
```

- [ ] **Step 3: Create composer.proto**

```protobuf
syntax = "proto3";
package storyteller.v1;

import "storyteller/v1/common.proto";
import "google/protobuf/empty.proto";

// Catalog service for scene composition options.
service ComposerService {
  rpc ListGenres(google.protobuf.Empty) returns (GenreList);
  rpc GetProfilesForGenre(GenreRequest) returns (ProfileList);
  rpc GetArchetypesForGenre(GenreRequest) returns (ArchetypeList);
  rpc GetDynamicsForGenre(DynamicsRequest) returns (DynamicsList);
  rpc GetNamesForGenre(GenreRequest) returns (NameList);
  rpc GetSettingsForGenre(GenreRequest) returns (SettingList);
}

message GenreRequest {
  string genre_id = 1;   // UUIDv7 or slug
}

message DynamicsRequest {
  string genre_id = 1;
  repeated string selected_archetype_ids = 2;
}

message GenreList {
  repeated GenreInfo genres = 1;
}

message GenreInfo {
  string entity_id = 1;
  string slug = 2;
  string display_name = 3;
  string description = 4;
  uint32 archetype_count = 5;
  uint32 profile_count = 6;
  uint32 dynamic_count = 7;
}

message ProfileList {
  repeated ProfileInfo profiles = 1;
}

message ProfileInfo {
  string entity_id = 1;
  string slug = 2;
  string display_name = 3;
  string description = 4;
  string scene_type = 5;
  double tension_min = 6;
  double tension_max = 7;
  uint32 cast_size_min = 8;
  uint32 cast_size_max = 9;
}

message ArchetypeList {
  repeated ArchetypeInfo archetypes = 1;
}

message ArchetypeInfo {
  string entity_id = 1;
  string slug = 2;
  string display_name = 3;
  string description = 4;
}

message DynamicsList {
  repeated DynamicInfo dynamics = 1;
}

message DynamicInfo {
  string entity_id = 1;
  string slug = 2;
  string display_name = 3;
  string description = 4;
  string role_a = 5;
  string role_b = 6;
}

message NameList {
  repeated string names = 1;
}

message SettingList {
  repeated SettingInfo settings = 1;
}

message SettingInfo {
  string profile_id = 1;
  string name = 2;
  optional string description = 3;
}
```

- [ ] **Step 4: Commit proto definitions**

```bash
git add proto/
git commit -m "feat: add protobuf definitions for engine and composer services"
```

---

### Task 4: Set Up tonic-build in storyteller-api

Configure `build.rs` to compile proto files and update dependencies.

**Files:**
- Create: `crates/storyteller-api/build.rs`
- Modify: `crates/storyteller-api/Cargo.toml`
- Modify: `Cargo.toml` (workspace dependencies: add tonic-build, tokio-stream, arc-swap, dashmap, tempfile)
- Create: `crates/storyteller-api/src/proto.rs` (re-export generated code)

**Context:**
- `tonic` 0.14 and `prost` 0.14 already in workspace dependencies
- Need to add `tonic-build` and `prost-build` as build dependencies
- Proto files at `../../proto/` relative to `crates/storyteller-api/`
- Generated code lives in `OUT_DIR`, included via `tonic::include_proto!`

- [ ] **Step 1: Add new dependencies to workspace Cargo.toml**

```toml
# Under [workspace.dependencies], add:
tonic-build = "0.14"
tokio-stream = "0.1"
arc-swap = "1"
dashmap = "6"
tempfile = "3"
```

- [ ] **Step 2: Update storyteller-api Cargo.toml**

```toml
[dependencies]
storyteller-core = { path = "../storyteller-core", version = "=0.1.0" }
storyteller-engine = { path = "../storyteller-engine", version = "=0.1.0" }
storyteller-composer = { path = "../storyteller-composer", version = "=0.1.0" }

# HTTP + gRPC framework
axum = { workspace = true }
tonic = { workspace = true }
prost = { workspace = true }
tokio = { workspace = true }
tokio-stream = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }

# Concurrency
arc-swap = { workspace = true }
dashmap = { workspace = true }

[build-dependencies]
tonic-build = { workspace = true }

[dev-dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
tempfile = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 3: Create build.rs**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)  // client stubs for integration tests
        .compile_protos(
            &[
                "../../proto/storyteller/v1/engine.proto",
                "../../proto/storyteller/v1/composer.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
```

- [ ] **Step 4: Create proto module for re-exports**

`crates/storyteller-api/src/proto.rs`:
```rust
//! Generated protobuf types and gRPC service definitions.

pub mod storyteller {
    pub mod v1 {
        tonic::include_proto!("storyteller.v1");
    }
}

pub use storyteller::v1::*;
```

- [ ] **Step 5: Add proto module to lib.rs**

In `crates/storyteller-api/src/lib.rs`, add:
```rust
pub mod proto;
```

- [ ] **Step 6: Verify proto compilation**

```bash
cargo check -p storyteller-api
```

Expected: compiles successfully. The generated types should be accessible as `storyteller_api::proto::EngineEvent`, `storyteller_api::proto::storyteller_engine_server::StorytellerEngine`, etc.

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-api/build.rs crates/storyteller-api/Cargo.toml \
    crates/storyteller-api/src/proto.rs crates/storyteller-api/src/lib.rs \
    Cargo.toml
git commit -m "feat: set up tonic-build for gRPC proto compilation"
```

---

## Chunk 3: Server Infrastructure

### Task 5: Event Persistence

Implement the three-file session persistence model: `composition.json`, `events.jsonl`, `turns.jsonl`.

**Files:**
- Create: `crates/storyteller-api/src/persistence/mod.rs`
- Create: `crates/storyteller-api/src/persistence/composition.rs`
- Create: `crates/storyteller-api/src/persistence/events.rs`
- Create: `crates/storyteller-api/src/persistence/turns.rs`
- Create: `crates/storyteller-api/src/persistence/session_store.rs`

**Context:**
- Design spec: composition.json is write-once, events.jsonl is append-only (one EngineEvent per line), turns.jsonl is append-only turn index referencing event UUIDs
- Session directory: `.story/sessions/{uuidv7}/`
- Breaking change from old format (see design spec)
- Current `SessionStore` in workshop `session.rs` (~873 lines) — use as reference for directory setup, .gitignore, listing logic
- Events are serialized as JSON with `event_id`, `event_type`, `payload`, `session_id`, `turn`, `timestamp`

- [ ] **Step 1: Write tests for CompositionWriter**

Test: write a composition, read it back, verify round-trip. Test that writing twice to the same session fails (write-once semantics).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_and_read_composition() {
        let dir = TempDir::new().unwrap();
        let writer = CompositionWriter::new(dir.path());

        let composition = serde_json::json!({
            "selections": {"genre_id": "test"},
            "scene": {"title": "Test Scene"},
            "characters": [],
            "goals": null,
            "intentions": null
        });

        writer.write("session-1", &composition).unwrap();
        let read_back = writer.read("session-1").unwrap();
        assert_eq!(read_back["selections"]["genre_id"], "test");
    }

    #[test]
    fn write_composition_twice_fails() {
        let dir = TempDir::new().unwrap();
        let writer = CompositionWriter::new(dir.path());
        let data = serde_json::json!({"test": true});

        writer.write("session-1", &data).unwrap();
        let result = writer.write("session-1", &data);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Implement CompositionWriter**

```rust
//! Write-once composition persistence.

use std::path::{Path, PathBuf};
use std::fs;

/// Writes and reads composition.json files (write-once per session).
#[derive(Debug, Clone)]
pub struct CompositionWriter {
    base_dir: PathBuf,
}

impl CompositionWriter {
    pub fn new(base_dir: &Path) -> Self {
        Self { base_dir: base_dir.to_path_buf() }
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id)
    }

    pub fn write(&self, session_id: &str, composition: &serde_json::Value) -> Result<(), String> {
        let dir = self.session_dir(session_id);
        fs::create_dir_all(&dir).map_err(|e| format!("create session dir: {e}"))?;

        let path = dir.join("composition.json");
        let json = serde_json::to_string_pretty(composition)
            .map_err(|e| format!("serialize composition: {e}"))?;

        // Atomic write-once: create_new(true) fails if file already exists (no TOCTOU race)
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("composition.json already exists or write failed for session {session_id}: {e}"))?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("write composition: {e}"))?;
        Ok(())
    }

    pub fn read(&self, session_id: &str) -> Result<serde_json::Value, String> {
        let path = self.session_dir(session_id).join("composition.json");
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("read composition: {e}"))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("parse composition: {e}"))
    }
}
```

- [ ] **Step 3: Write tests for EventWriter**

Test: append events, read them back in order. Verify UUIDv7 event_id and timestamp are set.

- [ ] **Step 4: Implement EventWriter**

Append-only JSONL writer. Each line is a serialized event with `event_id`, `event_type`, `session_id`, `turn`, `timestamp`, `payload`.

```rust
//! Append-only event stream persistence.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use chrono::Utc;

/// A single persisted event record.
/// Named `PersistedEvent` to avoid collision with the proto-generated `StoredEvent`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedEvent {
    pub event_id: String,
    pub event_type: String,
    pub session_id: String,
    pub turn: Option<u32>,
    pub timestamp: String,
    pub payload: serde_json::Value,
}

/// Appends events to events.jsonl for a session.
#[derive(Debug, Clone)]
pub struct EventWriter {
    base_dir: PathBuf,
}

impl EventWriter {
    pub fn new(base_dir: &Path) -> Self {
        Self { base_dir: base_dir.to_path_buf() }
    }

    /// Append an event and return its assigned event_id.
    pub fn append(
        &self,
        session_id: &str,
        event_type: &str,
        turn: Option<u32>,
        payload: &serde_json::Value,
    ) -> Result<String, String> {
        let dir = self.base_dir.join(session_id);
        fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;

        let event_id = Uuid::now_v7().to_string();
        let record = PersistedEvent {
            event_id: event_id.clone(),
            event_type: event_type.to_string(),
            session_id: session_id.to_string(),
            turn,
            timestamp: Utc::now().to_rfc3339(),
            payload: payload.clone(),
        };

        let mut line = serde_json::to_string(&record)
            .map_err(|e| format!("serialize event: {e}"))?;
        line.push('\n');

        let path = dir.join("events.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("open events.jsonl: {e}"))?;
        file.write_all(line.as_bytes())
            .map_err(|e| format!("write event: {e}"))?;

        Ok(event_id)
    }

    /// Read all events for a session.
    pub fn read_all(&self, session_id: &str) -> Result<Vec<PersistedEvent>, String> {
        let path = self.base_dir.join(session_id).join("events.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("read events: {e}"))?;
        contents
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| format!("parse event: {e}")))
            .collect()
    }
}
```

- [ ] **Step 5: Write tests for TurnWriter**

Test: append turns with event_id references, read them back, verify turn numbers and event_id lists.

- [ ] **Step 6: Implement TurnWriter**

```rust
//! Turn index persistence — references into the event stream.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// A turn index entry referencing events by ID.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnEntry {
    pub turn: u32,
    pub timestamp: String,
    pub player_input: Option<String>,
    pub event_ids: Vec<String>,
}

/// Appends turn index entries to turns.jsonl.
#[derive(Debug, Clone)]
pub struct TurnWriter {
    base_dir: PathBuf,
}

impl TurnWriter {
    pub fn new(base_dir: &Path) -> Self {
        Self { base_dir: base_dir.to_path_buf() }
    }

    pub fn append(&self, session_id: &str, entry: &TurnEntry) -> Result<(), String> {
        let dir = self.base_dir.join(session_id);
        fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;

        let mut line = serde_json::to_string(entry)
            .map_err(|e| format!("serialize turn: {e}"))?;
        line.push('\n');

        let path = dir.join("turns.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("open turns.jsonl: {e}"))?;
        file.write_all(line.as_bytes())
            .map_err(|e| format!("write turn: {e}"))?;
        Ok(())
    }

    pub fn read_all(&self, session_id: &str) -> Result<Vec<TurnEntry>, String> {
        let path = self.base_dir.join(session_id).join("turns.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("read turns: {e}"))?;
        contents
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| format!("parse turn: {e}")))
            .collect()
    }

    pub fn turn_count(&self, session_id: &str) -> Result<usize, String> {
        self.read_all(session_id).map(|v| v.len())
    }
}
```

- [ ] **Step 7: Create SessionStore that composes all three writers**

`crates/storyteller-api/src/persistence/session_store.rs`:

```rust
//! Unified session store composing composition, event, and turn writers.

use std::path::{Path, PathBuf};
use std::fs;

use super::composition::CompositionWriter;
use super::events::EventWriter;
use super::turns::TurnWriter;

/// Manages session directories and delegates to specialized writers.
#[derive(Debug, Clone)]
pub struct SessionStore {
    base_dir: PathBuf,
    pub composition: CompositionWriter,
    pub events: EventWriter,
    pub turns: TurnWriter,
}

impl SessionStore {
    pub fn new(base_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(base_dir)
            .map_err(|e| format!("create sessions dir: {e}"))?;

        // Prevent session data from being committed to git
        let gitignore = base_dir.join(".gitignore");
        if !gitignore.exists() {
            let _ = fs::write(&gitignore, "*\n");
        }

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            composition: CompositionWriter::new(base_dir),
            events: EventWriter::new(base_dir),
            turns: TurnWriter::new(base_dir),
        })
    }

    /// List all session IDs (directories in base_dir).
    pub fn list_session_ids(&self) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        let entries = fs::read_dir(&self.base_dir)
            .map_err(|e| format!("read sessions dir: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("read entry: {e}"))?;
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    ids.push(name.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Create a new session directory, returning the session_id (UUIDv7).
    pub fn create_session(&self) -> Result<String, String> {
        let session_id = uuid::Uuid::now_v7().to_string();
        let dir = self.base_dir.join(&session_id);
        fs::create_dir_all(&dir)
            .map_err(|e| format!("create session dir: {e}"))?;
        Ok(session_id)
    }
}
```

- [ ] **Step 8: Create persistence mod.rs**

```rust
//! Session persistence — composition, events, and turn index.

pub mod composition;
pub mod events;
pub mod turns;
pub mod session_store;

pub use session_store::SessionStore;
pub use composition::CompositionWriter;
pub use events::{EventWriter, PersistedEvent};
pub use turns::{TurnWriter, TurnEntry};
```

- [ ] **Step 9: Wire persistence module into lib.rs**

Add `pub mod persistence;` to `crates/storyteller-api/src/lib.rs`.

- [ ] **Step 10: Run tests**

```bash
cargo test -p storyteller-api --all-features
```

- [ ] **Step 11: Commit**

```bash
git add crates/storyteller-api/src/persistence/
git commit -m "feat: add event persistence model (composition, events, turns)"
```

---

### Task 6: EngineStateManager

Implement the SWMR session state manager using `ArcSwap` for lock-free reads.

**Files:**
- Create: `crates/storyteller-api/src/engine/mod.rs`
- Create: `crates/storyteller-api/src/engine/state_manager.rs`
- Create: `crates/storyteller-api/src/engine/providers.rs`
- Create: `crates/storyteller-api/src/engine/types.rs`

**Context:**
- Design spec section "Server-Side Engine State" defines the data model
- `ArcSwap<RuntimeSnapshot>` for lock-free reads, `Mutex<RuntimeMut>` for single writer
- `DashMap<String, SessionState>` for session lookup without global lock
- `EngineProviders` holds shared LLM/ML resources (not per-session)
- Composition is `Arc<Composition>` — immutable after creation
- Runtime snapshot published at phase boundaries during turn pipeline

- [ ] **Step 1: Write tests for EngineStateManager**

Test creation, session add/remove, snapshot read, snapshot update. Test concurrent reads during write (spawn tokio tasks).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_composition() -> Composition {
        Composition {
            scene: serde_json::json!({"title": "test"}),
            characters: vec![],
            goals: None,
            intentions: None,
            selections: serde_json::json!({}),
        }
    }

    #[test]
    fn create_and_get_session() {
        let mgr = EngineStateManager::new();
        let session_id = "test-session";
        mgr.create_session(session_id, make_test_composition());

        assert!(mgr.get_composition(session_id).is_some());
        assert!(mgr.get_runtime_snapshot(session_id).is_some());
    }

    #[test]
    fn get_nonexistent_session_returns_none() {
        let mgr = EngineStateManager::new();
        assert!(mgr.get_composition("nope").is_none());
    }

    #[tokio::test]
    async fn concurrent_reads_during_write() {
        let mgr = Arc::new(EngineStateManager::new());
        mgr.create_session("s1", make_test_composition());

        let mgr2 = mgr.clone();
        let reader = tokio::spawn(async move {
            for _ in 0..100 {
                let _ = mgr2.get_runtime_snapshot("s1");
            }
        });

        // Simulate writer updating snapshot
        mgr.update_runtime_snapshot("s1", |snap| {
            let mut new = snap.clone();
            new.turn_count = 5;
            new
        }).await;

        reader.await.unwrap();
        let snap = mgr.get_runtime_snapshot("s1").unwrap();
        assert_eq!(snap.turn_count, 5);
    }
}
```

- [ ] **Step 2: Define types**

`crates/storyteller-api/src/engine/types.rs`:

```rust
//! Server-side engine state types.

use std::sync::Arc;

/// Immutable composition data — created once per session.
#[derive(Debug, Clone)]
pub struct Composition {
    pub scene: serde_json::Value,
    pub characters: Vec<serde_json::Value>,
    pub goals: Option<serde_json::Value>,
    pub intentions: Option<serde_json::Value>,
    pub selections: serde_json::Value,
}

/// Mutable runtime state — published as snapshots via ArcSwap.
#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub journal_entries: Vec<String>,
    pub turn_count: u32,
    pub player_entity_id: Option<String>,
    pub prediction_history: Vec<serde_json::Value>,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            journal_entries: Vec::new(),
            turn_count: 0,
            player_entity_id: None,
            prediction_history: Vec::new(),
        }
    }
}
```

Note: Types use `serde_json::Value` for now. As the implementation matures, these will be replaced with strongly-typed domain objects from `storyteller-core`. This avoids coupling the server infrastructure to specific domain type versions during initial buildout.

- [ ] **Step 3: Implement EngineStateManager**

`crates/storyteller-api/src/engine/state_manager.rs`:

```rust
//! SWMR session state manager.
//!
//! Uses `DashMap` for lock-free session lookup and `ArcSwap` for
//! lock-free reads of runtime snapshots. Writers publish new snapshots
//! at pipeline phase boundaries.

use std::sync::Arc;
use arc_swap::ArcSwap;
use dashmap::DashMap;

use super::types::{Composition, RuntimeSnapshot};

struct SessionState {
    composition: Arc<Composition>,
    runtime: ArcSwap<RuntimeSnapshot>,
    write_handle: tokio::sync::Mutex<()>,  // SWMR: single writer guard
}

/// Manages all active sessions with lock-free reads.
#[derive(Debug)]
pub struct EngineStateManager {
    sessions: DashMap<String, SessionState>,
}

impl EngineStateManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    /// Create a new session with the given composition.
    pub fn create_session(&self, session_id: &str, composition: Composition) {
        let state = SessionState {
            composition: Arc::new(composition),
            runtime: ArcSwap::from_pointee(RuntimeSnapshot::default()),
            write_handle: tokio::sync::Mutex::new(()),
        };
        self.sessions.insert(session_id.to_string(), state);
    }

    /// Get the immutable composition for a session.
    pub fn get_composition(&self, session_id: &str) -> Option<Arc<Composition>> {
        self.sessions.get(session_id).map(|s| s.composition.clone())
    }

    /// Get a snapshot of the current runtime state (lock-free).
    pub fn get_runtime_snapshot(&self, session_id: &str) -> Option<Arc<RuntimeSnapshot>> {
        self.sessions.get(session_id).map(|s| s.runtime.load_full())
    }

    /// Update the runtime snapshot atomically. Acquires the write-side mutex
    /// to enforce SWMR: only one writer at a time per session.
    /// The closure receives the current snapshot and returns the new one.
    pub async fn update_runtime_snapshot(
        &self,
        session_id: &str,
        f: impl FnOnce(&RuntimeSnapshot) -> RuntimeSnapshot,
    ) {
        if let Some(state) = self.sessions.get(session_id) {
            let _guard = state.write_handle.lock().await;
            let current = state.runtime.load();
            let new_snapshot = f(&current);
            state.runtime.store(Arc::new(new_snapshot));
        }
    }

    /// Remove a session.
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Check if a session exists.
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// List all active session IDs.
    pub fn session_ids(&self) -> Vec<String> {
        self.sessions.iter().map(|e| e.key().clone()).collect()
    }
}

// Debug impl for SessionState (DashMap requires it transitively)
impl std::fmt::Debug for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionState")
            .field("composition", &"Arc<Composition>")
            .field("runtime", &"ArcSwap<RuntimeSnapshot>")
            .finish()
    }
}
```

- [ ] **Step 4: Implement EngineProviders**

`crates/storyteller-api/src/engine/providers.rs`:

```rust
//! Shared engine providers — LLM and ML resources shared across sessions.

use std::sync::Arc;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::structured_llm::StructuredLlmProvider;

/// Shared providers for LLM and ML inference.
///
/// These are stateless and can be safely shared across concurrent sessions.
/// Constructed once at server startup.
#[derive(Clone)]
pub struct EngineProviders {
    pub narrator_llm: Arc<dyn LlmProvider>,
    pub structured_llm: Option<Arc<dyn StructuredLlmProvider>>,
    pub intent_llm: Option<Arc<dyn LlmProvider>>,
    pub predictor_available: bool,
    // CharacterPredictor and PlutchikWestern will be added when
    // the turn pipeline is wired in Task 12+
}

impl std::fmt::Debug for EngineProviders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineProviders")
            .field("narrator_llm", &"Arc<dyn LlmProvider>")
            .field("structured_llm", &self.structured_llm.is_some())
            .field("intent_llm", &self.intent_llm.is_some())
            .field("predictor_available", &self.predictor_available)
            .finish()
    }
}
```

- [ ] **Step 5: Create engine mod.rs**

```rust
//! Server-side engine state and provider management.

pub mod providers;
pub mod state_manager;
pub mod types;

pub use providers::EngineProviders;
pub use state_manager::EngineStateManager;
pub use types::{Composition, RuntimeSnapshot};
```

- [ ] **Step 6: Wire engine module into lib.rs**

Add `pub mod engine;` to `crates/storyteller-api/src/lib.rs`.

- [ ] **Step 7: Run tests**

```bash
cargo test -p storyteller-api --all-features
```

- [ ] **Step 8: Commit**

```bash
git add crates/storyteller-api/src/engine/
git commit -m "feat: add EngineStateManager with ArcSwap SWMR and shared providers"
```

---

## Chunk 4: gRPC Service Implementations

### Task 7: Composer gRPC Service

Implement the `ComposerService` — unary read-only RPCs for catalog queries.

**Files:**
- Create: `crates/storyteller-api/src/grpc/mod.rs`
- Create: `crates/storyteller-api/src/grpc/composer_service.rs`
- Modify: `crates/storyteller-api/src/state.rs` (populate AppState)
- Modify: `crates/storyteller-api/src/lib.rs` (add grpc module, update AppState)

**Context:**
- `SceneComposer` in `storyteller-composer` already has all the catalog query methods: `genres()`, `profiles_for_genre()`, `archetypes_for_genre()`, `dynamics_for_genre()`, `names_for_genre()`
- The gRPC service wraps these with proto type conversion
- `SceneComposer` is loaded once at startup and shared via `Arc`
- Summary types in `catalog.rs` already mirror the proto response shapes closely
- Lookups support both slug and entity_id (from Task 1)

- [ ] **Step 1: Update AppState with real fields**

Replace the empty `AppState` in `state.rs`:

```rust
use std::sync::Arc;
use storyteller_composer::SceneComposer;
use crate::engine::{EngineProviders, EngineStateManager};
use crate::persistence::SessionStore;

/// Shared state available to all API handlers and gRPC services.
#[derive(Debug, Clone)]
pub struct AppState {
    pub composer: Arc<SceneComposer>,
    pub state_manager: Arc<EngineStateManager>,
    pub session_store: Arc<SessionStore>,
    pub providers: Arc<EngineProviders>,
}
```

- [ ] **Step 2: Implement ComposerServiceImpl**

```rust
//! gRPC ComposerService implementation.

use std::sync::Arc;
use tonic::{Request, Response, Status};
use storyteller_composer::SceneComposer;

use crate::proto::composer_service_server::ComposerService;
use crate::proto::*;

pub struct ComposerServiceImpl {
    composer: Arc<SceneComposer>,
}

impl ComposerServiceImpl {
    pub fn new(composer: Arc<SceneComposer>) -> Self {
        Self { composer }
    }
}

#[tonic::async_trait]
impl ComposerService for ComposerServiceImpl {
    async fn list_genres(
        &self,
        _request: Request<()>,
    ) -> Result<Response<GenreList>, Status> {
        let genres = self.composer.genres();
        let genre_infos = genres.into_iter().map(|g| GenreInfo {
            entity_id: String::new(), // TODO: wire entity_id from descriptor
            slug: g.id,
            display_name: g.display_name,
            description: g.description,
            archetype_count: g.archetype_count as u32,
            profile_count: g.profile_count as u32,
            dynamic_count: g.dynamic_count as u32,
        }).collect();

        Ok(Response::new(GenreList { genres: genre_infos }))
    }

    async fn get_profiles_for_genre(
        &self,
        request: Request<GenreRequest>,
    ) -> Result<Response<ProfileList>, Status> {
        let genre_id = &request.get_ref().genre_id;
        let profiles = self.composer.profiles_for_genre(genre_id);
        let profile_infos = profiles.into_iter().map(|p| ProfileInfo {
            entity_id: String::new(),
            slug: p.id,
            display_name: p.display_name,
            description: p.description,
            scene_type: p.scene_type,
            tension_min: p.tension_min,
            tension_max: p.tension_max,
            cast_size_min: p.cast_size_min as u32,
            cast_size_max: p.cast_size_max as u32,
        }).collect();

        Ok(Response::new(ProfileList { profiles: profile_infos }))
    }

    async fn get_archetypes_for_genre(
        &self,
        request: Request<GenreRequest>,
    ) -> Result<Response<ArchetypeList>, Status> {
        let genre_id = &request.get_ref().genre_id;
        let archetypes = self.composer.archetypes_for_genre(genre_id);
        let archetype_infos = archetypes.into_iter().map(|a| ArchetypeInfo {
            entity_id: String::new(),
            slug: a.id,
            display_name: a.display_name,
            description: a.description,
        }).collect();

        Ok(Response::new(ArchetypeList { archetypes: archetype_infos }))
    }

    async fn get_dynamics_for_genre(
        &self,
        request: Request<DynamicsRequest>,
    ) -> Result<Response<DynamicsList>, Status> {
        let req = request.get_ref();
        let dynamics = self.composer.dynamics_for_genre(
            &req.genre_id,
            &req.selected_archetype_ids,
        );
        let dynamic_infos = dynamics.into_iter().map(|d| DynamicInfo {
            entity_id: String::new(),
            slug: d.id,
            display_name: d.display_name,
            description: d.description,
            role_a: d.role_a,
            role_b: d.role_b,
        }).collect();

        Ok(Response::new(DynamicsList { dynamics: dynamic_infos }))
    }

    async fn get_names_for_genre(
        &self,
        request: Request<GenreRequest>,
    ) -> Result<Response<NameList>, Status> {
        let genre_id = &request.get_ref().genre_id;
        let names = self.composer.names_for_genre(genre_id);
        Ok(Response::new(NameList { names }))
    }

    async fn get_settings_for_genre(
        &self,
        request: Request<GenreRequest>,
    ) -> Result<Response<SettingList>, Status> {
        // Settings are keyed by genre_id in the descriptor set
        // For now, return what we can from the descriptors
        let _genre_id = &request.get_ref().genre_id;
        // TODO: Expose settings from DescriptorSet (currently accessed
        // internally by compose_setting but not via a public catalog query)
        Ok(Response::new(SettingList { settings: vec![] }))
    }
}
```

Note: The `entity_id` fields in responses are initially empty. They will be populated once the descriptor `entity_id` fields are wired through the `GenreSummary`, `ProfileSummary`, etc. types in `catalog.rs`. This is a follow-up within this task.

- [ ] **Step 3: Wire entity_id through catalog summary types**

Update `crates/storyteller-composer/src/catalog.rs` to include `entity_id` in all summary types:

```rust
// Add to GenreSummary, ProfileSummary, ArchetypeSummary, DynamicSummary:
pub entity_id: String,

// In the mapping functions, add:
entity_id: g.entity_id.clone(),  // (or a.entity_id, p.entity_id, d.entity_id)
```

Then update the ComposerService to use these entity_id values.

- [ ] **Step 4: Create grpc/mod.rs**

```rust
//! gRPC service implementations.

pub mod composer_service;
```

- [ ] **Step 5: Wire grpc module into lib.rs**

Add `pub mod grpc;` to `crates/storyteller-api/src/lib.rs`.

- [ ] **Step 6: Run tests**

```bash
cargo check -p storyteller-api --all-features
cargo test -p storyteller-api --all-features
```

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-api/src/grpc/ crates/storyteller-api/src/state.rs \
    crates/storyteller-api/src/lib.rs crates/storyteller-composer/src/catalog.rs
git commit -m "feat: implement ComposerService gRPC with catalog queries"
```

---

### Task 8: Engine gRPC Service — ComposeScene

Implement the `ComposeScene` server-streaming RPC. This extracts the core orchestration from `commands.rs::setup_and_render_opening()`.

**Files:**
- Create: `crates/storyteller-api/src/grpc/engine_service.rs`
- Modify: `crates/storyteller-api/src/grpc/mod.rs`

**Context:**
- `ComposeScene` is the most complex RPC — it composes the scene, generates intentions, renders opening narration, and streams `EngineEvent`s as each phase completes
- Source logic: `commands.rs::setup_and_render_opening()` (lines ~1061-1350) and `compose_scene()` Tauri command
- The server creates a tokio channel, spawns the pipeline work, and yields events from the receiver
- Events are also persisted via `EventWriter` and `TurnWriter` as they're produced
- Session composition written via `CompositionWriter`
- `EngineStateManager` updated with the new session

**Key extraction from commands.rs:**
1. Build `SceneSelections` from proto request
2. Call `composer.compose(selections)` → `ComposedScene`
3. Call `composer.intersect_goals(selections, composed)` → `ComposedGoals`
4. Create LLM providers (Ollama main, structured 3b, intent synthesis 3b)
5. Call `generate_intentions()` → `GeneratedIntentions`
6. Assemble narrator opening context
7. Call `NarratorAgent::render()` → opening prose
8. Persist composition.json, write events, write turn 0
9. Create EngineState in state manager

- [ ] **Step 1: Create engine_service.rs with ComposeScene**

```rust
//! gRPC EngineService implementation.

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use chrono::Utc;

use crate::proto::storyteller_engine_server::StorytellerEngine;
use crate::proto::*;
use crate::engine::{EngineStateManager, EngineProviders, Composition};
use crate::persistence::SessionStore;

use storyteller_composer::{SceneComposer, SceneSelections, CastSelection, DynamicSelection};

pub struct EngineServiceImpl {
    composer: Arc<SceneComposer>,
    state_manager: Arc<EngineStateManager>,
    session_store: Arc<SessionStore>,
    providers: Arc<EngineProviders>,
}

impl EngineServiceImpl {
    pub fn new(
        composer: Arc<SceneComposer>,
        state_manager: Arc<EngineStateManager>,
        session_store: Arc<SessionStore>,
        providers: Arc<EngineProviders>,
    ) -> Self {
        Self { composer, state_manager, session_store, providers }
    }

}

/// Build an EngineEvent with a fresh UUIDv7 and timestamp.
/// Free function (not a method) so it's usable inside `tokio::spawn` closures.
fn make_event(session_id: &str, turn: Option<u32>, payload: engine_event::Payload) -> EngineEvent {
    EngineEvent {
        event_id: Uuid::now_v7().to_string(),
        session_id: session_id.to_string(),
        turn,
        timestamp: Utc::now().to_rfc3339(),
        payload: Some(payload),
    }
}

#[tonic::async_trait]
impl StorytellerEngine for EngineServiceImpl {
    type ComposeSceneStream = ReceiverStream<Result<EngineEvent, Status>>;
    type SubmitInputStream = ReceiverStream<Result<EngineEvent, Status>>;
    type ResumeSessionStream = ReceiverStream<Result<EngineEvent, Status>>;
    type GetSessionEventsStream = ReceiverStream<Result<StoredEvent, Status>>;
    type StreamLogsStream = ReceiverStream<Result<LogEntry, Status>>;

    async fn compose_scene(
        &self,
        request: Request<ComposeSceneRequest>,
    ) -> Result<Response<Self::ComposeSceneStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(32);

        let composer = self.composer.clone();
        let state_manager = self.state_manager.clone();
        let session_store = self.session_store.clone();
        let providers = self.providers.clone();

        tokio::spawn(async move {
            // This is the core pipeline — extracted from commands.rs
            // Each phase sends an event, persists it, and updates state.
            // Errors are sent as ErrorOccurred events.

            let session_id = session_store.create_session()
                .unwrap_or_else(|_| Uuid::now_v7().to_string());

            // Phase 1: Compose scene
            let _ = tx.send(Ok(make_event(
                &session_id, Some(0),
                engine_event::Payload::PhaseStarted(PhaseStarted {
                    phase: "composition".to_string(),
                }),
            ))).await;

            let selections = SceneSelections {
                genre_id: req.genre_id,
                profile_id: req.profile_id,
                cast: req.cast.into_iter().map(|c| CastSelection {
                    archetype_id: c.archetype_id,
                    name: c.name,
                    role: c.role,
                }).collect(),
                dynamics: req.dynamics.into_iter().map(|d| DynamicSelection {
                    dynamic_id: d.dynamic_id,
                    cast_index_a: d.cast_index_a as usize,
                    cast_index_b: d.cast_index_b as usize,
                }).collect(),
                title_override: req.title_override,
                setting_override: req.setting_override,
                seed: req.seed,
            };

            let composed = match composer.compose(&selections) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Ok(make_event(
                        &session_id, Some(0),
                        engine_event::Payload::Error(ErrorOccurred {
                            phase: "composition".to_string(),
                            message: e,
                        }),
                    ))).await;
                    return;
                }
            };

            // Goal intersection
            let goals = composer.intersect_goals(&selections, &composed);

            let cast_names: Vec<String> = composed.characters.iter()
                .map(|c| c.name.clone()).collect();

            let _ = tx.send(Ok(make_event(
                &session_id, Some(0),
                engine_event::Payload::SceneComposed(SceneComposed {
                    title: composed.scene.title.clone(),
                    setting_description: composed.scene.setting.description.clone(),
                    cast_names: cast_names.clone(),
                    composition_json: serde_json::to_string(&composed)
                        .unwrap_or_default(),
                }),
            ))).await;

            // Persist composition
            let composition_value = serde_json::json!({
                "selections": selections,
                "scene": composed.scene,
                "characters": composed.characters,
                "goals": goals,
                "intentions": null, // filled after generation
            });

            if let Err(e) = session_store.composition.write(&session_id, &composition_value) {
                tracing::error!("Failed to persist composition: {e}");
            }

            // Phase 2: Goal generation event
            // (Goal intersection already done above — emit the event)
            let _ = tx.send(Ok(make_event(
                &session_id, Some(0),
                engine_event::Payload::Goals(GoalsGenerated {
                    scene_goals: goals.scene_goals.iter()
                        .map(|g| format!("{} ({})", g.goal_id, g.goal_type))
                        .collect(),
                    character_goals: vec![], // simplified for now
                    scene_direction: None,
                    character_drives: None,
                    player_context: None,
                    timing_ms: 0,
                }),
            ))).await;

            // Phase 3: Intention generation (LLM call)
            // Phase 4: Opening narration (LLM call)
            // These require wiring the actual LLM providers from EngineProviders.
            // For now, emit placeholder events — the LLM integration will be
            // wired when the full turn pipeline is extracted from commands.rs.

            let _ = tx.send(Ok(make_event(
                &session_id, Some(0),
                engine_event::Payload::PhaseStarted(PhaseStarted {
                    phase: "narrator".to_string(),
                }),
            ))).await;

            // TODO: Wire actual narrator LLM call here.
            // For now, emit a placeholder narrator complete event.
            let _ = tx.send(Ok(make_event(
                &session_id, Some(0),
                engine_event::Payload::NarratorComplete(NarratorComplete {
                    prose: "[Narrator LLM integration pending — scene composed successfully]".to_string(),
                    generation_ms: 0,
                }),
            ))).await;

            // Register session in state manager
            let composition = Composition {
                scene: serde_json::to_value(&composed.scene).unwrap_or_default(),
                characters: composed.characters.iter()
                    .map(|c| serde_json::to_value(c).unwrap_or_default())
                    .collect(),
                goals: Some(serde_json::to_value(&goals).unwrap_or_default()),
                intentions: None,
                selections: serde_json::to_value(&selections).unwrap_or_default(),
            };
            state_manager.create_session(&session_id, composition);

            // Turn complete
            let _ = tx.send(Ok(make_event(
                &session_id, Some(0),
                engine_event::Payload::TurnComplete(TurnComplete {
                    turn: 0,
                    total_ms: 0,
                }),
            ))).await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    // --- Stub implementations for remaining RPCs ---
    // These will be implemented in subsequent tasks.

    async fn submit_input(
        &self,
        _request: Request<SubmitInputRequest>,
    ) -> Result<Response<Self::SubmitInputStream>, Status> {
        Err(Status::unimplemented("SubmitInput not yet implemented"))
    }

    async fn resume_session(
        &self,
        _request: Request<ResumeSessionRequest>,
    ) -> Result<Response<Self::ResumeSessionStream>, Status> {
        Err(Status::unimplemented("ResumeSession not yet implemented"))
    }

    async fn list_sessions(
        &self,
        _request: Request<()>,
    ) -> Result<Response<SessionList>, Status> {
        Err(Status::unimplemented("ListSessions not yet implemented"))
    }

    async fn get_scene_state(
        &self,
        _request: Request<GetSceneStateRequest>,
    ) -> Result<Response<SceneState>, Status> {
        Err(Status::unimplemented("GetSceneState not yet implemented"))
    }

    async fn check_llm_status(
        &self,
        _request: Request<()>,
    ) -> Result<Response<LlmStatus>, Status> {
        Err(Status::unimplemented("CheckLlmStatus not yet implemented"))
    }

    async fn get_prediction_history(
        &self,
        _request: Request<PredictionHistoryRequest>,
    ) -> Result<Response<PredictionHistoryResponse>, Status> {
        Err(Status::unimplemented("GetPredictionHistory not yet implemented"))
    }

    async fn get_session_events(
        &self,
        _request: Request<SessionEventsRequest>,
    ) -> Result<Response<Self::GetSessionEventsStream>, Status> {
        Err(Status::unimplemented("GetSessionEvents not yet implemented"))
    }

    async fn stream_logs(
        &self,
        _request: Request<LogFilter>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        Err(Status::unimplemented("StreamLogs not yet implemented"))
    }
}
```

- [ ] **Step 2: Update grpc/mod.rs**

```rust
pub mod composer_service;
pub mod engine_service;
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p storyteller-api --all-features
```

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-api/src/grpc/engine_service.rs \
    crates/storyteller-api/src/grpc/mod.rs
git commit -m "feat: implement ComposeScene gRPC with streaming events"
```

---

### Task 9: Server Startup and gRPC Wiring

Wire the gRPC services into a tonic server and create the `serve` entry point.

**Files:**
- Create: `crates/storyteller-api/src/server.rs`
- Modify: `crates/storyteller-cli/src/main.rs` (add `serve` subcommand)
- Modify: `crates/storyteller-cli/Cargo.toml` (add storyteller-composer dep)

**Context:**
- The server needs to: load descriptors via `SceneComposer`, construct `EngineProviders` (LLM connections), create `EngineStateManager`, create `SessionStore`, wire gRPC services, start listening
- Port: use an env var `STORYTELLER_GRPC_PORT` with default `50051`
- LLM providers constructed from env vars (see design spec "Model Configuration" section)
- The `serve` subcommand replaces the current stub `main.rs`

- [ ] **Step 1: Create server.rs**

```rust
//! gRPC server startup and configuration.

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

use storyteller_composer::SceneComposer;

use crate::engine::{EngineProviders, EngineStateManager};
use crate::grpc::composer_service::ComposerServiceImpl;
use crate::grpc::engine_service::EngineServiceImpl;
use crate::persistence::SessionStore;
use crate::proto::composer_service_server::ComposerServiceServer;
use crate::proto::storyteller_engine_server::StorytellerEngineServer;

/// Server configuration from environment variables.
#[derive(Debug)]
pub struct ServerConfig {
    pub grpc_port: u16,
    pub data_path: String,
    pub sessions_dir: String,
    pub narrator_model: String,
    pub decomposition_model: String,
    pub intent_model: String,
    pub ollama_url: String,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            grpc_port: std::env::var("STORYTELLER_GRPC_PORT")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(50051),
            data_path: std::env::var("STORYTELLER_DATA_PATH")
                .unwrap_or_else(|_| "../storyteller-data/training-data/descriptors".to_string()),
            sessions_dir: std::env::var("STORYTELLER_SESSIONS_DIR")
                .unwrap_or_else(|_| ".story/sessions".to_string()),
            narrator_model: std::env::var("STORYTELLER_NARRATOR_MODEL")
                .unwrap_or_else(|_| "qwen2.5:14b".to_string()),
            decomposition_model: std::env::var("STORYTELLER_DECOMPOSITION_MODEL")
                .unwrap_or_else(|_| "qwen2.5:3b-instruct".to_string()),
            intent_model: std::env::var("STORYTELLER_INTENT_MODEL")
                .unwrap_or_else(|_| "qwen2.5:3b-instruct".to_string()),
            ollama_url: std::env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
        }
    }
}

/// Start the gRPC server.
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading descriptors from {}", config.data_path);
    let composer = Arc::new(
        SceneComposer::load(std::path::Path::new(&config.data_path))
            .map_err(|e| format!("Failed to load descriptors: {e}"))?
    );

    let state_manager = Arc::new(EngineStateManager::new());

    let session_store = Arc::new(
        SessionStore::new(std::path::Path::new(&config.sessions_dir))
            .map_err(|e| format!("Failed to create session store: {e}"))?
    );

    // TODO: Construct real LLM providers from config.
    // For now, providers is a placeholder — LLM wiring is a follow-up.
    // The server can serve composer RPCs and ComposeScene (composition only)
    // without LLM providers.

    let addr: SocketAddr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    info!("Starting gRPC server on {addr}");

    let composer_service = ComposerServiceImpl::new(composer.clone());
    // Note: EngineServiceImpl requires EngineProviders which needs LLM.
    // We'll wire this incrementally — for now, start with just the composer service.

    Server::builder()
        .add_service(ComposerServiceServer::new(composer_service))
        // .add_service(StorytellerEngineServer::new(engine_service))  // after LLM wiring
        .serve(addr)
        .await?;

    Ok(())
}
```

- [ ] **Step 2: Update storyteller-cli main.rs with serve subcommand**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "storyteller-cli", about = "Storyteller engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gRPC engine server.
    Serve,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Load .env if available
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    match cli.command {
        Commands::Serve => {
            let config = storyteller_api::server::ServerConfig::from_env();
            storyteller_api::server::run_server(config).await?;
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Update storyteller-cli Cargo.toml**

Add dependencies:
```toml
storyteller-composer = { path = "../storyteller-composer", version = "=0.1.0" }
dotenvy = { workspace = true }
tracing-subscriber = { workspace = true }
```

- [ ] **Step 4: Wire server module into lib.rs**

Add `pub mod server;` to `crates/storyteller-api/src/lib.rs`.

- [ ] **Step 5: Verify server starts**

```bash
# Build
cargo build -p storyteller-cli

# Start server (Ctrl+C to stop)
STORYTELLER_DATA_PATH="$STORYTELLER_DATA_PATH/training-data/descriptors" \
    cargo run -p storyteller-cli -- serve
```

Expected: "Starting gRPC server on 0.0.0.0:50051"

- [ ] **Step 6: Test with grpcurl**

Note: Without `tonic-reflection` (added in a later phase), grpcurl needs proto file paths.

```bash
# Install grpcurl if needed: brew install grpcurl

# List genres (pass proto import paths)
grpcurl -plaintext \
    -import-path proto \
    -proto storyteller/v1/composer.proto \
    localhost:50051 storyteller.v1.ComposerService/ListGenres
```

Expected: JSON response with genre list.

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-api/src/server.rs crates/storyteller-cli/src/main.rs \
    crates/storyteller-cli/Cargo.toml crates/storyteller-api/src/lib.rs
git commit -m "feat: add gRPC server startup with serve subcommand"
```

---

### Task 10: Engine gRPC Service — Query RPCs

Implement the unary query RPCs: `ListSessions`, `GetSceneState`, `CheckLlmStatus`.

**Files:**
- Modify: `crates/storyteller-api/src/grpc/engine_service.rs`

**Context:**
- `ListSessions`: reads session directories, loads composition.json for metadata, counts turns
- `GetSceneState`: reads composition + runtime snapshot from EngineStateManager
- `CheckLlmStatus`: probes Ollama endpoints for model availability
- These replace stubs from Task 8

- [ ] **Step 1: Implement ListSessions**

Replace the stub in `engine_service.rs`:

```rust
async fn list_sessions(
    &self,
    _request: Request<Empty>,
) -> Result<Response<SessionList>, Status> {
    let session_ids = self.session_store.list_session_ids()
        .map_err(|e| Status::internal(format!("list sessions: {e}")))?;

    let mut summaries = Vec::new();
    for id in session_ids {
        // Try to load composition for metadata
        if let Ok(comp) = self.session_store.composition.read(&id) {
            let turn_count = self.session_store.turns.turn_count(&id)
                .unwrap_or(0) as u32;

            summaries.push(SessionSummary {
                session_id: id,
                genre: comp.get("selections")
                    .and_then(|s| s.get("genre_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown").to_string(),
                profile: comp.get("selections")
                    .and_then(|s| s.get("profile_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown").to_string(),
                title: comp.get("scene")
                    .and_then(|s| s.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled").to_string(),
                cast_names: comp.get("characters")
                    .and_then(|c| c.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
                        .map(|s| s.to_string())
                        .collect())
                    .unwrap_or_default(),
                turn_count,
                created_at: String::new(), // TODO: from directory mtime
            });
        }
    }

    Ok(Response::new(SessionList { sessions: summaries }))
}
```

- [ ] **Step 2: Implement GetSceneState**

```rust
async fn get_scene_state(
    &self,
    request: Request<GetSceneStateRequest>,
) -> Result<Response<SceneState>, Status> {
    let session_id = &request.get_ref().session_id;

    let composition = self.state_manager.get_composition(session_id)
        .ok_or_else(|| Status::not_found(format!("session {session_id} not found")))?;

    let snapshot = self.state_manager.get_runtime_snapshot(session_id);

    let characters: Vec<CharacterState> = composition.characters.iter()
        .filter_map(|c| {
            Some(CharacterState {
                entity_id: c.get("entity_id")?.as_str()?.to_string(),
                name: c.get("name")?.as_str()?.to_string(),
                role: c.get("backstory").and_then(|b| b.as_str())
                    .unwrap_or("").to_string(),
                performance_notes: c.get("performance_notes")
                    .and_then(|p| p.as_str())
                    .unwrap_or("").to_string(),
            })
        })
        .collect();

    let current_turn = snapshot.map(|s| s.turn_count).unwrap_or(0);

    Ok(Response::new(SceneState {
        session_id: session_id.to_string(),
        title: composition.scene.get("title")
            .and_then(|t| t.as_str()).unwrap_or("").to_string(),
        setting_description: composition.scene.get("setting")
            .and_then(|s| s.get("description"))
            .and_then(|d| d.as_str()).unwrap_or("").to_string(),
        characters,
        scene_goals_json: composition.goals.as_ref()
            .map(|g| g.to_string()),
        intentions_json: composition.intentions.as_ref()
            .map(|i| i.to_string()),
        current_turn,
    }))
}
```

- [ ] **Step 3: Implement CheckLlmStatus**

```rust
async fn check_llm_status(
    &self,
    _request: Request<Empty>,
) -> Result<Response<LlmStatus>, Status> {
    // For now, report based on provider availability.
    // Full health checks (Ollama ping) can be added later.
    Ok(Response::new(LlmStatus {
        narrator_available: true, // placeholder
        narrator_model: String::new(),
        decomposition_available: self.providers.structured_llm.is_some(),
        decomposition_model: String::new(),
        intent_available: self.providers.intent_llm.is_some(),
        intent_model: String::new(),
        predictor_available: self.providers.predictor_available,
    }))
}
```

- [ ] **Step 4: Run tests and verify compilation**

```bash
cargo check -p storyteller-api --all-features
cargo test -p storyteller-api --all-features
```

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-api/src/grpc/engine_service.rs
git commit -m "feat: implement ListSessions, GetSceneState, CheckLlmStatus RPCs"
```

---

### Task 11: Integration Test — Server + gRPC Client

Write an integration test that starts the server, calls RPCs via tonic client stubs, and verifies responses.

**Files:**
- Create: `crates/storyteller-api/tests/grpc_integration.rs`

**Context:**
- tonic-build generated client stubs (we enabled `build_client(true)` in Task 4)
- Test starts server on a random port, connects a client, calls RPCs
- Requires `STORYTELLER_DATA_PATH` — skip if not set
- Focus on ComposerService RPCs (ListGenres, GetProfilesForGenre) since they don't need LLM

- [ ] **Step 1: Write integration test**

```rust
//! Integration tests for gRPC services.
//!
//! These tests start a real gRPC server and verify RPCs via tonic client stubs.
//! Requires STORYTELLER_DATA_PATH to be set.

use storyteller_api::proto::composer_service_client::ComposerServiceClient;
use storyteller_api::proto::composer_service_server::ComposerServiceServer;
use storyteller_api::proto::*;
use storyteller_api::grpc::composer_service::ComposerServiceImpl;

use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::{Channel, Server};

async fn start_test_server() -> Option<String> {
    let data_path = std::env::var("STORYTELLER_DATA_PATH").ok()?;
    let composer = Arc::new(
        storyteller_composer::SceneComposer::load(std::path::Path::new(&data_path))
            .expect("load descriptors"),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}");

    let service = ComposerServiceImpl::new(composer);

    tokio::spawn(async move {
        Server::builder()
            .add_service(ComposerServiceServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Some(url)
}

#[tokio::test]
async fn list_genres_returns_non_empty() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client.list_genres(()).await.unwrap();
    let genres = response.into_inner().genres;

    assert!(!genres.is_empty(), "should return at least one genre");
    assert!(
        genres.iter().any(|g| g.slug == "low_fantasy_folklore"),
        "should contain low_fantasy_folklore"
    );
}

#[tokio::test]
async fn profiles_for_genre_returns_results() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client.get_profiles_for_genre(GenreRequest {
        genre_id: "low_fantasy_folklore".to_string(),
    }).await.unwrap();

    let profiles = response.into_inner().profiles;
    assert!(!profiles.is_empty(), "should return profiles for genre");
}

#[tokio::test]
async fn invalid_genre_returns_empty_profiles() {
    let Some(url) = start_test_server().await else {
        eprintln!("STORYTELLER_DATA_PATH not set — skipping");
        return;
    };

    let channel = Channel::from_shared(url).unwrap().connect().await.unwrap();
    let mut client = ComposerServiceClient::new(channel);

    let response = client.get_profiles_for_genre(GenreRequest {
        genre_id: "nonexistent_genre".to_string(),
    }).await.unwrap();

    assert!(response.into_inner().profiles.is_empty());
}
```

- [ ] **Step 2: Add tokio-stream to dev-dependencies if needed**

In `crates/storyteller-api/Cargo.toml`:
```toml
[dev-dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
tempfile = { workspace = true }
tokio-stream = { workspace = true }
storyteller-composer = { path = "../storyteller-composer", version = "=0.1.0" }
```

- [ ] **Step 3: Run integration tests**

```bash
cargo test -p storyteller-api --test grpc_integration --all-features
```

Expected: 3 tests pass (or skip if `STORYTELLER_DATA_PATH` not set).

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-api/tests/grpc_integration.rs \
    crates/storyteller-api/Cargo.toml
git commit -m "test: add gRPC integration tests for composer service"
```

---

## Chunk 5: LLM Provider Wiring + Full ComposeScene

### Task 12: Wire LLM Providers into Server

Connect the actual Ollama LLM providers to the server, enabling real narrator rendering in `ComposeScene`.

**Files:**
- Modify: `crates/storyteller-api/src/engine/providers.rs`
- Modify: `crates/storyteller-api/src/server.rs`
- Modify: `crates/storyteller-api/Cargo.toml` (add storyteller-ml dep)

**Context:**
- `ExternalServerProvider` in `storyteller-engine/src/inference/external.rs` — Ollama HTTP provider implementing `LlmProvider`
- `OllamaStructuredProvider` in `storyteller-engine/src/inference/structured.rs` — structured output via Ollama
- `CharacterPredictor` in `storyteller-ml` — ONNX inference for character behavior prediction
- `PlutchikWestern` in `storyteller-core/src/grammars/` — emotional grammar
- Commands.rs constructs these in `setup_and_render_opening()` — reference lines ~1070-1120
- Current construction pattern:
  ```rust
  let llm = ExternalServerProvider::new(ollama_url, narrator_model, timeout, max_tokens);
  let structured = OllamaStructuredProvider::new(ollama_url, decomp_model, timeout);
  let intent_llm = ExternalServerProvider::new(ollama_url, intent_model, timeout, max_tokens);
  let predictor = CharacterPredictor::new(model_path);
  ```

- [ ] **Step 1: Expand EngineProviders with full provider set**

```rust
use std::sync::Arc;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::grammars::PlutchikWestern;
use storyteller_engine::inference::frame::CharacterPredictor;

/// Shared providers for LLM and ML inference.
#[derive(Clone)]
pub struct EngineProviders {
    pub narrator_llm: Arc<dyn LlmProvider>,
    pub structured_llm: Option<Arc<dyn StructuredLlmProvider>>,
    pub intent_llm: Option<Arc<dyn LlmProvider>>,
    pub predictor: Option<CharacterPredictor>,
    pub grammar: PlutchikWestern,
}
```

Note: Check if `StructuredLlmProvider` is in `storyteller_core::traits` or `storyteller_engine::inference::structured`. Import from wherever it's defined.

- [ ] **Step 2: Update server.rs to construct real providers**

```rust
// In run_server(), after loading composer:
use storyteller_engine::inference::external::{ExternalServerProvider, ExternalServerConfig};
use std::time::Duration;

let narrator_llm = Arc::new(ExternalServerProvider::new(ExternalServerConfig {
    base_url: config.ollama_url.clone(),
    model: config.narrator_model.clone(),
    timeout: Duration::from_secs(120),
}));

let intent_llm = Arc::new(ExternalServerProvider::new(ExternalServerConfig {
    base_url: config.ollama_url.clone(),
    model: config.intent_model.clone(),
    timeout: Duration::from_secs(60),
}));

let grammar = PlutchikWestern::default();

let providers = Arc::new(EngineProviders {
    narrator_llm,
    structured_llm: None,  // TODO: construct OllamaStructuredProvider
    intent_llm: Some(intent_llm),
    predictor: None,        // TODO: construct CharacterPredictor from model path
    grammar,
});
```

- [ ] **Step 3: Wire EngineService into the server**

Update `run_server()` to also add the EngineService:

```rust
let engine_service = EngineServiceImpl::new(
    composer.clone(),
    state_manager.clone(),
    session_store.clone(),
    providers.clone(),
);

Server::builder()
    .add_service(ComposerServiceServer::new(composer_service))
    .add_service(StorytellerEngineServer::new(engine_service))
    .serve(addr)
    .await?;
```

- [ ] **Step 4: Update ComposeScene to use real narrator**

In `engine_service.rs`, replace the placeholder narrator call with actual LLM rendering. Reference the narrator context assembly from `commands.rs::setup_and_render_opening()`.

This involves:
1. Building the narrator preamble (scene + characters + goals)
2. Calling `providers.narrator_llm.complete()` with the assembled context
3. Streaming the result as `NarratorComplete` event

The exact implementation will need to reference:
- `crates/storyteller-engine/src/agents/narrator.rs` — `NarratorAgent` and context assembly
- `crates/storyteller-engine/src/context/` — context assembly helpers
- `crates/storyteller-workshop/src-tauri/src/commands.rs` — the current wiring

- [ ] **Step 5: Verify end-to-end with grpcurl**

```bash
# Start server
cargo run -p storyteller-cli -- serve

# In another terminal, compose a scene
grpcurl -plaintext \
    -import-path proto \
    -proto storyteller/v1/engine.proto \
    -d '{
  "genre_id": "low_fantasy_folklore",
  "profile_id": "quiet_reunion",
  "cast": [
    {"archetype_id": "wandering_artist", "role": "protagonist"},
    {"archetype_id": "stoic_survivor", "role": "deuteragonist"}
  ],
  "seed": "42"
}' localhost:50051 storyteller.v1.StorytellerEngine/ComposeScene
```

Expected: streamed events including `SceneComposed` and `NarratorComplete` with actual prose.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-api/src/engine/providers.rs \
    crates/storyteller-api/src/server.rs \
    crates/storyteller-api/src/grpc/engine_service.rs \
    crates/storyteller-api/Cargo.toml
git commit -m "feat: wire LLM providers and real narrator into ComposeScene"
```

---

### Task 13: Engine gRPC Service — SubmitInput

Implement the `SubmitInput` server-streaming RPC — the full per-turn pipeline.

**Files:**
- Modify: `crates/storyteller-api/src/grpc/engine_service.rs`

**Context:**
- Source logic: `commands.rs::submit_input()` (lines ~559-1060)
- Turn pipeline phases: Decomposition → Prediction → Arbitration → Intent Synthesis → Context Assembly → Narrator
- Each phase emits an `EngineEvent`, persists to `events.jsonl`, updates `RuntimeSnapshot`
- At turn end, appends a `TurnEntry` to `turns.jsonl` with all event IDs from this turn
- `EngineStateManager.update_runtime_snapshot()` called at phase boundaries

**Pipeline extraction pattern:**

```rust
async fn submit_input(
    &self,
    request: Request<SubmitInputRequest>,
) -> Result<Response<Self::SubmitInputStream>, Status> {
    let req = request.into_inner();
    let session_id = req.session_id.clone();
    let input = req.input;

    // Verify session exists
    if !self.state_manager.has_session(&session_id) {
        return Err(Status::not_found("session not found"));
    }

    let (tx, rx) = mpsc::channel(32);
    // Clone what we need for the spawned task
    let state_manager = self.state_manager.clone();
    let session_store = self.session_store.clone();
    let providers = self.providers.clone();

    tokio::spawn(async move {
        let mut event_ids: Vec<String> = Vec::new();
        let snapshot = state_manager.get_runtime_snapshot(&session_id)
            .unwrap_or_default();
        let turn = snapshot.as_ref().map(|s| s.turn_count + 1).unwrap_or(1);

        // Phase 1: Event Decomposition
        // Extract from commands.rs decomposition phase
        // Emit DecompositionComplete event

        // Phase 2: ML Prediction
        // Extract from commands.rs prediction phase
        // Emit PredictionComplete event

        // Phase 3: Action Arbitration
        // Emit ArbitrationComplete event

        // Phase 4: Intent Synthesis
        // Emit IntentSynthesisComplete event

        // Phase 5: Context Assembly
        // Emit ContextAssembled event

        // Phase 6: Narrator Rendering
        // Emit NarratorComplete event

        // Update runtime snapshot
        state_manager.update_runtime_snapshot(&session_id, |snap| {
            let mut new = snap.clone();
            new.turn_count = turn;
            // Update journal, prediction_history, etc.
            new
        }).await;

        // Persist turn entry
        let turn_entry = crate::persistence::TurnEntry {
            turn,
            timestamp: chrono::Utc::now().to_rfc3339(),
            player_input: Some(input.clone()),
            event_ids,
        };
        let _ = session_store.turns.append(&session_id, &turn_entry);

        // Turn complete
        let _ = tx.send(Ok(make_event(
            &session_id, Some(turn),
            engine_event::Payload::TurnComplete(TurnComplete {
                turn, total_ms: 0,
            }),
        ))).await;
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}
```

The implementer should reference `commands.rs::submit_input()` for the exact orchestration of each phase. Key functions to call:
- `storyteller_engine::inference::event_decomposition::decompose_events()` — structured LLM
- `storyteller_engine::context::prediction::predict_character_behaviors()` — ML inference
- `storyteller_engine::systems::arbitration::check_action_possibility()` — rules engine
- `storyteller_engine::inference::intent_synthesis::synthesize_intents()` — LLM
- `storyteller_engine::agents::narrator::assemble_narrator_context()` — context builder
- `storyteller_engine::agents::narrator::NarratorAgent` — LLM rendering

- [ ] **Step 1: Implement the full pipeline with event emission**

Extract each phase from `commands.rs`, emit events via the channel, persist via `EventWriter`, collect event_ids.

- [ ] **Step 2: Test manually with grpcurl**

After composing a scene, submit input:
```bash
grpcurl -plaintext \
    -import-path proto \
    -proto storyteller/v1/engine.proto \
    -d '{"session_id": "<id>", "input": "I look around the room"}' \
    localhost:50051 storyteller.v1.StorytellerEngine/SubmitInput
```

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-api/src/grpc/engine_service.rs
git commit -m "feat: implement SubmitInput gRPC with full turn pipeline"
```

---

### Task 14: Engine gRPC Service — ResumeSession

Implement `ResumeSession` — reload a persisted session and stream state.

**Files:**
- Modify: `crates/storyteller-api/src/grpc/engine_service.rs`

**Context:**
- Source logic: `commands.rs::resume_session()` (lines ~332-557)
- Reads `composition.json` → hydrates `Composition` in state manager
- Reads `turns.jsonl` → reconstructs journal, sets turn_count
- Streams: `SceneComposed` event + `TurnComplete` for each existing turn + final state
- If no turns exist, runs opening narration (like ComposeScene)

- [ ] **Step 1: Implement ResumeSession**

```rust
async fn resume_session(
    &self,
    request: Request<ResumeSessionRequest>,
) -> Result<Response<Self::ResumeSessionStream>, Status> {
    let session_id = request.into_inner().session_id;
    let (tx, rx) = mpsc::channel(32);

    let state_manager = self.state_manager.clone();
    let session_store = self.session_store.clone();

    tokio::spawn(async move {
        // Load composition
        let comp = match session_store.composition.read(&session_id) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Ok(make_event(
                    &session_id, None,
                    engine_event::Payload::Error(ErrorOccurred {
                        phase: "resume".to_string(),
                        message: format!("load composition: {e}"),
                    }),
                ))).await;
                return;
            }
        };

        // Register in state manager
        let composition = Composition {
            scene: comp.get("scene").cloned().unwrap_or_default(),
            characters: comp.get("characters")
                .and_then(|c| c.as_array())
                .map(|a| a.to_vec())
                .unwrap_or_default(),
            goals: comp.get("goals").cloned(),
            intentions: comp.get("intentions").cloned(),
            selections: comp.get("selections").cloned().unwrap_or_default(),
        };

        // Emit SceneComposed
        let cast_names: Vec<String> = composition.characters.iter()
            .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
            .map(|s| s.to_string())
            .collect();

        let _ = tx.send(Ok(make_event(
            &session_id, None,
            engine_event::Payload::SceneComposed(SceneComposed {
                title: composition.scene.get("title")
                    .and_then(|t| t.as_str()).unwrap_or("").to_string(),
                setting_description: composition.scene.get("setting")
                    .and_then(|s| s.get("description"))
                    .and_then(|d| d.as_str()).unwrap_or("").to_string(),
                cast_names,
                composition_json: String::new(),
            }),
        ))).await;

        // Load turns and reconstruct state
        let turns = session_store.turns.read_all(&session_id)
            .unwrap_or_default();

        let turn_count = turns.len() as u32;

        state_manager.create_session(&session_id, composition);
        state_manager.update_runtime_snapshot(&session_id, |snap| {
            let mut new = snap.clone();
            new.turn_count = turn_count;
            new
        }).await;

        // Emit TurnComplete for each existing turn
        // (client uses these to reconstruct chat history)
        for turn in &turns {
            let _ = tx.send(Ok(make_event(
                &session_id, Some(turn.turn),
                engine_event::Payload::TurnComplete(TurnComplete {
                    turn: turn.turn,
                    total_ms: 0,
                }),
            ))).await;
        }
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}
```

- [ ] **Step 2: Test with grpcurl**

```bash
# Resume a previously created session
grpcurl -plaintext \
    -import-path proto \
    -proto storyteller/v1/engine.proto \
    -d '{"session_id": "<id>"}' \
    localhost:50051 storyteller.v1.StorytellerEngine/ResumeSession
```

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-api/src/grpc/engine_service.rs
git commit -m "feat: implement ResumeSession gRPC with turn history replay"
```

---

### Task 15: Full Workspace Verification

Final verification that everything compiles, tests pass, and the server runs.

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace check**

```bash
cargo make check
```

Or manually:
```bash
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt --check
cargo test --workspace --all-features
```

- [ ] **Step 2: Run gRPC integration tests**

```bash
cargo test -p storyteller-api --test grpc_integration --all-features
```

- [ ] **Step 3: Manual smoke test**

```bash
# Terminal 1: Start server
cargo run -p storyteller-cli -- serve

# Terminal 2: Test composer RPCs
grpcurl -plaintext \
    -import-path proto \
    -proto storyteller/v1/composer.proto \
    localhost:50051 storyteller.v1.ComposerService/ListGenres

# Test scene composition (if LLM available)
grpcurl -plaintext \
    -import-path proto \
    -proto storyteller/v1/engine.proto \
    -d '{
  "genre_id": "low_fantasy_folklore",
  "profile_id": "quiet_reunion",
  "cast": [
    {"archetype_id": "wandering_artist", "role": "protagonist"},
    {"archetype_id": "stoic_survivor", "role": "deuteragonist"}
  ]
}' localhost:50051 storyteller.v1.StorytellerEngine/ComposeScene
```

- [ ] **Step 4: Verify session persistence**

Check that `.story/sessions/<id>/` contains:
- `composition.json` — scene setup data
- `events.jsonl` — streamed events (if any)
- `turns.jsonl` — turn index (if turns were submitted)

- [ ] **Step 5: Commit any fixes**

```bash
git add -A && git commit -m "fix: address workspace verification issues"
```

---

## Deferred RPCs (Phase 2+)

The following RPCs remain as stubs returning `Status::unimplemented` after Phase 1:

- **`GetPredictionHistory`** — deferred until prediction pipeline is fully extracted from `commands.rs`
- **`GetSessionEvents`** — event replay from `events.jsonl`; straightforward to implement but lower priority than core gameplay RPCs
- **`StreamLogs`** — requires wiring a tracing subscriber to gRPC; deferred to Phase 2 when the client library is built

These will be implemented in Phase 2 alongside the `storyteller-client` crate.

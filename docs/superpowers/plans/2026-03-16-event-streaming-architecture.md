# Event Streaming Architecture Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add gameplay streaming channel, narrator prose streaming, turn lifecycle formalization, and async integration points to the storyteller workshop.

**Architecture:** Bottom-up infrastructure first (structural changes → plumbing → features). The Bevy turn pipeline collapses from 7 to 5 variants with an internal enrichment sub-pipeline. A new `workshop:gameplay` Tauri event channel carries player-facing events. Narrator output streams as sentence-batched `NarratorProse` chunks. A `directives.jsonl` store provides the integration surface for future async agents.

**Tech Stack:** Rust (Bevy ECS, tonic/prost, tokio), Protocol Buffers, TypeScript/Svelte 5, Tauri event system, Ollama HTTP streaming API.

**Spec:** `docs/superpowers/specs/2026-03-16-event-streaming-architecture-design.md`

---

## Chunk 1: Structural Changes (Layer 1)

Internal refactoring and new type definitions. No behavior change visible to the user.

### Task 1: Collapse TurnCycleStage enum (7 → 5 variants)

**Files:**
- Modify: `crates/storyteller-core/src/types/turn_cycle.rs`

- [ ] **Step 1: Update the `TurnCycleStage` enum**

Remove `Classifying`, `Predicting`, `Resolving`. Add `Enriching`. Update the docstring to remove the stale "Eight variants" claim. The enum becomes:

```rust
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum TurnCycleStage {
    #[default]
    AwaitingInput,
    CommittingPrevious,
    /// All enrichment work: event classification, ML prediction,
    /// game system arbitration, intent synthesis. Internal sub-pipeline
    /// managed by EnrichmentPhase.
    Enriching,
    AssemblingContext,
    Rendering,
}
```

Update `next()`:

```rust
impl TurnCycleStage {
    pub fn next(self) -> Self {
        match self {
            Self::AwaitingInput => Self::CommittingPrevious,
            Self::CommittingPrevious => Self::Enriching,
            Self::Enriching => Self::AssemblingContext,
            Self::AssemblingContext => Self::Rendering,
            Self::Rendering => Self::AwaitingInput,
        }
    }
}
```

- [ ] **Step 2: Add `EnrichmentPhase` enum**

In the same file, below `TurnCycleStage`:

```rust
/// Sub-pipeline phases within the Enriching stage.
///
/// Managed by the enrichment system internally — not visible to the
/// top-level Bevy schedule. Extensible: new phases slot in without
/// changing TurnCycleStage.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EnrichmentPhase {
    #[default]
    EventClassification,
    BehaviorPrediction,
    GameSystemArbitration,
    IntentSynthesis,
    Complete,
}

impl EnrichmentPhase {
    /// Advance to the next phase. `Complete` stays at `Complete`.
    pub fn next(self) -> Self {
        match self {
            Self::EventClassification => Self::BehaviorPrediction,
            Self::BehaviorPrediction => Self::GameSystemArbitration,
            Self::GameSystemArbitration => Self::IntentSynthesis,
            Self::IntentSynthesis => Self::Complete,
            Self::Complete => Self::Complete,
        }
    }

    /// Whether the enrichment sub-pipeline is complete.
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }
}
```

- [ ] **Step 3: Update tests**

Replace existing stage transition tests to reflect the new 5-variant cycle:

```rust
#[test]
fn next_cycles_through_all_stages() {
    let mut stage = TurnCycleStage::AwaitingInput;
    let expected = [
        TurnCycleStage::CommittingPrevious,
        TurnCycleStage::Enriching,
        TurnCycleStage::AssemblingContext,
        TurnCycleStage::Rendering,
        TurnCycleStage::AwaitingInput,
    ];
    for &exp in &expected {
        stage = stage.next();
        assert_eq!(stage, exp);
    }
}

#[test]
fn enriching_precedes_assembling_context() {
    assert_eq!(
        TurnCycleStage::Enriching.next(),
        TurnCycleStage::AssemblingContext
    );
}

#[test]
fn enrichment_phase_cycles_to_complete() {
    let mut phase = EnrichmentPhase::default();
    let expected = [
        EnrichmentPhase::BehaviorPrediction,
        EnrichmentPhase::GameSystemArbitration,
        EnrichmentPhase::IntentSynthesis,
        EnrichmentPhase::Complete,
    ];
    for &exp in &expected {
        phase = phase.next();
        assert_eq!(phase, exp);
    }
    assert!(phase.is_complete());
}

#[test]
fn enrichment_complete_stays_complete() {
    assert_eq!(EnrichmentPhase::Complete.next(), EnrichmentPhase::Complete);
}

#[test]
fn serde_roundtrip_enriching() {
    let stage = TurnCycleStage::Enriching;
    let json = serde_json::to_string(&stage).expect("serialize");
    let deserialized: TurnCycleStage = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(stage, deserialized);
}
```

Remove tests that reference `Classifying`, `Predicting`, `Resolving`. Keep the `committing_previous_precedes_*` test updated to check `Enriching` instead of `Classifying`. Keep `entity_category_all_variants_distinct`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p storyteller-core -- turn_cycle`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-core/src/types/turn_cycle.rs
git commit -m "refactor: collapse TurnCycleStage to 5 variants with EnrichmentPhase sub-pipeline"
```

### Task 2: Add EnrichmentState resource and collapse Bevy systems

**Files:**
- Modify: `crates/storyteller-engine/src/components/turn.rs`
- Modify: `crates/storyteller-engine/src/systems/turn_cycle.rs`
- Modify: `crates/storyteller-engine/src/plugin.rs`

- [ ] **Step 1: Add `EnrichmentState` Bevy Resource**

In `components/turn.rs`, add:

```rust
use storyteller_core::types::turn_cycle::EnrichmentPhase;

/// Tracks the current phase within the Enriching stage.
/// Reset to default (EventClassification) when Enriching begins.
#[derive(Debug, Default, Resource)]
pub struct EnrichmentState(pub EnrichmentPhase);
```

- [ ] **Step 2: Collapse `TurnCycleSets` from 7 to 5 variants**

In `systems/turn_cycle.rs`, update the SystemSet enum:

```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TurnCycleSets {
    Input,
    CommittingPrevious,
    Enrichment,
    ContextAssembly,
    Rendering,
}
```

Remove `Classification`, `Prediction`, `Resolution`.

- [ ] **Step 3: Create unified `enrichment_system`**

In `systems/turn_cycle.rs`, create a single `enrichment_system` that replaces `classify_system`, `predict_system`, and `resolve_system`. It manages the `EnrichmentState` sub-pipeline internally:

```rust
pub fn enrichment_system(
    mut stage: ResMut<ActiveTurnStage>,
    mut enrichment: ResMut<EnrichmentState>,
    mut turn_ctx: ResMut<TurnContext>,
    predictor: Option<Res<PredictorResource>>,
    scene_res: Option<Res<SceneResource>>,
    grammar_res: Option<Res<GrammarResource>>,
) {
    match enrichment.0 {
        EnrichmentPhase::EventClassification => {
            // Currently a no-op (DistilBERT removed).
            // Future: wired to structured LLM event classification.
            enrichment.0 = enrichment.0.next();
        }
        EnrichmentPhase::BehaviorPrediction => {
            // Existing predict_system logic
            if let (Some(predictor), Some(scene_res), Some(grammar_res)) =
                (predictor.as_ref(), scene_res.as_ref(), grammar_res.as_ref())
            {
                // ... existing prediction code from predict_system ...
            }
            enrichment.0 = enrichment.0.next();
        }
        EnrichmentPhase::GameSystemArbitration => {
            // Existing resolve_system logic
            if let Some(ref input) = turn_ctx.player_input {
                let result = check_action_possibility(input, &[], &CapabilityLexicon::new(), None);
                turn_ctx.arbitration = Some(result);
            }
            // ... existing resolver_output construction ...
            enrichment.0 = enrichment.0.next();
        }
        EnrichmentPhase::IntentSynthesis => {
            // Intent synthesis placeholder — currently done server-side.
            enrichment.0 = enrichment.0.next();
        }
        EnrichmentPhase::Complete => {
            // Reset enrichment state for next turn
            enrichment.0 = EnrichmentPhase::default();
            // Advance top-level stage
            stage.0 = stage.0.next();
        }
    }
}
```

Move the actual logic from the existing `predict_system` (lines ~86-135) and `resolve_system` (lines ~177-213) into the appropriate match arms. Remove the standalone `classify_system`, `predict_system`, and `resolve_system` functions.

- [ ] **Step 4: Update plugin SystemSet configuration**

In `plugin.rs`, update `configure_sets` and `add_systems`:

```rust
app.configure_sets(
    Update,
    (
        TurnCycleSets::Input,
        TurnCycleSets::CommittingPrevious.after(TurnCycleSets::Input),
        TurnCycleSets::Enrichment.after(TurnCycleSets::CommittingPrevious),
        TurnCycleSets::ContextAssembly.after(TurnCycleSets::Enrichment),
        TurnCycleSets::Rendering.after(TurnCycleSets::ContextAssembly),
    ),
);

app.add_systems(
    Update,
    (
        commit_previous_system
            .run_if(in_stage(TurnCycleStage::CommittingPrevious))
            .in_set(TurnCycleSets::CommittingPrevious),
        enrichment_system
            .run_if(in_stage(TurnCycleStage::Enriching))
            .in_set(TurnCycleSets::Enrichment),
        assemble_context_system
            .run_if(in_stage(TurnCycleStage::AssemblingContext))
            .in_set(TurnCycleSets::ContextAssembly),
        rendering_system
            .run_if(in_stage(TurnCycleStage::Rendering))
            .in_set(TurnCycleSets::Rendering),
    ),
);
```

Register `EnrichmentState` as a resource:

```rust
app.init_resource::<EnrichmentState>();
```

- [ ] **Step 5: Update turn_cycle.rs tests**

Update all tests that reference `TurnCycleStage::Classifying`, `Predicting`, `Resolving` to use `TurnCycleStage::Enriching`. Update the full pipeline test to use the new `enrichment_system`. Update any system ordering tests.

- [ ] **Step 6: Update rendering.rs tests**

Review tests in `rendering.rs` that set initial stage — these may reference removed variants. Update to use `Enriching` or `Rendering` as appropriate.

- [ ] **Step 7: Run tests**

Run: `cargo test -p storyteller-engine`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/storyteller-engine/src/components/turn.rs \
       crates/storyteller-engine/src/systems/turn_cycle.rs \
       crates/storyteller-engine/src/plugin.rs
git commit -m "refactor: collapse Bevy pipeline to 5 stages with enrichment sub-pipeline"
```

### Task 3: Add `stream_complete()` to LlmProvider trait

**Files:**
- Modify: `crates/storyteller-core/src/traits/llm.rs`

- [ ] **Step 1: Add newtype wrappers and `stream_complete()` method**

```rust
use tokio::sync::mpsc;

/// Newtype for the receiving end of a narrator token stream.
/// Bounded channel (capacity 64) per project convention.
#[derive(Debug)]
pub struct NarratorTokenStream(pub mpsc::Receiver<String>);

/// Newtype for the sending end of a narrator token stream.
#[derive(Debug, Clone)]
pub struct NarratorTokenSender(pub mpsc::Sender<String>);

/// Create a bounded narrator token channel pair.
pub fn narrator_token_channel() -> (NarratorTokenSender, NarratorTokenStream) {
    let (tx, rx) = mpsc::channel(64);
    (NarratorTokenSender(tx), NarratorTokenStream(rx))
}
```

Add `stream_complete()` to the `LlmProvider` trait with a default implementation:

```rust
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync + std::fmt::Debug {
    async fn complete(&self, request: CompletionRequest) -> StorytellerResult<CompletionResponse>;

    /// Stream completion tokens. Default implementation calls `complete()`
    /// and sends the full response as a single chunk.
    async fn stream_complete(&self, request: CompletionRequest) -> StorytellerResult<NarratorTokenStream> {
        let response = self.complete(request).await?;
        let (sender, receiver) = narrator_token_channel();
        // Spawn a task to send the complete response as one chunk.
        // The sender is moved into the task and dropped on completion.
        tokio::spawn(async move {
            let _ = sender.0.send(response.content).await;
        });
        Ok(receiver)
    }
}
```

- [ ] **Step 2: Export new types from `crates/storyteller-core/src/traits/mod.rs`**

Ensure `NarratorTokenStream`, `NarratorTokenSender`, and `narrator_token_channel` are publicly accessible. Check how `llm.rs` is currently re-exported and add the new types.

- [ ] **Step 3: Run check**

Run: `cargo check -p storyteller-core --all-features`
Expected: Compiles without errors. (No tests needed for trait definition — tested via implementors.)

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-core/src/traits/llm.rs crates/storyteller-core/src/traits/mod.rs
git commit -m "feat: add stream_complete() to LlmProvider with NarratorTokenStream newtypes"
```

### Task 4: Add proto messages for gameplay events

**Files:**
- Modify: `proto/storyteller/v1/engine.proto`

- [ ] **Step 1: Add new message types**

After the existing `NarratorToken` message (line 143), add:

```protobuf
message NarratorProse {
  string chunk = 1;
  uint32 turn = 2;
}

message SceneReady {
  string scene_id = 1;
  string title = 2;
  string setting_summary = 3;
  repeated string cast_names = 4;
  string player_character = 5;
  optional string player_intent = 6;
}

message InputReceived {
  uint32 turn = 1;
}

message ProcessingUpdate {
  string phase = 1;
}
```

Add `ready_for_input` to `TurnComplete`:

```protobuf
message TurnComplete { uint32 turn = 1; uint64 total_ms = 2; bool ready_for_input = 3; }
```

Add player character fields to `ComposeSceneRequest`:

```protobuf
message ComposeSceneRequest {
  string genre_id = 1;
  string profile_id = 2;
  repeated CastMember cast = 3;
  repeated DynamicPairing dynamics = 4;
  optional string title_override = 5;
  optional string setting_override = 6;
  optional uint64 seed = 7;
  optional PlayerCharacter player_character = 8;
}

message PlayerCharacter {
  string name = 1;
  optional string age = 2;
  optional string gender_presentation = 3;
  optional string intent = 4;
}
```

Add new variants to `EngineEvent.payload` oneof:

```protobuf
oneof payload {
    // ... existing variants 10-20, 30 ...
    NarratorProse narrator_prose = 21;
    SceneReady scene_ready = 22;
    InputReceived input_received = 23;
    ProcessingUpdate processing_update = 24;
}
```

- [ ] **Step 2: Run build to verify proto compilation**

Run: `cargo check -p storyteller-server --all-features`
Expected: Compiles. Tonic-build generates updated Rust types from proto.

- [ ] **Step 3: Commit**

```bash
git add proto/storyteller/v1/engine.proto
git commit -m "feat: add gameplay event proto messages (NarratorProse, SceneReady, InputReceived, ProcessingUpdate, PlayerCharacter)"
```

### Task 5: Add DirectiveStore

**Files:**
- Create: `crates/storyteller-server/src/persistence/directives.rs`
- Modify: `crates/storyteller-server/src/persistence/mod.rs`
- Modify: `crates/storyteller-server/src/persistence/session_store.rs`

- [ ] **Step 1: Create `directives.rs` with `DirectiveEntry` and `DirectiveStore`**

Follow the same pattern as `turns.rs` and `events.rs`:

```rust
//! Append-only directive store for async agent outputs.
//!
//! Writers: future dramaturge (Tier C.1), world agent (Tier C.2).
//! Reader: context assembly at assembly time.

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// A directive entry from an async agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveEntry {
    pub id: String,
    pub agency: String,
    #[serde(rename = "type")]
    pub directive_type: String,
    pub applicable_turns: Vec<u32>,
    pub based_on_turns: Vec<u32>,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

/// Append-only store for directives in `directives.jsonl`.
///
/// Follows the same pattern as `TurnWriter` and `EventWriter`:
/// takes `base_dir` at construction, accepts `session_id` per method call.
#[derive(Debug, Clone)]
pub struct DirectiveStore {
    base_dir: PathBuf,
}

impl DirectiveStore {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    fn path_for(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id).join("directives.jsonl")
    }

    /// Append a directive entry.
    pub fn append(&self, session_id: &str, entry: &DirectiveEntry) -> std::io::Result<()> {
        let path = self.path_for(session_id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let json = serde_json::to_string(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{json}")
    }

    /// Read all directives for a session.
    fn read_all(&self, session_id: &str) -> std::io::Result<Vec<DirectiveEntry>> {
        let path = self.path_for(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: DirectiveEntry = serde_json::from_str(&line)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            entries.push(entry);
        }
        Ok(entries)
    }

    /// Latest directive by agency.
    pub fn latest_by_agency(&self, session_id: &str, agency: &str) -> std::io::Result<Option<DirectiveEntry>> {
        let entries = self.read_all(session_id)?;
        Ok(entries.into_iter().rev().find(|e| e.agency == agency))
    }

    /// All directives applicable to a given turn.
    pub fn applicable_for_turn(&self, session_id: &str, turn: u32) -> std::io::Result<Vec<DirectiveEntry>> {
        let entries = self.read_all(session_id)?;
        Ok(entries
            .into_iter()
            .filter(|e| e.applicable_turns.contains(&turn))
            .collect())
    }

    /// Last N directives across all agencies.
    pub fn last_n(&self, session_id: &str, n: usize) -> std::io::Result<Vec<DirectiveEntry>> {
        let entries = self.read_all(session_id)?;
        let start = entries.len().saturating_sub(n);
        Ok(entries[start..].to_vec())
    }
}
```

- [ ] **Step 2: Add inline tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_entry(agency: &str, turns: Vec<u32>) -> DirectiveEntry {
        DirectiveEntry {
            id: uuid::Uuid::new_v4().to_string(),
            agency: agency.to_string(),
            directive_type: "test".to_string(),
            applicable_turns: turns,
            based_on_turns: vec![1],
            payload: serde_json::json!({"note": "test"}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn setup_store() -> (TempDir, DirectiveStore, String) {
        let dir = TempDir::new().unwrap();
        let store = DirectiveStore::new(dir.path());
        let session_id = "test-session";
        // Create session directory (normally done by SessionStore)
        std::fs::create_dir_all(dir.path().join(session_id)).unwrap();
        (dir, store, session_id.to_string())
    }

    #[test]
    fn append_and_read_all() {
        let (_dir, store, sid) = setup_store();
        let entry = make_entry("dramaturge", vec![3, 4]);
        store.append(&sid, &entry).unwrap();
        let all = store.read_all(&sid).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].agency, "dramaturge");
    }

    #[test]
    fn latest_by_agency() {
        let (_dir, store, sid) = setup_store();
        store.append(&sid, &make_entry("dramaturge", vec![1])).unwrap();
        store.append(&sid, &make_entry("world_agent", vec![2])).unwrap();
        store.append(&sid, &make_entry("dramaturge", vec![3])).unwrap();
        let latest = store.latest_by_agency(&sid, "dramaturge").unwrap().unwrap();
        assert_eq!(latest.applicable_turns, vec![3]);
    }

    #[test]
    fn applicable_for_turn() {
        let (_dir, store, sid) = setup_store();
        store.append(&sid, &make_entry("dramaturge", vec![2, 3])).unwrap();
        store.append(&sid, &make_entry("world_agent", vec![3, 4])).unwrap();
        let turn3 = store.applicable_for_turn(&sid, 3).unwrap();
        assert_eq!(turn3.len(), 2);
        let turn4 = store.applicable_for_turn(&sid, 4).unwrap();
        assert_eq!(turn4.len(), 1);
    }

    #[test]
    fn empty_store_returns_empty() {
        let (_dir, store, sid) = setup_store();
        assert!(store.read_all(&sid).unwrap().is_empty());
        assert!(store.latest_by_agency(&sid, "dramaturge").unwrap().is_none());
        assert!(store.applicable_for_turn(&sid, 1).unwrap().is_empty());
    }

    #[test]
    fn last_n() {
        let (_dir, store, sid) = setup_store();
        for i in 0..5 {
            store.append(&sid, &make_entry("dramaturge", vec![i])).unwrap();
        }
        let last2 = store.last_n(&sid, 2).unwrap();
        assert_eq!(last2.len(), 2);
        assert_eq!(last2[0].applicable_turns, vec![3]);
        assert_eq!(last2[1].applicable_turns, vec![4]);
    }
}
```

- [ ] **Step 3: Export from `mod.rs` and add to `SessionStore`**

In `persistence/mod.rs`, add:

```rust
pub mod directives;
pub use directives::{DirectiveEntry, DirectiveStore};
```

In `persistence/session_store.rs`, add a `directives` field to `SessionStore`. `DirectiveStore` follows the same `base_dir` + `session_id`-per-call pattern as other writers:

```rust
pub struct SessionStore {
    base_dir: PathBuf,
    pub composition: CompositionWriter,
    pub events: EventWriter,
    pub turns: TurnWriter,
    pub directives: DirectiveStore,
}
```

Update `SessionStore::new()` to initialize `DirectiveStore` with the same `base_dir`:

```rust
Ok(Self {
    base_dir: base_dir.to_path_buf(),
    composition: CompositionWriter::new(base_dir),
    events: EventWriter::new(base_dir),
    turns: TurnWriter::new(base_dir),
    directives: DirectiveStore::new(base_dir),
})
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p storyteller-server -- directives`
Expected: All directive store tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-server/src/persistence/directives.rs \
       crates/storyteller-server/src/persistence/mod.rs \
       crates/storyteller-server/src/persistence/session_store.rs
git commit -m "feat: add DirectiveStore for async agent integration (directives.jsonl)"
```

### Task 6: Add GameplayEvent types for frontend

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/types.rs`
- Modify: `crates/storyteller-workshop/src/lib/types.ts`

- [ ] **Step 1: Add Rust GameplayEvent type with ts-rs export**

In `types.rs`, add the `GameplayEvent` discriminated union following the existing pattern used by debug events:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind")]
pub enum GameplayEvent {
    SceneReady {
        scene_id: String,
        title: String,
        setting_summary: String,
        cast_names: Vec<String>,
        player_character: String,
        player_intent: Option<String>,
    },
    InputReceived {
        turn: u32,
    },
    ProcessingUpdate {
        phase: String,
    },
    NarratorProse {
        chunk: String,
        turn: u32,
    },
    NarratorComplete {
        prose: String,
        turn: u32,
    },
    TurnComplete {
        turn: u32,
        ready_for_input: bool,
    },
}
```

Add the channel constant:

```rust
pub const GAMEPLAY_CHANNEL: &str = "workshop:gameplay";
```

- [ ] **Step 2: Add TypeScript `GameplayEvent` type**

In `types.ts`, add matching TypeScript types:

```typescript
export const GAMEPLAY_CHANNEL = "workshop:gameplay";

export type GameplayEvent =
  | { kind: "SceneReady"; scene_id: string; title: string; setting_summary: string; cast_names: string[]; player_character: string; player_intent: string | null }
  | { kind: "InputReceived"; turn: number }
  | { kind: "ProcessingUpdate"; phase: string }
  | { kind: "NarratorProse"; chunk: string; turn: number }
  | { kind: "NarratorComplete"; prose: string; turn: number }
  | { kind: "TurnComplete"; turn: number; ready_for_input: boolean };
```

- [ ] **Step 3: Run type check**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: Type check passes.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/types.rs \
       crates/storyteller-workshop/src/lib/types.ts
git commit -m "feat: add GameplayEvent discriminated union (Rust + TypeScript)"
```

### Task 7: Verify Layer 1

- [ ] **Step 1: Full workspace check**

Run: `cargo check --all-features`
Expected: Compiles. There may be warnings about unused imports from the removed systems — fix any that appear.

- [ ] **Step 2: Full test suite**

Run: `cargo test --all-features`
Expected: All tests pass. Fix any failures from the pipeline collapse.

- [ ] **Step 3: Frontend check**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: Type check passes.

- [ ] **Step 4: Fix any issues and commit**

```bash
git add -u
git commit -m "chore: fix warnings and test adjustments after Layer 1 structural changes"
```

---

## Chunk 2: Plumbing (Layer 2)

Wiring infrastructure together. No new visible features yet, but the channels and streaming paths are connected.

### Task 8: Wire `workshop:gameplay` channel emission in Tauri commands

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Add gameplay channel emission alongside existing debug channel**

Import the new types:

```rust
use crate::types::{GameplayEvent, GAMEPLAY_CHANNEL};
```

In the `compose_scene` command, gameplay events are emitted **inside the `while let Some(event) = stream.message()` loop**, keyed off the `engine_event::Payload` variant. This follows the same pattern already used for debug events. Specifically:

- When `SceneComposed` payload is received (where the debug `SceneComposed` event is currently emitted), also emit `SceneReady` on gameplay:

```rust
// Inside the stream consumption loop, in the SceneComposed match arm:
Some(engine_event::Payload::SceneComposed(composed)) => {
    // Existing debug emission...
    app.emit(DEBUG_CHANNEL, &debug_event)?;

    // NEW: gameplay emission
    app.emit(GAMEPLAY_CHANNEL, &GameplayEvent::SceneReady {
        scene_id: session_id.clone(),
        title: composed.title.clone(),
        setting_summary: composed.setting_description.clone(),
        cast_names: composed.cast_names.clone(),
        player_character: String::new(), // populated in Task 13
        player_intent: None, // populated in Task 13
    })?;
}
```

- When `NarratorProse` payload is received (new variant from Task 4), emit on gameplay:

```rust
Some(engine_event::Payload::NarratorProse(prose)) => {
    app.emit(GAMEPLAY_CHANNEL, &GameplayEvent::NarratorProse {
        chunk: prose.chunk.clone(),
        turn: prose.turn,
    })?;
}
```

Similarly for `NarratorComplete`, `TurnComplete`, `ProcessingUpdate`, and `InputReceived` — each maps from an `engine_event::Payload` match arm to a `GameplayEvent` emission on the gameplay channel.

In the `submit_input` command flow, emit `InputReceived` when the first event arrives (or at the top of the spawned task before stream processing):

```rust
app.emit(GAMEPLAY_CHANNEL, &GameplayEvent::InputReceived {
    turn: current_turn,
});
```

Where `NarratorComplete` debug event is emitted, also emit on gameplay channel:

```rust
app.emit(GAMEPLAY_CHANNEL, &GameplayEvent::NarratorComplete {
    prose: narrator_prose.clone(),
    turn: turn_number,
});
```

Where `TurnComplete` debug event is emitted, also emit on gameplay channel:

```rust
app.emit(GAMEPLAY_CHANNEL, &GameplayEvent::TurnComplete {
    turn: turn_number,
    ready_for_input: true,
});
```

- [ ] **Step 2: Run check**

Run: `cargo check -p storyteller-workshop --all-features`
Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat: wire workshop:gameplay channel emission in Tauri commands"
```

### Task 9: Add frontend gameplay channel listener

**Files:**
- Modify: `crates/storyteller-workshop/src/routes/+page.svelte`
- Modify: `crates/storyteller-workshop/src/lib/StoryPane.svelte`
- Modify: `crates/storyteller-workshop/src/lib/InputBar.svelte`

- [ ] **Step 1: Add gameplay event listener in `+page.svelte`**

Import gameplay types and `listen` from Tauri:

```typescript
import { GAMEPLAY_CHANNEL, type GameplayEvent } from "$lib/types";
```

Add a gameplay event handler alongside the existing debug event handler. Use `$effect` to register the listener on mount:

```typescript
let gameplayLoading = $state(false);

$effect(() => {
  const unlisten = listen<GameplayEvent>(GAMEPLAY_CHANNEL, (event) => {
    const payload = event.payload;
    switch (payload.kind) {
      case "NarratorProse":
        // Append chunk to current turn's block
        appendProseChunk(payload.chunk, payload.turn);
        break;
      case "NarratorComplete":
        // Reconcile: replace accumulated chunks with final prose
        reconcileNarratorBlock(payload.prose, payload.turn);
        break;
      case "InputReceived":
        gameplayLoading = true;
        break;
      case "TurnComplete":
        if (payload.ready_for_input) {
          gameplayLoading = false;
        }
        break;
      case "SceneReady":
        // Store scene metadata for UI chrome
        break;
      case "ProcessingUpdate":
        // Future: update processing indicator
        break;
    }
  });
  return () => { unlisten.then(fn => fn()); };
});
```

Implement `appendProseChunk` and `reconcileNarratorBlock` helper functions that operate on the `blocks` array.

- [ ] **Step 2: Update `StoryPane.svelte`**

No structural changes needed — it already renders from the `blocks` array. The incremental chunk appending via `appendProseChunk` will trigger Svelte reactivity automatically. Verify the auto-scroll logic still works with incremental updates (it watches `blocks.length` which may need to also watch block content changes).

- [ ] **Step 3: Update `InputBar.svelte`**

Pass `gameplayLoading` as the `loading` prop instead of (or alongside) the existing loading state. The input bar already disables on `loading === true` and re-enables on `loading === false`.

- [ ] **Step 4: Run type check**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: Type check passes.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-workshop/src/routes/+page.svelte \
       crates/storyteller-workshop/src/lib/StoryPane.svelte \
       crates/storyteller-workshop/src/lib/InputBar.svelte
git commit -m "feat: add workshop:gameplay event listener with prose chunk rendering"
```

### Task 10: Implement Ollama streaming in ExternalServerProvider

**Files:**
- Modify: `crates/storyteller-engine/src/inference/external.rs`

- [ ] **Step 1: Implement `stream_complete()` override**

Override the default `stream_complete()` in `ExternalServerProvider`. Ollama's `/api/generate` with `stream: true` returns newline-delimited JSON where each line has a `response` field:

```rust
async fn stream_complete(&self, request: CompletionRequest) -> StorytellerResult<NarratorTokenStream> {
    let (sender, receiver) = narrator_token_channel();

    let url = format!("{}/api/generate", self.config.base_url);
    let body = serde_json::json!({
        "model": self.config.model,
        "prompt": /* build prompt from request */,
        "system": request.system_prompt,
        "stream": true,
        "options": {
            "temperature": request.temperature,
            "num_predict": request.max_tokens,
        }
    });

    let client = self.client.clone();
    tokio::spawn(async move {
        let response = match client.post(&url).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Ollama stream request failed: {e}");
                return;
            }
        };

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    // Process complete lines
                    while let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].trim().to_string();
                        buffer = buffer[newline_pos + 1..].to_string();
                        if line.is_empty() { continue; }
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                            if let Some(token) = json.get("response").and_then(|v| v.as_str()) {
                                if sender.0.send(token.to_string()).await.is_err() {
                                    return; // receiver dropped
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Ollama stream error: {e}");
                    break;
                }
            }
        }
        // Sender drops here, closing the channel
    });

    Ok(receiver)
}
```

Note: check how the existing `complete()` method builds the prompt from `CompletionRequest` and follow the same pattern. The `reqwest::Client` should already be stored on the provider — verify and use it.

- [ ] **Step 2: Add `futures-util` or `tokio-stream` dependency if needed**

The `bytes_stream()` method returns a `Stream`. Check if `futures-util::StreamExt` is already available in the dependency tree. If not, add `futures-util` to `storyteller-engine/Cargo.toml` under `[dependencies]`.

- [ ] **Step 3: Run check**

Run: `cargo check -p storyteller-engine --all-features`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-engine/src/inference/external.rs
git commit -m "feat: implement Ollama streaming in ExternalServerProvider"
```

### Task 11: Add prose batching and gameplay emission in engine service

**Files:**
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs`

- [ ] **Step 1: Add prose batching helper function**

Add a helper that consumes a `NarratorTokenStream`, batches by sentence boundaries, and emits `NarratorProse` events:

```rust
/// Consume a token stream, batch by sentence/paragraph boundaries,
/// and emit NarratorProse events. Returns the full accumulated prose.
async fn stream_narrator_prose(
    mut token_stream: NarratorTokenStream,
    tx: &mpsc::Sender<Result<EngineEvent, tonic::Status>>,
    session_id: &str,
    turn: u32,
) -> String {
    let mut full_buffer = String::new();
    let mut chunk_buffer = String::new();

    while let Some(token) = token_stream.0.recv().await {
        full_buffer.push_str(&token);
        chunk_buffer.push_str(&token);

        // Flush on sentence-ending punctuation or paragraph break
        let should_flush = chunk_buffer.ends_with(". ")
            || chunk_buffer.ends_with(".\n")
            || chunk_buffer.ends_with("? ")
            || chunk_buffer.ends_with("?\n")
            || chunk_buffer.ends_with("! ")
            || chunk_buffer.ends_with("!\n")
            || chunk_buffer.contains("\n\n");

        if should_flush && !chunk_buffer.trim().is_empty() {
            let event = make_event(
                session_id,
                Some(turn),
                engine_event::Payload::NarratorProse(NarratorProse {
                    chunk: chunk_buffer.clone(),
                    turn,
                }),
            );
            let _ = tx.send(Ok(event)).await;
            chunk_buffer.clear();
        }
    }

    // Flush any remaining content
    if !chunk_buffer.trim().is_empty() {
        let event = make_event(
            session_id,
            Some(turn),
            engine_event::Payload::NarratorProse(NarratorProse {
                chunk: chunk_buffer,
                turn,
            }),
        );
        let _ = tx.send(Ok(event)).await;
    }

    full_buffer
}
```

- [ ] **Step 2: Add `stream_render()` and `stream_render_opening()` to `NarratorAgent`**

**File:** `crates/storyteller-engine/src/agents/narrator.rs`

Add two new methods that mirror `render()` and `render_opening()` but call `self.llm.stream_complete()` instead of `self.llm.complete()`. They return `NarratorTokenStream` instead of `NarratorRendering` — the caller collects the full prose from the stream.

```rust
/// Stream a turn render as tokens. Caller accumulates into full prose.
pub async fn stream_render(
    &self,
    context: &NarratorContextInput,
    observer: &dyn PhaseObserver,
) -> StorytellerResult<NarratorTokenStream> {
    let user_message = build_turn_message(context);

    observer.emit(PhaseEvent {
        timestamp: Utc::now(),
        turn_number: context.journal.entries.last().map_or(0, |e| e.turn_number),
        stage: TurnCycleStage::Rendering,
        detail: PhaseEventDetail::NarratorPromptBuilt {
            system_prompt_chars: self.system_prompt.len(),
            user_message_chars: user_message.len(),
        },
    });

    let request = CompletionRequest {
        system_prompt: self.system_prompt.clone(),
        messages: vec![Message { role: MessageRole::User, content: user_message }],
        max_tokens: 400,
        temperature: self.temperature,
    };

    self.llm.stream_complete(request).await
}

/// Stream a scene opening as tokens. Caller accumulates into full prose.
pub async fn stream_render_opening(
    &self,
    observer: &dyn PhaseObserver,
) -> StorytellerResult<NarratorTokenStream> {
    let user_message = "Open the scene. Establish the setting and mood. \
        The characters have not yet interacted. Under 200 words.".to_string();

    observer.emit(PhaseEvent {
        timestamp: Utc::now(),
        turn_number: 0,
        stage: TurnCycleStage::Rendering,
        detail: PhaseEventDetail::NarratorPromptBuilt {
            system_prompt_chars: self.system_prompt.len(),
            user_message_chars: user_message.len(),
        },
    });

    let request = CompletionRequest {
        system_prompt: self.system_prompt.clone(),
        messages: vec![Message { role: MessageRole::User, content: user_message }],
        max_tokens: 600,
        temperature: self.temperature,
    };

    self.llm.stream_complete(request).await
}
```

Add the necessary import at the top of `narrator.rs`:
```rust
use storyteller_core::traits::llm::NarratorTokenStream;
```

- [ ] **Step 3: Replace batch narrator call with streaming in `submit_input`**

**File:** `crates/storyteller-server/src/grpc/engine_service.rs`

In the narrator rendering phase of `submit_input` (around lines 813-874), replace:
```rust
let rendering = narrator.render(&context, &noop).await;
```

With:
```rust
let token_stream = narrator.stream_render(&context, &noop).await?;
let prose = stream_narrator_prose(token_stream, &tx, &session_id, turn_number).await;
```

Then construct the `NarratorRendering` from the accumulated prose:
```rust
let rendering = NarratorRendering {
    text: prose,
    stage_directions: Some(resolver_output.scene_dynamics.clone()),
};
```

- [ ] **Step 4: Replace batch narrator call with streaming in `compose_scene` (turn 0)**

**File:** `crates/storyteller-server/src/grpc/engine_service.rs`

In `compose_scene`, the opening narration uses `narrator.render_opening()` (around line 332). Replace:
```rust
let rendering = narrator.render_opening(&noop).await;
```

With:
```rust
let token_stream = narrator.stream_render_opening(&noop).await?;
let prose = stream_narrator_prose(token_stream, &tx, &session_id, 0).await;
let rendering = NarratorRendering {
    text: prose,
    stage_directions: None,
};
```

Note: `render_opening()` has a simpler signature than `render()` — no `context` parameter. The `stream_render_opening()` mirrors this (only takes `observer`).

- [ ] **Step 3: Add `ProcessingUpdate` emissions**

At the start of each major phase in the turn flow, emit a `ProcessingUpdate`:

```rust
// Before enrichment
let _ = tx.send(Ok(make_event(
    &session_id, Some(turn),
    engine_event::Payload::ProcessingUpdate(ProcessingUpdate { phase: "enriching".into() }),
))).await;

// Before context assembly
let _ = tx.send(Ok(make_event(
    &session_id, Some(turn),
    engine_event::Payload::ProcessingUpdate(ProcessingUpdate { phase: "assembling".into() }),
))).await;

// Before narrator rendering
let _ = tx.send(Ok(make_event(
    &session_id, Some(turn),
    engine_event::Payload::ProcessingUpdate(ProcessingUpdate { phase: "rendering".into() }),
))).await;
```

- [ ] **Step 4: Add `InputReceived` emission**

At the start of `submit_input`, emit:

```rust
let _ = tx.send(Ok(make_event(
    &session_id, Some(turn),
    engine_event::Payload::InputReceived(InputReceived { turn }),
))).await;
```

- [ ] **Step 5: Update `TurnComplete` to include `ready_for_input`**

Where `TurnComplete` is constructed, add the new field:

```rust
engine_event::Payload::TurnComplete(TurnComplete {
    turn: turn_number,
    total_ms: elapsed,
    ready_for_input: true,
})
```

- [ ] **Step 6: Run check**

Run: `cargo check -p storyteller-server --all-features`
Expected: Compiles.

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-server/src/grpc/engine_service.rs
git commit -m "feat: add prose batching, ProcessingUpdate, and InputReceived emission in engine service"
```

### Task 12: Wire directive read path into context assembly

**Files:**
- Modify: `crates/storyteller-engine/src/context/mod.rs`

- [ ] **Step 1: Add optional directive content to context assembly**

Modify `assemble_narrator_context()` to accept an optional directive string parameter:

```rust
pub fn assemble_narrator_context(
    scene: &SceneData,
    characters: &[&CharacterSheet],
    journal: &SceneJournal,
    resolver_output: &ResolverOutput,
    player_input: &str,
    referenced_entities: &[EntityId],
    total_budget: u32,
    observer: &dyn PhaseObserver,
    player_entity_id: Option<EntityId>,
    directive_context: Option<&str>,  // NEW
) -> NarratorContextInput
```

If `directive_context` is `Some`, append it to the preamble as a compact turn-level guidance note. This is a simple string injection — the directive store reads and formatting happen at the call site in `engine_service.rs`.

- [ ] **Step 2: Update all call sites**

Search for all calls to `assemble_narrator_context()` and add `None` as the new directive parameter. There are call sites in:
- `engine_service.rs` (submit_input and compose_scene flows)
- `systems/turn_cycle.rs` (`assemble_context_system`)
- Any test files that call this function

In `engine_service.rs`, read from directive store before calling context assembly:

```rust
let directive_context = session_store
    .directives
    .applicable_for_turn(&session_id, turn_number)
    .ok()
    .and_then(|directives| {
        directives.last().map(|d| {
            format!("[Dramatic Direction] {}", d.payload)
        })
    });

let context = assemble_narrator_context(
    // ... existing params ...
    directive_context.as_deref(),
);
```

- [ ] **Step 3: Run tests**

Run: `cargo test --all-features`
Expected: All tests pass (existing tests pass `None` for the new parameter).

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-engine/src/context/mod.rs \
       crates/storyteller-server/src/grpc/engine_service.rs \
       crates/storyteller-engine/src/systems/turn_cycle.rs
git commit -m "feat: wire directive read path into context assembly (graceful on empty)"
```

### Task 13: Wire player character data into ComposeScene RPC

**Files:**
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs`

- [ ] **Step 1: Handle `PlayerCharacter` field in ComposeScene**

In the `compose_scene` handler, extract the new `player_character` field from the request:

```rust
let player_character = request.player_character;
```

Pass player character data (name, age, gender, intent) through to the scene composition logic. This may involve:
- Adding player character fields to whatever struct the composer receives
- Including player intent in the composition output that gets written to `composition.json`
- Passing player character name to the `SceneReady` event emission (from Task 8)

The exact wiring depends on how `compose_scene` currently builds the scene — read the existing flow and thread the new data through.

- [ ] **Step 2: Emit `SceneReady` with player data**

Update the `SceneReady` emission (from Task 8) to include the player character name and intent:

```rust
app.emit(GAMEPLAY_CHANNEL, &GameplayEvent::SceneReady {
    scene_id: session_id.clone(),
    title: scene_info.title.clone(),
    setting_summary: scene_info.setting_description.clone(),
    cast_names: scene_info.cast_names.clone(),
    player_character: player_character.as_ref().map(|pc| pc.name.clone()).unwrap_or_default(),
    player_intent: player_character.as_ref().and_then(|pc| pc.intent.clone()),
});
```

- [ ] **Step 3: Run check**

Run: `cargo check -p storyteller-server --all-features`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-server/src/grpc/engine_service.rs
git commit -m "feat: wire player character data into ComposeScene RPC"
```

### Task 14: Verify Layer 2

- [ ] **Step 1: Full workspace check and test**

Run: `cargo check --all-features && cargo test --all-features`
Expected: All pass.

- [ ] **Step 2: Frontend check**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: Type check passes.

- [ ] **Step 3: Commit any fixes**

```bash
git add -u
git commit -m "chore: Layer 2 plumbing verification and fixes"
```

---

## Chunk 3: Features (Layer 3)

Visible behavior changes. Each task delivers independently.

### Task 15: Add Player Character wizard step

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/SceneSetup.svelte`
- Modify: `crates/storyteller-workshop/src/lib/api.ts`

- [ ] **Step 1: Add Player Character step to wizard**

The wizard currently has 6 steps (indices 0-5). Insert a new step at index 4, shifting Setting to 5 and Launch to 6. Update the `steps` array label list.

The new step renders:
- **Name** — text input
- **Age** — select from age categories (can be hardcoded initially: "Young", "Adult", "Middle-aged", "Elder" or loaded from axis-vocabulary via a new RPC if available)
- **Gender presentation** — radio buttons: Masculine, Feminine, Non-binary
- **Intent** — textarea with placeholder "What does your character want to accomplish in this scene?"
- **Checkbox** — pre-checked "Let the system decide my character's intention" — when checked, disable the intent textarea

Add reactive state variables:

```typescript
let playerName = $state("");
let playerAge = $state("");
let playerGender = $state("");
let playerIntent = $state("");
let systemDecidesIntent = $state(true);
```

- [ ] **Step 2: Update `api.ts` compose call**

Add player character data to the `composeScene` function parameters:

```typescript
export async function composeScene(
  genreId: string,
  profileId: string,
  cast: CastMember[],
  dynamics: DynamicPairing[],
  titleOverride?: string,
  settingOverride?: string,
  playerCharacter?: { name: string; age?: string; gender_presentation?: string; intent?: string },
): Promise<SceneInfo> {
  return await invoke("compose_scene", {
    genreId, profileId, cast, dynamics, titleOverride, settingOverride,
    playerCharacter,
  });
}
```

- [ ] **Step 3: Update the Launch step to pass player character data**

In the Launch step's compose call, pass the player character data:

```typescript
const result = await composeScene(
  selectedGenre, selectedProfile, castMembers, dynamicPairings,
  titleOverride, settingOverride,
  {
    name: playerName,
    age: playerAge || undefined,
    gender_presentation: playerGender || undefined,
    intent: systemDecidesIntent ? undefined : playerIntent || undefined,
  },
);
```

- [ ] **Step 4: Update Tauri command to accept player character**

In `commands.rs`, update the `compose_scene` command signature to accept the new parameter and pass it through to the gRPC `ComposeSceneRequest`.

- [ ] **Step 5: Run type check**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: Type check passes.

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-workshop/src/lib/SceneSetup.svelte \
       crates/storyteller-workshop/src/lib/api.ts \
       crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat: add Player Character wizard step with name, age, gender, intent"
```

### Task 16: Drive frontend rendering from gameplay channel

**Files:**
- Modify: `crates/storyteller-workshop/src/routes/+page.svelte`
- Modify: `crates/storyteller-workshop/src/lib/StoryPane.svelte`

- [ ] **Step 1: Implement `appendProseChunk` and `reconcileNarratorBlock`**

In `+page.svelte`:

```typescript
function appendProseChunk(chunk: string, turn: number) {
  // Find or create a narrator block for this turn
  const existingIdx = blocks.findIndex(
    (b) => b.kind === "narrator" && b.turn === turn
  );
  if (existingIdx >= 0) {
    // Append to existing block
    blocks[existingIdx] = {
      ...blocks[existingIdx],
      text: blocks[existingIdx].text + chunk,
    };
  } else {
    // Create new block for this turn
    blocks.push({ kind: turn === 0 ? "opening" : "narrator", turn, text: chunk });
  }
}

function reconcileNarratorBlock(prose: string, turn: number) {
  // Replace accumulated chunks with final authoritative prose
  const existingIdx = blocks.findIndex(
    (b) => (b.kind === "narrator" || b.kind === "opening") && b.turn === turn
  );
  if (existingIdx >= 0) {
    blocks[existingIdx] = { ...blocks[existingIdx], text: prose };
  } else {
    blocks.push({ kind: turn === 0 ? "opening" : "narrator", turn, text: prose });
  }
}
```

- [ ] **Step 2: Update `StoryPane` auto-scroll**

The current auto-scroll watches `blocks.length`. With incremental updates, the block count doesn't change — only the content of the last block grows. Update the scroll trigger to also watch the text content of the last block:

```typescript
$effect(() => {
  const lastBlock = blocks[blocks.length - 1];
  const _ = lastBlock?.text; // reactive dependency on content
  if (!userScrolledUp) {
    scrollToBottom();
  }
});
```

- [ ] **Step 3: Remove or gate the old synchronous narrator response handling**

The existing `handleSubmit` function in `+page.svelte` currently awaits the full `TurnResult` and appends the narrator block all at once. This path should be replaced by the gameplay channel listener. Either:
- Remove the old `TurnResult` handling and rely entirely on gameplay events
- Or keep it as a fallback gated behind a feature check

The cleaner approach is to remove it — the gameplay channel is now the primary rendering path.

- [ ] **Step 4: Run type check and manual review**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: Type check passes.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-workshop/src/routes/+page.svelte \
       crates/storyteller-workshop/src/lib/StoryPane.svelte
git commit -m "feat: drive story rendering from workshop:gameplay channel with prose chunking"
```

### Task 17: Final verification

- [ ] **Step 1: Full workspace build**

Run: `cargo build -p storyteller-workshop --all-features`
Expected: Builds successfully.

- [ ] **Step 2: Full test suite**

Run: `cargo test --all-features`
Expected: All tests pass.

- [ ] **Step 3: Frontend type check**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: Passes.

- [ ] **Step 4: Review all changes**

Run: `git diff main...HEAD --stat`
Review the change summary to ensure scope discipline — only files related to A.3 should be modified.

- [ ] **Step 5: Final commit if needed**

```bash
git add -u
git commit -m "chore: final verification cleanup for event streaming architecture"
```

---

## Implementation Notes for the Agent

### Key files to read before starting
- `docs/superpowers/specs/2026-03-16-event-streaming-architecture-design.md` — the full design spec
- `crates/storyteller-core/src/types/turn_cycle.rs` — current state machine to refactor
- `crates/storyteller-server/src/grpc/engine_service.rs` — main server-side turn flow
- `crates/storyteller-workshop/src-tauri/src/commands.rs` — Tauri command layer
- `crates/storyteller-workshop/src/routes/+page.svelte` — main UI component

### Testing patterns
- All Rust tests use inline `#[cfg(test)]` modules, not separate test files
- Bevy tests use a `test_app()` helper pattern — check existing tests in `turn_cycle.rs` and `rendering.rs`
- Persistence tests use `tempfile::TempDir` for isolated filesystem testing
- Frontend uses `bun run check` for type checking (no unit test framework currently)

### Conventions
- `#[expect(lint_name, reason = "...")]` instead of `#[allow]`
- All public types must implement `Debug`
- All MPSC channels must be bounded (no `unbounded_channel()`)
- Workspace dependencies in root `Cargo.toml` under `[workspace.dependencies]`

### What NOT to implement
- Dramaturge or World Agent writers for directives.jsonl (Tier C)
- `workshop:media` channel (deferred to Tier B)
- Structured intent taxonomy (free text for now)
- Database persistence layer (jsonl modeling surface)

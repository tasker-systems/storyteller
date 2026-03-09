# Workshop UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a playable Tauri+Svelte prototype UI that wraps the existing storyteller engine pipeline, enabling interactive scene play through a web-rendered desktop app.

**Architecture:** A new `crates/storyteller-workshop` crate containing a Tauri v2 app with a Svelte+TypeScript frontend. The Rust backend calls engine functions directly (no Bevy ECS loop), modeled after `play_scene_context.rs`. Communication uses Tauri commands (in-process, not HTTP). Session state lives in Tauri managed state; each turn appends to a JSONL session log.

**Tech Stack:** Tauri v2, Svelte 5, TypeScript, Vite, bun, storyteller-core, storyteller-engine

**Design doc:** `docs/plans/2026-03-08-workshop-ui-design.md`

---

## Task 1: Scaffold the Tauri+Svelte project

**Goal:** Generate the Tauri boilerplate inside `crates/storyteller-workshop/` using `create-tauri-app`, verify it builds and opens a window.

**Step 1: Create the Tauri app scaffold**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/crates
bun create tauri-app storyteller-workshop
```

Interactive prompts — select:
- Project name: `storyteller-workshop`
- Package manager: `bun`
- UI template: `Svelte`
- UI flavor: `TypeScript`

**Step 2: Install frontend dependencies**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/crates/storyteller-workshop
bun install
```

**Step 3: Verify the scaffold runs**

```bash
bun tauri dev
```

Expected: A Tauri window opens showing the default Svelte welcome page. Close it after confirming.

**Step 4: Adjust Tauri Rust crate for workspace integration**

The scaffold creates `src-tauri/Cargo.toml` as a standalone crate. We need to integrate it with the storyteller workspace.

Edit the root workspace `Cargo.toml` to add the new member:

```toml
members = [
    "crates/storyteller",
    "crates/storyteller-core",
    "crates/storyteller-storykeeper",
    "crates/storyteller-engine",
    "crates/storyteller-api",
    "crates/storyteller-cli",
    "crates/storyteller-ml",
    "crates/storyteller-workshop/src-tauri",
]
```

Note: Tauri places its Rust crate inside `src-tauri/`, so the workspace member path is `crates/storyteller-workshop/src-tauri`.

**Step 5: Update the Tauri crate's Cargo.toml**

In `crates/storyteller-workshop/src-tauri/Cargo.toml`:
- Add workspace lint inheritance: `[lints] workspace = true`
- Add dependencies on storyteller crates:

```toml
[dependencies]
storyteller-core = { path = "../../storyteller-core" }
storyteller-engine = { path = "../../storyteller-engine" }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
```

**Step 6: Verify workspace compilation**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
cargo check --workspace
```

Expected: All crates compile, including the new Tauri crate.

**Step 7: Add `sessions/` to .gitignore**

Append to `crates/storyteller-workshop/.gitignore` (the scaffold creates one):

```
sessions/
```

**Step 8: Commit**

```bash
git add crates/storyteller-workshop/ Cargo.toml
git commit -m "feat: scaffold Tauri+Svelte workshop UI crate"
```

---

## Task 2: Session log infrastructure

**Goal:** Create a simple JSONL session logger that records each turn's data to a file. This is the "shared artifact" that enables collaborative debugging.

**Files:**
- Create: `crates/storyteller-workshop/src-tauri/src/session_log.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/main.rs` (add module)

**Step 1: Define the log entry types**

Create `crates/storyteller-workshop/src-tauri/src/session_log.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// A single turn's record in the session log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub turn: u32,
    pub timestamp: DateTime<Utc>,
    pub player_input: String,
    pub narrator_output: String,
    pub context_assembly: ContextAssemblyLog,
    pub timing: TimingLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssemblyLog {
    pub preamble_tokens: u32,
    pub journal_tokens: u32,
    pub retrieved_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingLog {
    pub prediction_ms: u64,
    pub assembly_ms: u64,
    pub narrator_ms: u64,
}

/// Manages a JSONL session log file.
#[derive(Debug)]
pub struct SessionLog {
    path: PathBuf,
}

impl SessionLog {
    /// Create a new session log. Creates the sessions/ directory if needed.
    pub fn new(sessions_dir: &std::path::Path, scene_title: &str) -> std::io::Result<Self> {
        fs::create_dir_all(sessions_dir)?;

        let now = Utc::now();
        let slug = scene_title
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-")
            .trim_matches('-')
            .to_string();
        let filename = format!("{}-{}.jsonl", now.format("%Y-%m-%d-%H%M"), slug);
        let path = sessions_dir.join(filename);

        Ok(Self { path })
    }

    /// Append a log entry as a single JSON line.
    pub fn append(&self, entry: &LogEntry) -> std::io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        writeln!(writer, "{json}")?;
        Ok(())
    }

    /// Read all entries from the log file.
    pub fn read_all(&self) -> std::io::Result<Vec<LogEntry>> {
        let content = fs::read_to_string(&self.path).unwrap_or_default();
        let entries: Vec<LogEntry> = content
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(entries)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}
```

**Step 2: Register the module**

In `crates/storyteller-workshop/src-tauri/src/main.rs`, add near the top:

```rust
mod session_log;
```

**Step 3: Verify compilation**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
cargo check -p storyteller-workshop
```

Note: The package name may differ from crate directory — check what `create-tauri-app` generates in `src-tauri/Cargo.toml` for the `[package] name` field and use that.

**Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/session_log.rs
git add crates/storyteller-workshop/src-tauri/src/main.rs
git commit -m "feat: add JSONL session log writer for workshop"
```

---

## Task 3: Engine state and Tauri commands

**Goal:** Create the Tauri managed state (holding scene data, journal, LLM provider) and implement three commands: `start_scene`, `submit_input`, `get_session_log`.

**Files:**
- Create: `crates/storyteller-workshop/src-tauri/src/commands.rs`
- Create: `crates/storyteller-workshop/src-tauri/src/engine_state.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/main.rs`

**Step 1: Create the engine state wrapper**

Create `crates/storyteller-workshop/src-tauri/src/engine_state.rs`:

```rust
use std::sync::Arc;

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::types::character::CharacterSheet;
use storyteller_core::types::narrator_context::SceneJournal;
use storyteller_core::types::scene::SceneData;

use storyteller_engine::inference::event_classifier::EventClassifier;
use storyteller_engine::inference::frame::CharacterPredictor;

use crate::session_log::SessionLog;

/// Mutable engine state held across turns within a session.
///
/// Wrapped in a `tokio::sync::Mutex` inside Tauri managed state
/// because Tauri commands are async and may run concurrently.
#[derive(Debug)]
pub struct EngineState {
    pub scene: SceneData,
    pub characters: Vec<CharacterSheet>,
    pub journal: SceneJournal,
    pub llm: Arc<dyn LlmProvider>,
    pub predictor: Option<CharacterPredictor>,
    pub event_classifier: Option<EventClassifier>,
    pub grammar: PlutchikWestern,
    pub session_log: SessionLog,
    pub turn_count: u32,
}
```

**Step 2: Create the Tauri commands**

Create `crates/storyteller-workshop/src-tauri/src/commands.rs`:

```rust
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::traits::llm::LlmProvider;
use storyteller_core::traits::phase_observer::{CollectingObserver, PhaseEventDetail};
use storyteller_core::types::narrator_context::SceneJournal;
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::scene::SceneId;

use storyteller_engine::agents::narrator::NarratorAgent;
use storyteller_engine::context::journal::add_turn;
use storyteller_engine::context::prediction::predict_character_behaviors;
use storyteller_engine::context::{assemble_narrator_context, DEFAULT_TOTAL_TOKEN_BUDGET};
use storyteller_engine::inference::external::{ExternalServerConfig, ExternalServerProvider};
use storyteller_engine::workshop::the_flute_kept;

use crate::engine_state::EngineState;
use crate::session_log::{ContextAssemblyLog, LogEntry, SessionLog, TimingLog};

/// Scene metadata returned to the frontend on start.
#[derive(Debug, Clone, Serialize)]
pub struct SceneInfo {
    pub title: String,
    pub setting_description: String,
    pub cast: Vec<String>,
    pub opening_prose: String,
}

/// Turn result returned to the frontend after each player input.
#[derive(Debug, Clone, Serialize)]
pub struct TurnResult {
    pub turn: u32,
    pub narrator_prose: String,
    pub timing: TurnTiming,
    pub context_tokens: ContextTokens,
}

#[derive(Debug, Clone, Serialize)]
pub struct TurnTiming {
    pub prediction_ms: u64,
    pub assembly_ms: u64,
    pub narrator_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextTokens {
    pub preamble: u32,
    pub journal: u32,
    pub retrieved: u32,
    pub total: u32,
}

type WorkshopState = State<'_, Mutex<Option<EngineState>>>;

/// Initialize the engine with the workshop scene and generate the opening.
#[tauri::command]
pub async fn start_scene(state: WorkshopState) -> Result<SceneInfo, String> {
    // Load scene and character data
    let scene = the_flute_kept::scene();
    let bramblehoof = the_flute_kept::bramblehoof();
    let pyotir = the_flute_kept::pyotir();
    let characters = vec![bramblehoof.clone(), pyotir.clone()];
    let grammar = PlutchikWestern::new();

    // Create LLM provider (Ollama)
    let config = ExternalServerConfig {
        base_url: "http://localhost:11434".to_string(),
        model: "qwen2.5:14b".to_string(),
        ..Default::default()
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(ExternalServerProvider::new(config));

    // Create journal
    let journal = SceneJournal::new(SceneId::new(), 1200);

    // Create session log
    let sessions_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("sessions");
    let session_log = SessionLog::new(&sessions_dir, &scene.title)
        .map_err(|e| format!("Failed to create session log: {e}"))?;

    // Assemble opening context
    let character_refs: Vec<&_> = characters.iter().collect();
    let entity_ids = vec![bramblehoof.entity_id, pyotir.entity_id];
    let opening_resolver = ResolverOutput {
        sequenced_actions: vec![],
        original_predictions: vec![],
        scene_dynamics:
            "A quiet arrival — the distance between them is physical and temporal".to_string(),
        conflicts: vec![],
    };

    let observer = CollectingObserver::new();
    let context = assemble_narrator_context(
        &scene,
        &character_refs,
        &journal,
        &opening_resolver,
        "",
        &entity_ids,
        DEFAULT_TOTAL_TOKEN_BUDGET,
        &observer,
    );

    // Generate opening prose
    let narrator = NarratorAgent::new(&context, Arc::clone(&llm)).with_temperature(0.8);
    let opening = narrator
        .render_opening(&observer)
        .await
        .map_err(|e| format!("Narrator failed: {e}"))?;

    let cast_names: Vec<String> = scene.cast.iter().map(|c| c.name.clone()).collect();
    let setting_desc = scene.setting.description.clone();
    let title = scene.title.clone();

    // Record opening in journal
    let noop = storyteller_core::traits::NoopObserver;
    let mut journal = journal;
    add_turn(
        &mut journal,
        0,
        &opening.text,
        entity_ids,
        vec![],
        &noop,
    );

    // Store engine state
    let engine = EngineState {
        scene,
        characters,
        journal,
        llm,
        predictor: None, // ML models loaded separately if available
        event_classifier: None,
        grammar,
        session_log,
        turn_count: 0,
    };

    let mut guard = state.lock().await;
    *guard = Some(engine);

    Ok(SceneInfo {
        title,
        setting_description: setting_desc,
        cast: cast_names,
        opening_prose: opening.text,
    })
}

/// Process player input through the engine pipeline and return narrator prose.
#[tauri::command]
pub async fn submit_input(
    text: String,
    state: WorkshopState,
) -> Result<TurnResult, String> {
    let mut guard = state.lock().await;
    let engine = guard
        .as_mut()
        .ok_or_else(|| "Scene not started. Call start_scene first.".to_string())?;

    engine.turn_count += 1;
    let turn = engine.turn_count;
    let turn_start = Instant::now();

    // ML predictions (if model available)
    let predict_start = Instant::now();
    let character_refs: Vec<&_> = engine.characters.iter().collect();
    let resolver_output = if let Some(ref predictor) = engine.predictor {
        let (predictions, _classification) = predict_character_behaviors(
            predictor,
            &character_refs,
            &engine.scene,
            &text,
            &engine.grammar,
            engine.event_classifier.as_ref(),
        );
        ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: predictions,
            scene_dynamics: "ML-predicted character behavior".to_string(),
            conflicts: vec![],
        }
    } else {
        ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: vec![],
            scene_dynamics:
                "A quiet arrival — the distance between them is physical and temporal".to_string(),
            conflicts: vec![],
        }
    };
    let predict_elapsed = predict_start.elapsed();

    // Context assembly
    let observer = CollectingObserver::new();
    let entity_ids: Vec<_> = engine.characters.iter().map(|c| c.entity_id).collect();
    let assembly_start = Instant::now();
    let context = assemble_narrator_context(
        &engine.scene,
        &character_refs,
        &engine.journal,
        &resolver_output,
        &text,
        &entity_ids,
        DEFAULT_TOTAL_TOKEN_BUDGET,
        &observer,
    );
    let assembly_elapsed = assembly_start.elapsed();

    // Extract token counts from observer
    let events = observer.take_events();
    let mut ctx_tokens = ContextTokens {
        preamble: 0,
        journal: 0,
        retrieved: 0,
        total: context.estimated_tokens,
    };
    for event in &events {
        if let PhaseEventDetail::ContextAssembled {
            preamble_tokens,
            journal_tokens,
            retrieved_tokens,
            ..
        } = &event.detail
        {
            ctx_tokens.preamble = *preamble_tokens;
            ctx_tokens.journal = *journal_tokens;
            ctx_tokens.retrieved = *retrieved_tokens;
        }
    }

    // Narrator rendering
    let narrator = NarratorAgent::new(&context, Arc::clone(&engine.llm)).with_temperature(0.8);
    let render_observer = CollectingObserver::new();
    let llm_start = Instant::now();
    let rendering = narrator
        .render(&context, &render_observer)
        .await
        .map_err(|e| format!("Narrator failed: {e}"))?;
    let llm_elapsed = llm_start.elapsed();

    let total_elapsed = turn_start.elapsed();

    // Update journal
    let noop = storyteller_core::traits::NoopObserver;
    add_turn(
        &mut engine.journal,
        turn,
        &rendering.text,
        entity_ids,
        extract_emotional_markers(&text),
        &noop,
    );

    // Write session log
    let timing = TimingLog {
        prediction_ms: predict_elapsed.as_millis() as u64,
        assembly_ms: assembly_elapsed.as_millis() as u64,
        narrator_ms: llm_elapsed.as_millis() as u64,
    };
    let log_entry = LogEntry {
        turn,
        timestamp: chrono::Utc::now(),
        player_input: text,
        narrator_output: rendering.text.clone(),
        context_assembly: ContextAssemblyLog {
            preamble_tokens: ctx_tokens.preamble,
            journal_tokens: ctx_tokens.journal,
            retrieved_tokens: ctx_tokens.retrieved,
            total_tokens: ctx_tokens.total,
        },
        timing: timing.clone(),
    };
    let _ = engine.session_log.append(&log_entry);

    Ok(TurnResult {
        turn,
        narrator_prose: rendering.text,
        timing: TurnTiming {
            prediction_ms: timing.prediction_ms,
            assembly_ms: timing.assembly_ms,
            narrator_ms: timing.narrator_ms,
            total_ms: total_elapsed.as_millis() as u64,
        },
        context_tokens: ctx_tokens,
    })
}

/// Return all session log entries for debugging.
#[tauri::command]
pub async fn get_session_log(
    state: WorkshopState,
) -> Result<Vec<LogEntry>, String> {
    let guard = state.lock().await;
    let engine = guard
        .as_ref()
        .ok_or_else(|| "Scene not started.".to_string())?;
    engine
        .session_log
        .read_all()
        .map_err(|e| format!("Failed to read session log: {e}"))
}

/// Naive emotional marker extraction (matches play_scene_context.rs).
fn extract_emotional_markers(input: &str) -> Vec<String> {
    let lower = input.to_lowercase();
    let mut markers = Vec::new();
    let emotional_words = [
        ("cry", "sadness"),
        ("weep", "sadness"),
        ("laugh", "joy"),
        ("smile", "joy"),
        ("angry", "anger"),
        ("afraid", "fear"),
        ("wonder", "anticipation"),
        ("hope", "anticipation"),
        ("flute", "recognition"),
        ("music", "recognition"),
        ("remember", "recognition"),
    ];
    for (word, marker) in &emotional_words {
        if lower.contains(word) {
            markers.push((*marker).to_string());
        }
    }
    markers
}
```

**Step 3: Wire commands into Tauri main**

Update `crates/storyteller-workshop/src-tauri/src/main.rs` to register the commands:

```rust
mod commands;
mod engine_state;
mod session_log;

fn main() {
    tauri::Builder::default()
        .manage(tokio::sync::Mutex::new(
            None::<engine_state::EngineState>,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::start_scene,
            commands::submit_input,
            commands::get_session_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Note: The scaffold's `main.rs` will have some default content — replace it entirely with the above. Preserve any `#![cfg_attr(...)]` attributes the scaffold places at the top (Tauri often adds `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`).

**Step 4: Verify compilation**

```bash
cargo check -p storyteller-workshop
```

Expected: Compiles. There may be warnings about unused imports or fields — address any compilation errors but don't worry about warnings yet.

**Step 5: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/
git commit -m "feat: implement Tauri commands for scene start, input, and session log"
```

---

## Task 4: Frontend TypeScript types

**Goal:** Define TypeScript types mirroring the Rust command return types, and set up the Tauri invoke wrapper.

**Files:**
- Create: `crates/storyteller-workshop/src/lib/types.ts`
- Create: `crates/storyteller-workshop/src/lib/api.ts`

Note: The scaffold places the Svelte frontend in the root `src/` directory (not `ui/src/`). Adjust paths based on what `create-tauri-app` actually generates.

**Step 1: Define TypeScript types**

Create `src/lib/types.ts`:

```typescript
export interface SceneInfo {
  title: string;
  setting_description: string;
  cast: string[];
  opening_prose: string;
}

export interface TurnResult {
  turn: number;
  narrator_prose: string;
  timing: TurnTiming;
  context_tokens: ContextTokens;
}

export interface TurnTiming {
  prediction_ms: number;
  assembly_ms: number;
  narrator_ms: number;
  total_ms: number;
}

export interface ContextTokens {
  preamble: number;
  journal: number;
  retrieved: number;
  total: number;
}

export interface LogEntry {
  turn: number;
  timestamp: string;
  player_input: string;
  narrator_output: string;
  context_assembly: {
    preamble_tokens: number;
    journal_tokens: number;
    retrieved_tokens: number;
    total_tokens: number;
  };
  timing: {
    prediction_ms: number;
    assembly_ms: number;
    narrator_ms: number;
  };
}

/** A rendered story block — either narrator prose or player input. */
export type StoryBlock =
  | { kind: "narrator"; turn: number; text: string }
  | { kind: "player"; turn: number; text: string }
  | { kind: "opening"; text: string };
```

**Step 2: Create the API wrapper**

Create `src/lib/api.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { SceneInfo, TurnResult, LogEntry } from "./types";

export async function startScene(): Promise<SceneInfo> {
  return invoke<SceneInfo>("start_scene");
}

export async function submitInput(text: string): Promise<TurnResult> {
  return invoke<TurnResult>("submit_input", { text });
}

export async function getSessionLog(): Promise<LogEntry[]> {
  return invoke<LogEntry[]>("get_session_log");
}
```

**Step 3: Verify TypeScript compiles**

```bash
cd crates/storyteller-workshop
bun run build
```

Expected: Vite build succeeds (the types aren't used yet, but should compile).

**Step 4: Commit**

```bash
git add crates/storyteller-workshop/src/lib/
git commit -m "feat: add TypeScript types and Tauri API wrapper for workshop"
```

---

## Task 5: Story pane and input bar components

**Goal:** Build the main UI — a scrolling story pane showing narrator prose with collapsible player inputs, and a text input bar at the bottom.

**Files:**
- Modify: `crates/storyteller-workshop/src/App.svelte`
- Create: `crates/storyteller-workshop/src/lib/StoryPane.svelte`
- Create: `crates/storyteller-workshop/src/lib/InputBar.svelte`
- Create: `crates/storyteller-workshop/src/lib/StoryBlock.svelte`
- Modify: `crates/storyteller-workshop/src/styles.css` (or `app.css` — whatever the scaffold creates)

**Step 1: Create the StoryBlock component**

Create `src/lib/StoryBlock.svelte`:

```svelte
<script lang="ts">
  import type { StoryBlock } from "./types";

  export let block: StoryBlock;

  let expanded = false;
</script>

{#if block.kind === "opening" || block.kind === "narrator"}
  <div class="narrator-block">
    {#each block.text.split("\n") as paragraph}
      {#if paragraph.trim()}
        <p>{paragraph.trim()}</p>
      {/if}
    {/each}
  </div>
{:else if block.kind === "player"}
  <div class="player-block" class:expanded>
    <button
      class="player-toggle"
      on:click={() => (expanded = !expanded)}
    >
      <span class="player-indicator">You</span>
      {#if !expanded}
        <span class="player-summary">{block.text.length > 80 ? block.text.slice(0, 80) + "…" : block.text}</span>
      {/if}
      <span class="toggle-icon">{expanded ? "▾" : "▸"}</span>
    </button>
    {#if expanded}
      <div class="player-full-text">
        <p>{block.text}</p>
      </div>
    {/if}
  </div>
{/if}

<style>
  .narrator-block {
    margin: 1.5rem 0;
    line-height: 1.8;
    font-size: 1.05rem;
    color: var(--text-primary, #e0e0e0);
  }

  .narrator-block p {
    margin: 0.8rem 0;
    text-indent: 1.5em;
  }

  .narrator-block p:first-child {
    text-indent: 0;
  }

  .player-block {
    margin: 0.75rem 0;
    border-left: 2px solid var(--accent-dim, #555);
    padding-left: 0.75rem;
  }

  .player-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    color: var(--text-secondary, #999);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.25rem 0;
    width: 100%;
    text-align: left;
  }

  .player-toggle:hover {
    color: var(--text-primary, #e0e0e0);
  }

  .player-indicator {
    font-weight: 600;
    color: var(--accent, #7c9cbf);
    flex-shrink: 0;
  }

  .player-summary {
    opacity: 0.7;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .toggle-icon {
    flex-shrink: 0;
    opacity: 0.5;
  }

  .player-full-text {
    color: var(--text-secondary, #bbb);
    font-size: 0.9rem;
    padding: 0.25rem 0 0.5rem;
    line-height: 1.6;
  }
</style>
```

**Step 2: Create the StoryPane component**

Create `src/lib/StoryPane.svelte`:

```svelte
<script lang="ts">
  import type { StoryBlock as StoryBlockType } from "./types";
  import StoryBlock from "./StoryBlock.svelte";
  import { afterUpdate, onMount } from "svelte";

  export let blocks: StoryBlockType[] = [];
  export let loading = false;

  let container: HTMLDivElement;
  let userScrolledUp = false;

  function handleScroll() {
    if (!container) return;
    const { scrollTop, scrollHeight, clientHeight } = container;
    userScrolledUp = scrollHeight - scrollTop - clientHeight > 50;
  }

  afterUpdate(() => {
    if (container && !userScrolledUp) {
      container.scrollTop = container.scrollHeight;
    }
  });
</script>

<div class="story-pane" bind:this={container} on:scroll={handleScroll}>
  {#each blocks as block, i (i)}
    <StoryBlock {block} />
  {/each}

  {#if loading}
    <div class="loading">
      <span class="loading-text">The story unfolds…</span>
    </div>
  {/if}
</div>

<style>
  .story-pane {
    flex: 1;
    overflow-y: auto;
    padding: 2rem 3rem;
    max-width: 50rem;
    margin: 0 auto;
    scroll-behavior: smooth;
  }

  .loading {
    margin: 1.5rem 0;
    padding: 0.5rem 0;
  }

  .loading-text {
    color: var(--text-secondary, #999);
    font-style: italic;
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }
</style>
```

**Step 3: Create the InputBar component**

Create `src/lib/InputBar.svelte`:

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let disabled = false;

  const dispatch = createEventDispatcher<{ submit: string }>();
  let text = "";

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  function submit() {
    const trimmed = text.trim();
    if (!trimmed || disabled) return;
    dispatch("submit", trimmed);
    text = "";
  }
</script>

<div class="input-bar">
  <textarea
    bind:value={text}
    on:keydown={handleKeydown}
    placeholder={disabled ? "Waiting for the narrator…" : "What do you do?"}
    {disabled}
    rows="2"
  />
  <button on:click={submit} disabled={disabled || !text.trim()}>
    Send
  </button>
</div>

<style>
  .input-bar {
    display: flex;
    gap: 0.75rem;
    padding: 1rem 2rem;
    border-top: 1px solid var(--border, #333);
    background: var(--bg-input, #1a1a1a);
    align-items: flex-end;
  }

  textarea {
    flex: 1;
    resize: none;
    background: var(--bg-textarea, #252525);
    color: var(--text-primary, #e0e0e0);
    border: 1px solid var(--border, #444);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    font-family: inherit;
    font-size: 0.95rem;
    line-height: 1.5;
  }

  textarea:focus {
    outline: none;
    border-color: var(--accent, #7c9cbf);
  }

  textarea:disabled {
    opacity: 0.5;
  }

  button {
    padding: 0.6rem 1.25rem;
    background: var(--accent, #7c9cbf);
    color: var(--bg, #111);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.9rem;
  }

  button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  button:hover:not(:disabled) {
    opacity: 0.9;
  }
</style>
```

**Step 4: Wire up App.svelte**

Replace the scaffold's `src/App.svelte` with:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import StoryPane from "./lib/StoryPane.svelte";
  import InputBar from "./lib/InputBar.svelte";
  import { startScene, submitInput } from "./lib/api";
  import type { StoryBlock } from "./lib/types";

  let blocks: StoryBlock[] = [];
  let loading = true;
  let sceneTitle = "";
  let error = "";

  onMount(async () => {
    try {
      const scene = await startScene();
      sceneTitle = scene.title;
      blocks = [{ kind: "opening", text: scene.opening_prose }];
      loading = false;
    } catch (e) {
      error = `Failed to start scene: ${e}`;
      loading = false;
    }
  });

  async function handleSubmit(event: CustomEvent<string>) {
    const text = event.detail;

    // Add player input block (collapsed)
    blocks = [
      ...blocks,
      { kind: "player", turn: blocks.filter((b) => b.kind === "narrator").length + 1, text },
    ];
    loading = true;

    try {
      const result = await submitInput(text);
      blocks = [
        ...blocks,
        { kind: "narrator", turn: result.turn, text: result.narrator_prose },
      ];
    } catch (e) {
      error = `Turn failed: ${e}`;
    } finally {
      loading = false;
    }
  }
</script>

<main>
  <header>
    <h1>{sceneTitle || "Storyteller Workshop"}</h1>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="content">
    <StoryPane {blocks} {loading} />
  </div>

  <InputBar on:submit={handleSubmit} disabled={loading} />
</main>

<style>
  :global(body) {
    margin: 0;
    background: var(--bg, #111);
    color: var(--text-primary, #e0e0e0);
    font-family: "Georgia", "Times New Roman", serif;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    padding: 0.75rem 2rem;
    border-bottom: 1px solid var(--border, #333);
    background: var(--bg-header, #161616);
  }

  header h1 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 400;
    color: var(--text-secondary, #999);
    letter-spacing: 0.05em;
  }

  .content {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .error {
    padding: 0.75rem 2rem;
    background: #3a1a1a;
    color: #e88;
    font-size: 0.85rem;
  }
</style>
```

**Step 5: Update global styles**

Find the scaffold's global CSS file (likely `src/styles.css` or `src/app.css`) and replace its contents with:

```css
:root {
  --bg: #111;
  --bg-header: #161616;
  --bg-input: #1a1a1a;
  --bg-textarea: #252525;
  --text-primary: #e0e0e0;
  --text-secondary: #999;
  --accent: #7c9cbf;
  --accent-dim: #555;
  --border: #333;
}

*, *::before, *::after {
  box-sizing: border-box;
}
```

**Step 6: Verify the full app runs**

```bash
cd crates/storyteller-workshop
bun tauri dev
```

Expected: A dark-themed window opens, shows "The Flute Kept" header, the narrator generates opening prose (requires Ollama running at localhost:11434 with qwen2.5:14b), and you can type player input.

If Ollama is not running, the app should show an error message from the `start_scene` command failure — that's fine for now.

**Step 7: Commit**

```bash
git add crates/storyteller-workshop/src/
git commit -m "feat: implement story pane, input bar, and app layout for workshop UI"
```

---

## Task 6: Smoke test and polish

**Goal:** Run a real play session, fix any issues that surface, and make sure the session log writes correctly.

**Step 1: Start Ollama**

Ensure Ollama is running with the qwen2.5:14b model:

```bash
ollama run qwen2.5:14b
```

(Or whatever model is available — adjust the model name in `commands.rs` `start_scene` if needed.)

**Step 2: Run the workshop**

```bash
cd crates/storyteller-workshop
bun tauri dev
```

**Step 3: Play through 3-4 turns**

- Wait for the opening prose to render
- Type a player action and submit
- Verify narrator responds with prose
- Verify player input blocks appear collapsed and are expandable
- Verify auto-scroll behavior
- Check that the loading indicator shows during narrator generation

**Step 4: Verify session log**

```bash
cat crates/storyteller-workshop/sessions/*.jsonl | head -5
```

Expected: JSONL entries with turn data, timing, and token counts.

**Step 5: Fix any issues found during play**

Address compilation errors, UI layout problems, or command failures. Common issues to watch for:
- Tauri command serialization mismatches (field name casing)
- Missing Tauri permissions in `tauri.conf.json` or `capabilities/`
- LLM timeout if Ollama is slow to respond

**Step 6: Final commit**

```bash
git add -A crates/storyteller-workshop/
git commit -m "feat: workshop UI smoke tested and working with live scene play"
```

---

## Notes for the Implementing Engineer

### Prerequisite: Ollama
The workshop requires a running Ollama instance at `localhost:11434` with a compatible model. Without it, `start_scene` will fail when trying to generate the opening prose. The error will surface in the UI.

### Scaffold Differences
The `create-tauri-app` scaffold may generate slightly different directory structures or file names than this plan assumes. Key things to watch:
- The Svelte frontend lives at `src/` (not `ui/src/`) in the scaffold
- The Rust backend lives at `src-tauri/`
- Global CSS filename may be `styles.css`, `app.css`, or `global.css`
- The scaffold's `main.rs` may have platform-specific attributes — preserve those

### Workspace Integration
The Tauri crate is at `crates/storyteller-workshop/src-tauri/`, which is one level deeper than other workspace members. This is normal for Tauri projects. The workspace member path in the root `Cargo.toml` must reflect this.

### Type Alignment
The Rust `#[derive(Serialize)]` structs in `commands.rs` produce camelCase JSON by default with Tauri. If field names don't match TypeScript types, add `#[serde(rename_all = "snake_case")]` to the Rust structs or adjust the TypeScript types to use camelCase.

### Future: ML Model Loading
The plan sets `predictor: None` and `event_classifier: None` in `start_scene`. To enable ML predictions, add model resolution logic (copy from `play_scene_context.rs` functions `resolve_model_path` and `resolve_event_classifier_path`). This is a natural follow-up once the basic loop works.

# Workshop Debug Inspector Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a collapsible Chrome DevTools-style debug inspector panel to the storyteller-workshop Tauri app with real-time pipeline observability via Tauri events.

**Architecture:** Tauri events emitted from `commands.rs` at each pipeline phase boundary populate a tabbed debug panel in the Svelte frontend. The existing `submit_input` command/response flow is unchanged — events are additive observability. The debug panel is collapsible (Cmd+D) so the workshop doubles as a clean playtesting environment.

**Tech Stack:** Tauri 2 events (`app_handle.emit()`), Svelte 5 runes, `@tauri-apps/api/event` (`listen()`), existing storyteller-core/engine types.

**Design doc:** `docs/plans/2026-03-09-workshop-debug-inspector-design.md`

---

### Task 1: Update Window Size and Add Debug Panel CSS Variables

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/tauri.conf.json`
- Modify: `crates/storyteller-workshop/src/app.css`

**Step 1: Update window dimensions**

In `tauri.conf.json`, change the window config:

```json
"windows": [
  {
    "title": "storyteller-workshop",
    "width": 1400,
    "height": 900
  }
]
```

**Step 2: Add debug panel CSS variables to app.css**

Add after the existing `:root` variables:

```css
  --bg-debug: #1a1a1a;
  --bg-debug-tab: #222;
  --bg-debug-active-tab: #2a2a2a;
  --text-debug: #ccc;
  --text-debug-dim: #777;
  --border-debug: #2a2a2a;
  --font-mono: "SF Mono", "Fira Code", "Cascadia Code", "Courier New", Courier, monospace;
  --debug-green: #5a9;
  --debug-yellow: #da5;
  --debug-grey: #555;
```

**Step 3: Verify the frontend builds**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: No errors.

**Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/tauri.conf.json crates/storyteller-workshop/src/app.css
git commit -m "feat(workshop): enlarge window and add debug panel CSS variables"
```

---

### Task 2: Define Rust Event Types (Single Tagged Enum)

**Files:**
- Create: `crates/storyteller-workshop/src-tauri/src/events.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/lib.rs` (add `mod events;`)

**Step 1: Create events.rs with a single tagged enum**

All debug events are emitted on the `"workshop:debug"` channel. The `DebugEvent` enum uses `#[serde(tag = "type")]` so the frontend receives a JSON object with a `type` discriminator and phase-specific fields flattened alongside common metadata.

```rust
//! Debug inspector events — single tagged enum emitted on `"workshop:debug"`.
//!
//! All events share a consistent envelope: a `type` discriminator field (from the
//! serde tag) plus a `turn` field, with phase-specific payload fields alongside.
//! The frontend listens to one Tauri event and dispatches on `type`.

use serde::{Deserialize, Serialize};
use storyteller_core::types::character::CharacterSheet;
use storyteller_core::types::resolver::ResolverOutput;

/// The single event channel name used for all debug inspector events.
pub const DEBUG_EVENT_CHANNEL: &str = "workshop:debug";

/// All debug inspector events emitted during turn processing.
///
/// Serializes as `{ "type": "phase_started", "turn": 1, "phase": "prediction" }` etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DebugEvent {
    /// A pipeline phase has started processing.
    #[serde(rename = "phase_started")]
    PhaseStarted { turn: u32, phase: String },

    /// ML character predictions completed.
    #[serde(rename = "prediction_complete")]
    PredictionComplete {
        turn: u32,
        resolver_output: ResolverOutput,
        timing_ms: u64,
        model_loaded: bool,
    },

    /// Context assembly completed — the three tiers rendered as text.
    #[serde(rename = "context_assembled")]
    ContextAssembled {
        turn: u32,
        preamble_text: String,
        journal_text: String,
        retrieved_text: String,
        token_counts: TokenCounts,
        timing_ms: u64,
    },

    /// Character data for the turn.
    #[serde(rename = "characters_updated")]
    CharactersUpdated {
        turn: u32,
        characters: Vec<CharacterSheet>,
        emotional_markers: Vec<String>,
    },

    /// Event classification results.
    #[serde(rename = "events_classified")]
    EventsClassified {
        turn: u32,
        classifications: Vec<String>,
        classifier_loaded: bool,
    },

    /// Narrator LLM call completed — raw prompt and response.
    #[serde(rename = "narrator_complete")]
    NarratorComplete {
        turn: u32,
        system_prompt: String,
        user_message: String,
        raw_response: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        tokens_used: u32,
        timing_ms: u64,
    },

    /// An error occurred during a pipeline phase.
    #[serde(rename = "error")]
    Error {
        turn: u32,
        phase: String,
        message: String,
    },
}

/// Token counts per context assembly tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCounts {
    pub preamble: u32,
    pub journal: u32,
    pub retrieved: u32,
    pub total: u32,
}
```

**Step 2: Register the module in lib.rs**

Add `mod events;` alongside the existing module declarations:

```rust
mod commands;
mod engine_state;
mod events;
mod session_log;
```

**Step 3: Verify it compiles**

Run: `cd crates/storyteller-workshop/src-tauri && cargo check`
Expected: Compiles cleanly.

**Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/events.rs crates/storyteller-workshop/src-tauri/src/lib.rs
git commit -m "feat(workshop): define DebugEvent tagged enum for inspector events"
```

---

### Task 3: Emit Events from `submit_input` Command

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`
- Modify: `crates/storyteller-engine/src/agents/narrator.rs`

This is the largest backend change. The `submit_input` command needs access to `AppHandle` to emit events, and needs to emit events at each phase boundary. The command signature changes to accept `app: tauri::AppHandle` as a parameter (Tauri automatically injects this).

All events are emitted on the single `DEBUG_EVENT_CHANNEL` using `DebugEvent` variants.

**Step 1: Add imports to commands.rs**

Add to the existing import block:

```rust
use tauri::Emitter;

use crate::events::{DebugEvent, TokenCounts, DEBUG_EVENT_CHANNEL};
```

**Step 2: Add a helper to emit debug events**

Add near the other helpers at the bottom of `commands.rs`:

```rust
/// Emit a debug event to the inspector panel. Failures are silently ignored —
/// debug events are best-effort observability, never blocking.
fn emit_debug(app: &tauri::AppHandle, event: DebugEvent) {
    let _ = app.emit(DEBUG_EVENT_CHANNEL, event);
}
```

**Step 3: Modify `submit_input` to accept `AppHandle` and emit events**

Key changes:
- Add `app: tauri::AppHandle` parameter
- Emit `DebugEvent::PhaseStarted` before each phase
- Emit phase-complete variants after each phase
- Emit `DebugEvent::Error` on failures
- Capture narrator prompt data for the Narrator tab
- The function still returns `TurnResult` as before

Replace the entire `submit_input` function (lines 228-344 of commands.rs):

```rust
/// Process player input through the engine pipeline and return the narrator's response.
#[tauri::command]
pub async fn submit_input(
    input: String,
    app: tauri::AppHandle,
    state: State<'_, Mutex<Option<EngineState>>>,
) -> Result<TurnResult, String> {
    let mut guard = state.lock().await;
    let engine = guard
        .as_mut()
        .ok_or_else(|| "No scene loaded. Call start_scene first.".to_string())?;

    engine.turn_count += 1;
    let turn = engine.turn_count;

    let characters_refs: Vec<&_> = engine.characters.iter().collect();
    let entity_ids: Vec<_> = engine.characters.iter().map(|c| c.entity_id).collect();

    // --- Phase: ML Predictions ---
    emit_debug(&app, DebugEvent::PhaseStarted {
        turn,
        phase: "prediction".to_string(),
    });

    let predict_start = Instant::now();
    let resolver_output = if let Some(ref predictor) = engine.predictor {
        let (predictions, _classification) = predict_character_behaviors(
            predictor,
            &characters_refs,
            &engine.scene,
            &input,
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
            scene_dynamics: "A quiet arrival — the distance between them is physical and temporal"
                .to_string(),
            conflicts: vec![],
        }
    };
    let prediction_ms = predict_start.elapsed().as_millis() as u64;

    emit_debug(&app, DebugEvent::PredictionComplete {
        turn,
        resolver_output: resolver_output.clone(),
        timing_ms: prediction_ms,
        model_loaded: engine.predictor.is_some(),
    });

    // --- Phase: Characters ---
    emit_debug(&app, DebugEvent::PhaseStarted {
        turn,
        phase: "characters".to_string(),
    });

    let emotional_markers = extract_emotional_markers(&input);
    emit_debug(&app, DebugEvent::CharactersUpdated {
        turn,
        characters: engine.characters.clone(),
        emotional_markers: emotional_markers.clone(),
    });

    // --- Phase: Event Classification ---
    emit_debug(&app, DebugEvent::PhaseStarted {
        turn,
        phase: "events".to_string(),
    });

    let classifications: Vec<String> = if let Some(ref classifier) = engine.event_classifier {
        match classifier.classify(&input) {
            Ok(results) => results.iter().map(|r| format!("{}: {:.2}", r.label, r.score)).collect(),
            Err(e) => vec![format!("Classification error: {e}")],
        }
    } else {
        vec![]
    };

    emit_debug(&app, DebugEvent::EventsClassified {
        turn,
        classifications,
        classifier_loaded: engine.event_classifier.is_some(),
    });

    // --- Phase: Context Assembly ---
    emit_debug(&app, DebugEvent::PhaseStarted {
        turn,
        phase: "context".to_string(),
    });

    let observer = CollectingObserver::new();
    let assembly_start = Instant::now();
    let context = assemble_narrator_context(
        &engine.scene,
        &characters_refs,
        &engine.journal,
        &resolver_output,
        &input,
        &entity_ids,
        DEFAULT_TOTAL_TOKEN_BUDGET,
        &observer,
    );
    let assembly_ms = assembly_start.elapsed().as_millis() as u64;

    let token_counts = extract_token_counts(&observer);

    // Render context tiers as text for the debug inspector
    let preamble_text = format!(
        "Narrator: {}\nSetting: {}\nCast: {}\nBoundaries: {}",
        context.preamble.narrator_identity,
        context.preamble.setting_description,
        context.preamble.cast_descriptions.iter()
            .map(|c| format!("{} ({})", c.name, c.role))
            .collect::<Vec<_>>()
            .join(", "),
        context.preamble.boundaries.join("; "),
    );
    let journal_text = context.journal.entries.iter()
        .map(|e| format!("[Turn {}] {}", e.turn_number, e.summary))
        .collect::<Vec<_>>()
        .join("\n");
    let retrieved_text = context.retrieved.iter()
        .map(|r| {
            let mut s = format!("{}: {}", r.subject, r.content);
            if let Some(ref emo) = r.emotional_context {
                s.push_str(&format!(" ({})", emo));
            }
            s
        })
        .collect::<Vec<_>>()
        .join("\n");

    emit_debug(&app, DebugEvent::ContextAssembled {
        turn,
        preamble_text,
        journal_text,
        retrieved_text,
        token_counts: TokenCounts {
            preamble: token_counts.0,
            journal: token_counts.1,
            retrieved: token_counts.2,
            total: token_counts.3,
        },
        timing_ms: assembly_ms,
    });

    // --- Phase: Narrator Rendering ---
    emit_debug(&app, DebugEvent::PhaseStarted {
        turn,
        phase: "narrator".to_string(),
    });

    let narrator = NarratorAgent::new(&context, Arc::clone(&engine.llm)).with_temperature(0.8);
    let llm_start = Instant::now();
    let rendering = match narrator.render(&context, &observer).await {
        Ok(r) => r,
        Err(e) => {
            emit_debug(&app, DebugEvent::Error {
                turn,
                phase: "narrator".to_string(),
                message: format!("{e}"),
            });
            return Err(format!("Narrator render failed: {e}"));
        }
    };
    let narrator_ms = llm_start.elapsed().as_millis() as u64;

    emit_debug(&app, DebugEvent::NarratorComplete {
        turn,
        system_prompt: narrator.system_prompt().to_string(),
        user_message: "See context assembly".to_string(),
        raw_response: rendering.text.clone(),
        model: "qwen2.5:14b".to_string(),
        temperature: 0.8,
        max_tokens: 400,
        tokens_used: 0, // Not exposed from current render API
        timing_ms: narrator_ms,
    });

    // Update journal
    let noop = NoopObserver;
    add_turn(
        &mut engine.journal,
        turn,
        &rendering.text,
        entity_ids,
        emotional_markers,
        &noop,
    );

    // Append to session log
    let log_entry = LogEntry {
        turn,
        timestamp: chrono::Utc::now(),
        player_input: input,
        narrator_output: rendering.text.clone(),
        context_assembly: ContextAssemblyLog {
            preamble_tokens: token_counts.0,
            journal_tokens: token_counts.1,
            retrieved_tokens: token_counts.2,
            total_tokens: token_counts.3,
        },
        timing: TimingLog {
            prediction_ms,
            assembly_ms,
            narrator_ms,
        },
    };
    engine.session_log.append(&log_entry)?;

    Ok(TurnResult {
        turn,
        narrator_prose: rendering.text,
        timing: TurnTiming {
            prediction_ms,
            assembly_ms,
            narrator_ms,
        },
        context_tokens: ContextTokens {
            preamble: token_counts.0,
            journal: token_counts.1,
            retrieved: token_counts.2,
            total: token_counts.3,
        },
    })
}
```

**Step 4: Expose `system_prompt` from NarratorAgent**

The `NarratorAgent.system_prompt` field is currently private. We need a public accessor. Add to `crates/storyteller-engine/src/agents/narrator.rs` after the `with_temperature` method:

```rust
/// Access the system prompt (for debugging/inspection).
pub fn system_prompt(&self) -> &str {
    &self.system_prompt
}
```

**Step 5: Check EventClassifier has a `classify` method**

Run: `grep -n "pub fn classify" crates/storyteller-engine/src/inference/event_classifier.rs`

Verify the method signature. If the method name or signature differs, adjust the event classification block in step 3 accordingly. The event classification section is best-effort — if the classifier API doesn't match, emit an empty classifications vec and note it for a future task.

**Step 6: Verify it compiles**

Run: `cd crates/storyteller-workshop/src-tauri && cargo check`
Expected: Compiles cleanly.

**Step 7: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs crates/storyteller-engine/src/agents/narrator.rs
git commit -m "feat(workshop): emit DebugEvent at pipeline phase boundaries"
```

---

### Task 4: Define TypeScript Event Types (Discriminated Union)

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/types.ts`

**Step 1: Add event types as a discriminated union**

Append to `types.ts`. The `DebugEvent` type mirrors the Rust `DebugEvent` enum — the `type` field is the discriminator, matching the `#[serde(rename = "...")]` values.

```typescript
// ---------------------------------------------------------------------------
// Debug inspector events — discriminated union on "type" field.
// All events arrive on the single "workshop:debug" Tauri event channel.
// ---------------------------------------------------------------------------

export const DEBUG_EVENT_CHANNEL = "workshop:debug";

export interface PhaseStartedEvent {
  type: "phase_started";
  turn: number;
  phase: string;
}

export interface PredictionCompleteEvent {
  type: "prediction_complete";
  turn: number;
  resolver_output: {
    sequenced_actions: unknown[];
    original_predictions: unknown[];
    scene_dynamics: string;
    conflicts: unknown[];
  };
  timing_ms: number;
  model_loaded: boolean;
}

export interface ContextAssembledEvent {
  type: "context_assembled";
  turn: number;
  preamble_text: string;
  journal_text: string;
  retrieved_text: string;
  token_counts: {
    preamble: number;
    journal: number;
    retrieved: number;
    total: number;
  };
  timing_ms: number;
}

export interface CharactersUpdatedEvent {
  type: "characters_updated";
  turn: number;
  characters: unknown[];
  emotional_markers: string[];
}

export interface EventsClassifiedEvent {
  type: "events_classified";
  turn: number;
  classifications: string[];
  classifier_loaded: boolean;
}

export interface NarratorCompleteEvent {
  type: "narrator_complete";
  turn: number;
  system_prompt: string;
  user_message: string;
  raw_response: string;
  model: string;
  temperature: number;
  max_tokens: number;
  tokens_used: number;
  timing_ms: number;
}

export interface ErrorEvent {
  type: "error";
  turn: number;
  phase: string;
  message: string;
}

export type DebugEvent =
  | PhaseStartedEvent
  | PredictionCompleteEvent
  | ContextAssembledEvent
  | CharactersUpdatedEvent
  | EventsClassifiedEvent
  | NarratorCompleteEvent
  | ErrorEvent;

export type PhaseStatus = "pending" | "processing" | "complete" | "skipped" | "error";

export interface DebugState {
  turn: number;
  phases: Record<string, PhaseStatus>;
  prediction: PredictionCompleteEvent | null;
  context: ContextAssembledEvent | null;
  characters: CharactersUpdatedEvent | null;
  events: EventsClassifiedEvent | null;
  narrator: NarratorCompleteEvent | null;
  error: ErrorEvent | null;
}
```

**Step 2: Verify types**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: No errors.

**Step 3: Commit**

```bash
git add crates/storyteller-workshop/src/lib/types.ts
git commit -m "feat(workshop): add TypeScript event payload types for debug inspector"
```

---

### Task 5: Create the DebugPanel Component

**Files:**
- Create: `crates/storyteller-workshop/src/lib/DebugPanel.svelte`

This is the main container component — tab bar, collapsible wrapper, and event listeners. It manages all debug state and passes data to tab content areas.

**Step 1: Create DebugPanel.svelte**

```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type { DebugState, DebugEvent, PhaseStatus } from "./types";
  import { DEBUG_EVENT_CHANNEL } from "./types";

  let { visible }: { visible: boolean } = $props();

  const TABS = ["Context", "ML Predictions", "Characters", "Events", "Narrator"] as const;
  type TabName = (typeof TABS)[number];
  const TAB_PHASE_MAP: Record<TabName, string> = {
    Context: "context",
    "ML Predictions": "prediction",
    Characters: "characters",
    Events: "events",
    Narrator: "narrator",
  };

  let activeTab: TabName = $state("Context");
  let debugState: DebugState = $state({
    turn: 0,
    phases: {},
    prediction: null,
    context: null,
    characters: null,
    events: null,
    narrator: null,
    error: null,
  });

  function resetForTurn(turn: number) {
    debugState = {
      turn,
      phases: {},
      prediction: null,
      context: null,
      characters: null,
      events: null,
      narrator: null,
      error: null,
    };
  }

  function phaseStatus(tab: TabName): PhaseStatus {
    const phase = TAB_PHASE_MAP[tab];
    return debugState.phases[phase] ?? "pending";
  }

  function handleDebugEvent(event: DebugEvent) {
    if (event.turn !== debugState.turn) {
      resetForTurn(event.turn);
    }

    switch (event.type) {
      case "phase_started":
        debugState.phases[event.phase] = "processing";
        break;
      case "prediction_complete":
        debugState.prediction = event;
        debugState.phases["prediction"] = "complete";
        break;
      case "context_assembled":
        debugState.context = event;
        debugState.phases["context"] = "complete";
        break;
      case "characters_updated":
        debugState.characters = event;
        debugState.phases["characters"] = "complete";
        break;
      case "events_classified":
        debugState.events = event;
        debugState.phases["events"] = "complete";
        break;
      case "narrator_complete":
        debugState.narrator = event;
        debugState.phases["narrator"] = "complete";
        break;
      case "error":
        debugState.error = event;
        debugState.phases[event.phase] = "error";
        break;
    }

    debugState = debugState; // trigger reactivity
  }

  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    unlisten = await listen<DebugEvent>(DEBUG_EVENT_CHANNEL, (e) => {
      handleDebugEvent(e.payload);
    });
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

{#if visible}
  <div class="debug-panel">
    <div class="debug-tab-bar">
      {#each TABS as tab}
        {@const status = phaseStatus(tab)}
        <button
          class="debug-tab"
          class:active={activeTab === tab}
          onclick={() => (activeTab = tab)}
        >
          <span class="phase-dot {status}"></span>
          {tab}
        </button>
      {/each}
      <span class="debug-turn-label">
        {#if debugState.turn > 0}Turn {debugState.turn}{/if}
      </span>
    </div>

    <div class="debug-content">
      {#if activeTab === "Context"}
        <div class="debug-tab-content">
          {#if debugState.context}
            <div class="debug-section">
              <h4>Preamble <span class="token-count">{debugState.context.token_counts.preamble}t</span></h4>
              <pre>{debugState.context.preamble_text}</pre>
            </div>
            <div class="debug-section">
              <h4>Journal <span class="token-count">{debugState.context.token_counts.journal}t</span></h4>
              <pre>{debugState.context.journal_text || "(empty)"}</pre>
            </div>
            <div class="debug-section">
              <h4>Retrieved <span class="token-count">{debugState.context.token_counts.retrieved}t</span></h4>
              <pre>{debugState.context.retrieved_text || "(none)"}</pre>
            </div>
            <div class="debug-section">
              <h4>Total: {debugState.context.token_counts.total}t | Assembly: {debugState.context.timing_ms}ms</h4>
            </div>
          {:else}
            <p class="debug-empty">Waiting for turn data...</p>
          {/if}
        </div>
      {:else if activeTab === "ML Predictions"}
        <div class="debug-tab-content">
          {#if debugState.prediction}
            {#if !debugState.prediction.model_loaded}
              <p class="debug-notice">No ML model loaded. Set STORYTELLER_MODEL_PATH or STORYTELLER_DATA_PATH.</p>
            {/if}
            <div class="debug-section">
              <h4>Scene Dynamics</h4>
              <pre>{debugState.prediction.resolver_output.scene_dynamics}</pre>
            </div>
            {#if debugState.prediction.resolver_output.original_predictions.length > 0}
              <div class="debug-section">
                <h4>Character Predictions</h4>
                <pre>{JSON.stringify(debugState.prediction.resolver_output.original_predictions, null, 2)}</pre>
              </div>
            {/if}
            <div class="debug-section">
              <h4>Prediction: {debugState.prediction.timing_ms}ms</h4>
            </div>
          {:else}
            <p class="debug-empty">Waiting for turn data...</p>
          {/if}
        </div>
      {:else if activeTab === "Characters"}
        <div class="debug-tab-content">
          {#if debugState.characters}
            <div class="debug-section">
              <h4>Emotional Markers</h4>
              <pre>{debugState.characters.emotional_markers.length > 0 ? debugState.characters.emotional_markers.join(", ") : "(none detected)"}</pre>
            </div>
            {#each debugState.characters.characters as char}
              <div class="debug-section">
                <h4>{(char as any).name ?? "Character"}</h4>
                <pre>{JSON.stringify(char, null, 2)}</pre>
              </div>
            {/each}
          {:else}
            <p class="debug-empty">Waiting for turn data...</p>
          {/if}
        </div>
      {:else if activeTab === "Events"}
        <div class="debug-tab-content">
          {#if debugState.events}
            {#if !debugState.events.classifier_loaded}
              <p class="debug-notice">No event classifier loaded. Set STORYTELLER_MODEL_PATH or STORYTELLER_DATA_PATH.</p>
            {/if}
            {#if debugState.events.classifications.length > 0}
              <div class="debug-section">
                <h4>Classifications</h4>
                {#each debugState.events.classifications as cls}
                  <pre>{cls}</pre>
                {/each}
              </div>
            {:else}
              <p class="debug-empty">No classifications produced.</p>
            {/if}
          {:else}
            <p class="debug-empty">Waiting for turn data...</p>
          {/if}
        </div>
      {:else if activeTab === "Narrator"}
        <div class="debug-tab-content">
          {#if debugState.narrator}
            <div class="debug-section">
              <h4>Model: {debugState.narrator.model} | Temp: {debugState.narrator.temperature} | Max: {debugState.narrator.max_tokens}t</h4>
            </div>
            <div class="debug-section">
              <h4>System Prompt</h4>
              <pre>{debugState.narrator.system_prompt}</pre>
            </div>
            <div class="debug-section">
              <h4>Raw Response</h4>
              <pre>{debugState.narrator.raw_response}</pre>
            </div>
            <div class="debug-section">
              <h4>Narrator LLM: {debugState.narrator.timing_ms}ms</h4>
            </div>
          {:else}
            <p class="debug-empty">Waiting for turn data...</p>
          {/if}
        </div>
      {/if}

      {#if debugState.error}
        <div class="debug-error">
          Error in {debugState.error.phase}: {debugState.error.message}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .debug-panel {
    flex-shrink: 0;
    height: 40%;
    background: var(--bg-debug);
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text-debug);
  }

  .debug-tab-bar {
    display: flex;
    align-items: center;
    gap: 0;
    border-bottom: 1px solid var(--border-debug);
    flex-shrink: 0;
    padding: 0 0.5rem;
  }

  .debug-tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-debug-dim);
    font-family: var(--font-mono);
    font-size: 0.75rem;
    padding: 0.5rem 0.75rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    box-shadow: none;
  }

  .debug-tab:hover {
    color: var(--text-debug);
  }

  .debug-tab.active {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .debug-turn-label {
    margin-left: auto;
    color: var(--text-debug-dim);
    font-size: 0.7rem;
    padding-right: 0.5rem;
  }

  .phase-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }

  .phase-dot.pending {
    background: var(--debug-grey);
  }

  .phase-dot.processing {
    background: var(--debug-yellow);
    animation: pulse 1s ease-in-out infinite;
  }

  .phase-dot.complete {
    background: var(--debug-green);
  }

  .phase-dot.skipped {
    background: var(--debug-grey);
    opacity: 0.4;
  }

  .phase-dot.error {
    background: #d55;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.4;
    }
    50% {
      opacity: 1;
    }
  }

  .debug-content {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem 1rem;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .debug-tab-content {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .debug-section h4 {
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 500;
    margin-bottom: 0.25rem;
  }

  .debug-section pre {
    background: var(--bg-debug-tab);
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.5;
    font-size: 0.75rem;
    max-height: 200px;
    overflow-y: auto;
  }

  .token-count {
    color: var(--debug-green);
    font-weight: 400;
  }

  .debug-empty {
    color: var(--text-debug-dim);
    font-style: italic;
  }

  .debug-notice {
    color: var(--debug-yellow);
    font-size: 0.75rem;
    padding: 0.25rem 0;
  }

  .debug-error {
    background: #2a1515;
    color: #d88;
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    margin-top: 0.5rem;
    font-size: 0.75rem;
  }
</style>
```

**Step 2: Verify types**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: No errors.

**Step 3: Commit**

```bash
git add crates/storyteller-workshop/src/lib/DebugPanel.svelte
git commit -m "feat(workshop): create DebugPanel component with tabbed inspector"
```

---

### Task 6: Integrate Debug Panel into Main Page

**Files:**
- Modify: `crates/storyteller-workshop/src/routes/+page.svelte`

**Step 1: Add debug panel toggle state, keyboard shortcut, and component**

Replace the entire `+page.svelte` with:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { startScene, submitInput } from "$lib/api";
  import type { StoryBlock, SceneInfo } from "$lib/types";
  import StoryPane from "$lib/StoryPane.svelte";
  import InputBar from "$lib/InputBar.svelte";
  import DebugPanel from "$lib/DebugPanel.svelte";

  let sceneInfo: SceneInfo | null = $state(null);
  let blocks: StoryBlock[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let turnCount = $state(0);
  let debugVisible = $state(true);

  onMount(async () => {
    // Cmd+D / Ctrl+D toggle for debug panel
    function handleKeydown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "d") {
        e.preventDefault();
        debugVisible = !debugVisible;
      }
    }
    window.addEventListener("keydown", handleKeydown);

    try {
      const info = await startScene();
      sceneInfo = info;
      blocks = [{ kind: "opening", text: info.opening_prose }];
      loading = false;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      loading = false;
    }

    return () => window.removeEventListener("keydown", handleKeydown);
  });

  async function handleSubmit(text: string) {
    turnCount += 1;
    const playerTurn = turnCount;

    blocks = [...blocks, { kind: "player", turn: playerTurn, text }];
    loading = true;
    error = null;

    try {
      const result = await submitInput(text);
      blocks = [...blocks, { kind: "narrator", turn: result.turn, text: result.narrator_prose }];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="app-layout">
  <header class="app-header">
    <h1 class="scene-title">{sceneInfo?.title ?? "Loading..."}</h1>
    <button
      class="debug-toggle"
      onclick={() => (debugVisible = !debugVisible)}
      title={debugVisible ? "Hide inspector (⌘D)" : "Show inspector (⌘D)"}
    >
      {debugVisible ? "▼" : "▲"} Inspector
    </button>
  </header>

  {#if error}
    <div class="error-banner">
      <span class="error-text">{error}</span>
      <button class="error-dismiss" onclick={() => (error = null)}>dismiss</button>
    </div>
  {/if}

  <StoryPane {blocks} {loading} />

  <InputBar disabled={loading} onsubmit={handleSubmit} />

  <DebugPanel visible={debugVisible} />
</div>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
  }

  .app-header {
    background: var(--bg-header);
    border-bottom: 1px solid var(--border);
    padding: 0.6rem 1.5rem;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .scene-title {
    font-family: Georgia, "Times New Roman", serif;
    font-size: 1.1rem;
    font-weight: 400;
    font-style: italic;
    color: var(--accent);
    text-align: center;
    letter-spacing: 0.02em;
  }

  .debug-toggle {
    position: absolute;
    right: 1rem;
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    padding: 0.2rem 0.6rem;
    border-radius: 3px;
    cursor: pointer;
    box-shadow: none;
  }

  .debug-toggle:hover {
    color: var(--text-primary);
    border-color: var(--accent-dim);
  }

  .error-banner {
    background: #2a1515;
    border-bottom: 1px solid #4a2020;
    padding: 0.5rem 1.5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
  }

  .error-text {
    color: #d88;
    font-size: 0.85rem;
  }

  .error-dismiss {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0.2em 0.5em;
    box-shadow: none;
  }

  .error-dismiss:hover {
    color: var(--text-primary);
  }
</style>
```

**Step 2: Verify frontend builds**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: No errors.

**Step 3: Commit**

```bash
git add crates/storyteller-workshop/src/routes/+page.svelte
git commit -m "feat(workshop): integrate debug panel with toggle into main page"
```

---

### Task 7: Build and Smoke Test

**Files:** None (verification only)

**Step 1: Full frontend type check**

Run: `cd crates/storyteller-workshop && bun run check`
Expected: No errors.

**Step 2: Rust compilation check**

Run: `cd crates/storyteller-workshop/src-tauri && cargo check`
Expected: Compiles cleanly.

**Step 3: Run existing workspace tests**

Run: `cargo test --workspace --exclude storyteller-workshop`
Expected: All tests pass (401+).

**Step 4: Launch the app**

Run: `cd crates/storyteller-workshop && cargo tauri dev`

Verify:
- Window opens at 1400x900
- Scene loads and opening prose renders
- Debug panel is visible at bottom with 5 tabs
- Cmd+D toggles panel visibility
- Submit player input — debug tabs populate with phase data
- Phase dots transition: grey → yellow (pulsing) → green
- Context tab shows preamble/journal/retrieved text with token counts
- Narrator tab shows system prompt and raw response
- ML Predictions tab shows "No ML model loaded" notice (unless env vars set)
- Events tab shows "No event classifier loaded" notice (unless env vars set)

**Step 5: Commit any fixes from smoke testing**

If any issues found during smoke test, fix and commit with descriptive message.

---

### Task 8: Final Cleanup and Documentation

**Files:**
- Modify: `docs/plans/2026-03-09-workshop-debug-inspector-design.md` (mark as implemented)

**Step 1: Mark design doc as implemented**

Update the status line:

```markdown
**Status:** Implemented
```

**Step 2: Commit**

```bash
git add docs/plans/2026-03-09-workshop-debug-inspector-design.md
git commit -m "docs: mark debug inspector design as implemented"
```

# Tracing Layer & Resizable Inspector Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Stream structured tracing logs from the Rust engine to a new "Logs" inspector tab with expandable JSON entries, and make the inspector panel drag-resizable.

**Architecture:** A custom `tracing_subscriber::Layer` captures log events from the engine and workshop crates, serializes them to JSON, and emits them over a dedicated Tauri event channel. The frontend renders them as a scrolling log stream with `svelte-json-tree` for expanding individual entries. The inspector panel gains a drag handle for resizing.

**Tech Stack:** Rust `tracing` + `tracing-subscriber`, Tauri events, Svelte 5, `svelte-json-tree`

---

### Task 1: Add svelte-json-tree dependency

**Files:**
- Modify: `crates/storyteller-workshop/package.json`

**Step 1: Install the package**

Run:
```bash
cd crates/storyteller-workshop && bun add svelte-json-tree
```

**Step 2: Verify it installed**

Run:
```bash
grep svelte-json-tree crates/storyteller-workshop/package.json
```
Expected: A line showing `"svelte-json-tree": "..."` in dependencies.

**Step 3: Commit**

```bash
git add crates/storyteller-workshop/package.json crates/storyteller-workshop/bun.lockb
git commit -m "chore(workshop): add svelte-json-tree dependency"
```

---

### Task 2: Add tracing-subscriber to workshop Cargo.toml

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/Cargo.toml:33`

**Step 1: Add the dependency**

Add after the `tracing` line in `[dependencies]`:
```toml
tracing-subscriber = { workspace = true }
```

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p storyteller-workshop
```
Expected: `Finished` with no errors.

**Step 3: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/Cargo.toml Cargo.lock
git commit -m "chore(workshop): add tracing-subscriber dependency"
```

---

### Task 3: Create TauriTracingLayer

**Files:**
- Create: `crates/storyteller-workshop/src-tauri/src/tracing_layer.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/lib.rs:1` (add `mod tracing_layer;`)

This is the core Rust component. It implements `tracing_subscriber::Layer` and emits JSON log events via `tauri::AppHandle::emit()`.

**Step 1: Create the tracing layer module**

Create `crates/storyteller-workshop/src-tauri/src/tracing_layer.rs`:

```rust
//! Custom tracing layer that forwards log events to the Tauri frontend.
//!
//! Events are serialized to JSON and emitted on the `"workshop:logs"` channel.
//! The frontend renders them in the Logs inspector tab.

use std::fmt;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Tauri event channel for log entries.
pub const LOG_EVENT_CHANNEL: &str = "workshop:logs";

/// A single structured log entry emitted to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: serde_json::Value,
}

/// Custom tracing layer that emits log events as Tauri events.
///
/// Clones the `AppHandle` and uses it to emit JSON-serialized log entries
/// on the `"workshop:logs"` channel.
pub struct TauriTracingLayer {
    app_handle: Arc<AppHandle>,
}

impl TauriTracingLayer {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle: Arc::new(app_handle),
        }
    }
}

/// Visitor that collects tracing event fields into a JSON map.
struct JsonVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
    message: Option<String>,
}

impl JsonVisitor {
    fn new() -> Self {
        Self {
            fields: serde_json::Map::new(),
            message: None,
        }
    }
}

impl Visit for JsonVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let val = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(val);
        } else {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::String(val));
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::from(value),
        );
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::from(value),
        );
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::from(value),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::from(value),
        );
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }
}

impl<S: Subscriber> Layer<S> for TauriTracingLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let target = metadata.target();

        // Filter: only storyteller crates, with level thresholds
        let dominated_by = |prefix: &str| target.starts_with(prefix);
        let dominated = dominated_by("storyteller_engine") || dominated_by("storyteller_workshop");
        if !dominated {
            return;
        }

        let level = *metadata.level();
        let deep_target = dominated_by("storyteller_engine::inference")
            || dominated_by("storyteller_engine::agents");

        // DEBUG only for inference and agents targets; INFO+ for everything else
        if !deep_target && level > tracing::Level::INFO {
            return;
        }

        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: level.to_string(),
            target: target.to_string(),
            message: visitor.message.unwrap_or_default(),
            fields: serde_json::Value::Object(visitor.fields),
        };

        let _ = self.app_handle.emit(LOG_EVENT_CHANNEL, &entry);
    }
}
```

**Step 2: Register the module**

In `crates/storyteller-workshop/src-tauri/src/lib.rs`, add at the top with the other `mod` declarations:
```rust
mod tracing_layer;
```

**Step 3: Verify it compiles**

Run:
```bash
cargo check -p storyteller-workshop
```
Expected: `Finished` with no errors.

**Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/tracing_layer.rs crates/storyteller-workshop/src-tauri/src/lib.rs
git commit -m "feat(workshop): create TauriTracingLayer for log streaming"
```

---

### Task 4: Install the tracing layer in lib.rs

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/lib.rs`

The layer must be installed as a `tracing_subscriber` with the Tauri setup hook (so we have access to `AppHandle`).

**Step 1: Rewrite lib.rs run function**

Replace the entire `run()` function in `crates/storyteller-workshop/src-tauri/src/lib.rs`:

```rust
mod commands;
mod engine_state;
mod events;
mod session_log;
mod tracing_layer;

use tokio::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::engine_state::EngineState;
use crate::tracing_layer::TauriTracingLayer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the workspace root (where `bun tauri dev` runs)
    let _ = dotenvy::dotenv();

    // Set up a base tracing subscriber with env filter.
    // The TauriTracingLayer does its own filtering, but we need a base
    // subscriber registered before Tauri starts. We add the Tauri layer
    // in the setup hook once we have an AppHandle.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,storyteller_engine::inference=debug,storyteller_engine::agents=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .compact();

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    registry.init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<EngineState>))
        .setup(|app| {
            // Now we have the AppHandle — install our Tauri tracing layer.
            // We add it as a "reload" layer on top of the existing subscriber.
            let tauri_layer = TauriTracingLayer::new(app.handle().clone());

            // Use tracing::dispatcher to add our layer to the current subscriber.
            // Since tracing_subscriber::registry is already init'd, we use
            // the global dispatch approach via a secondary subscriber.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                // The TauriTracingLayer runs in the existing subscriber context
                // via the Tauri event loop — it's already connected via AppHandle.
                let _ = handle;
            });

            // Register the layer with a supplemental subscriber that forwards
            // to the Tauri frontend.
            let tauri_subscriber = tracing_subscriber::registry().with(tauri_layer);
            let dispatch = tracing::Dispatch::new(tauri_subscriber);

            // Store for later use — events will be emitted via AppHandle.
            app.manage(dispatch);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_llm,
            commands::start_scene,
            commands::submit_input,
            commands::get_session_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**IMPORTANT:** The above approach of adding a layer after init is tricky with tracing's global subscriber. A simpler and more reliable approach: defer the full subscriber setup to the `setup` hook where we have `AppHandle`:

```rust
mod commands;
mod engine_state;
mod events;
mod session_log;
mod tracing_layer;

use tokio::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::engine_state::EngineState;
use crate::tracing_layer::TauriTracingLayer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the workspace root (where `bun tauri dev` runs)
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<EngineState>))
        .setup(|app| {
            // Set up tracing with both a console layer and our Tauri layer.
            // We do this in setup() so we have access to AppHandle.
            let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new(
                    "info,storyteller_engine::inference=debug,storyteller_engine::agents=debug",
                )
            });

            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .compact();

            let tauri_layer = TauriTracingLayer::new(app.handle().clone());

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .with(tauri_layer)
                .init();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_llm,
            commands::start_scene,
            commands::submit_input,
            commands::get_session_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Use the **second version** (setup hook approach). This is cleaner — one subscriber with all layers, initialized once we have the `AppHandle`.

**Step 2: Verify it compiles**

Run:
```bash
cargo check -p storyteller-workshop
```
Expected: `Finished` with no errors.

**Step 3: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/lib.rs
git commit -m "feat(workshop): install TauriTracingLayer in Tauri setup hook"
```

---

### Task 5: Add LogEntry type and LOG_EVENT_CHANNEL to TypeScript

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/types.ts`

**Step 1: Add the types**

Add at the end of `types.ts`:

```typescript
// ---------------------------------------------------------------------------
// Structured log streaming — events arrive on "workshop:logs" channel.
// ---------------------------------------------------------------------------

export const LOG_EVENT_CHANNEL = "workshop:logs";

export interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
  fields: Record<string, unknown>;
}
```

**Note:** There is already a `LogEntry` interface (lines 28-44) used for session logs. Rename the new one to `TracingLogEntry` to avoid collision:

```typescript
export const LOG_EVENT_CHANNEL = "workshop:logs";

export interface TracingLogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
  fields: Record<string, unknown>;
}
```

**Step 2: Commit**

```bash
git add crates/storyteller-workshop/src/lib/types.ts
git commit -m "feat(workshop): add TracingLogEntry type for log streaming"
```

---

### Task 6: Add Logs tab to DebugPanel

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/DebugPanel.svelte`

This is the largest frontend change. Add the Logs tab with:
- Listen on `LOG_EVENT_CHANNEL`
- Ring buffer (500 entries)
- Auto-scroll with pause-on-scroll-up
- Expand entries with svelte-json-tree
- Clear button
- Level color coding

**Step 1: Update the script section**

Add imports at the top of the `<script>` block (after existing imports):

```typescript
import JSONTree from "svelte-json-tree";
import type { TracingLogEntry } from "./types";
import { LOG_EVENT_CHANNEL } from "./types";
```

Add to TABS array — insert `"Logs"` at the end:
```typescript
const TABS = ["LLM", "Context", "ML Predictions", "Characters", "Events", "Narrator", "Logs"] as const;
```

Add `Logs: "logs"` to `TAB_PHASE_MAP`.

Add state variables after `llmChecking`:
```typescript
const MAX_LOG_ENTRIES = 500;
let logEntries: TracingLogEntry[] = $state([]);
let logAutoScroll = $state(true);
let logContainer: HTMLDivElement | undefined = $state(undefined);
let expandedLogIndices: Set<number> = $state(new Set());
```

Add log handler functions:
```typescript
function handleLogEntry(entry: TracingLogEntry) {
    logEntries = [...logEntries, entry].slice(-MAX_LOG_ENTRIES);
    if (logAutoScroll && logContainer) {
        requestAnimationFrame(() => {
            logContainer?.scrollTo({ top: logContainer.scrollHeight });
        });
    }
}

function clearLogs() {
    logEntries = [];
    expandedLogIndices = new Set();
}

function toggleLogExpand(index: number) {
    const next = new Set(expandedLogIndices);
    if (next.has(index)) {
        next.delete(index);
    } else {
        next.add(index);
    }
    expandedLogIndices = next;
}

function handleLogScroll() {
    if (!logContainer) return;
    const { scrollTop, scrollHeight, clientHeight } = logContainer;
    // Auto-scroll if within 20px of bottom
    logAutoScroll = scrollHeight - scrollTop - clientHeight < 20;
}

function levelColor(level: string): string {
    switch (level) {
        case "ERROR": return "log-error";
        case "WARN": return "log-warn";
        case "DEBUG": return "log-debug";
        case "TRACE": return "log-trace";
        default: return "log-info";
    }
}

function shortTimestamp(ts: string): string {
    // Extract HH:MM:SS.mmm from ISO timestamp
    const match = ts.match(/T(\d{2}:\d{2}:\d{2}\.\d{3})/);
    return match ? match[1] : ts;
}
```

Update `onMount` to also listen on the logs channel:
```typescript
onMount(() => {
    let unlistenDebug: UnlistenFn | undefined;
    let unlistenLogs: UnlistenFn | undefined;

    (async () => {
        unlistenDebug = await listen<DebugEvent>(DEBUG_EVENT_CHANNEL, (e) => {
            handleDebugEvent(e.payload);
        });
        unlistenLogs = await listen<TracingLogEntry>(LOG_EVENT_CHANNEL, (e) => {
            handleLogEntry(e.payload);
        });
    })();

    // Run LLM health check immediately
    runLlmCheck();

    return () => {
        unlistenDebug?.();
        unlistenLogs?.();
    };
});
```

Remove the `onDestroy` block (cleanup is now in the `onMount` return).

**Step 2: Add the Logs tab template**

Insert before the closing `{/if}` of the `debug-content` div (after the Narrator tab's `{/if}`, before the error section):

```svelte
{:else if activeTab === "Logs"}
  <div class="debug-tab-content logs-tab">
    <div class="logs-toolbar">
      <span class="log-count">{logEntries.length} entries</span>
      {#if !logAutoScroll}
        <button class="logs-btn" onclick={() => { logAutoScroll = true; logContainer?.scrollTo({ top: logContainer.scrollHeight }); }}>Resume scroll</button>
      {/if}
      <button class="logs-btn" onclick={clearLogs}>Clear</button>
    </div>
    <div
      class="logs-stream"
      bind:this={logContainer}
      onscroll={handleLogScroll}
    >
      {#each logEntries as entry, i}
        <div class="log-line" onclick={() => toggleLogExpand(i)}>
          <span class="log-ts">{shortTimestamp(entry.timestamp)}</span>
          <span class="log-level {levelColor(entry.level)}">{entry.level.substring(0, 4).padEnd(4)}</span>
          <span class="log-target">{entry.target.replace("storyteller_", "")}</span>
          <span class="log-msg">{entry.message}</span>
        </div>
        {#if expandedLogIndices.has(i)}
          <div class="log-expanded">
            <JSONTree value={entry} />
          </div>
        {/if}
      {/each}
    </div>
  </div>
```

**Step 3: Add the Logs tab styles**

Add these styles in the `<style>` block:

```css
.logs-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 0;
}

.logs-toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding-bottom: 0.4rem;
    border-bottom: 1px solid var(--border-debug);
    flex-shrink: 0;
}

.log-count {
    color: var(--text-debug-dim);
    font-size: 0.7rem;
    margin-right: auto;
}

.logs-btn {
    background: var(--bg-debug-tab);
    border: 1px solid var(--border-debug);
    color: var(--text-debug);
    font-family: var(--font-mono);
    font-size: 0.65rem;
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    cursor: pointer;
    box-shadow: none;
}

.logs-btn:hover {
    border-color: var(--accent-dim);
    color: var(--text-primary);
}

.logs-stream {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
    padding-top: 0.25rem;
}

.log-line {
    display: flex;
    gap: 0.5rem;
    padding: 0.1rem 0;
    cursor: pointer;
    font-size: 0.7rem;
    line-height: 1.4;
    border-bottom: 1px solid transparent;
}

.log-line:hover {
    background: var(--bg-debug-tab);
}

.log-ts {
    color: var(--text-debug-dim);
    flex-shrink: 0;
    font-size: 0.65rem;
}

.log-level {
    flex-shrink: 0;
    font-weight: 600;
    font-size: 0.65rem;
    width: 3em;
}

.log-error { color: #d55; }
.log-warn { color: var(--debug-yellow); }
.log-info { color: var(--debug-green); }
.log-debug { color: var(--text-debug-dim); }
.log-trace { color: var(--debug-grey); }

.log-target {
    color: var(--accent);
    flex-shrink: 0;
    max-width: 20em;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.65rem;
}

.log-msg {
    color: var(--text-debug);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.log-expanded {
    padding: 0.25rem 0 0.25rem 1.5rem;
    border-bottom: 1px solid var(--border-debug);
    font-size: 0.7rem;
}
```

**Step 4: Also add global svelte-json-tree theme overrides**

Add to `crates/storyteller-workshop/src/app.css` at the end:

```css
/* svelte-json-tree theme overrides */
:root {
  --json-tree-string-color: #a8cc8c;
  --json-tree-number-color: #dbab79;
  --json-tree-boolean-color: #e88388;
  --json-tree-null-color: #777;
  --json-tree-property-color: #7c9cbf;
  --json-tree-label-color: #ccc;
  --json-tree-arrow-color: #777;
  --json-tree-font-family: var(--font-mono);
  --json-tree-font-size: 0.7rem;
}
```

**Step 5: Verify the app starts**

Run:
```bash
cd crates/storyteller-workshop && bun tauri dev
```
Expected: App launches, "Logs" tab appears in inspector. Logs should stream in as events occur (at minimum during `start_scene` and `submit_input`).

**Step 6: Commit**

```bash
git add crates/storyteller-workshop/src/lib/DebugPanel.svelte crates/storyteller-workshop/src/lib/types.ts crates/storyteller-workshop/src/app.css
git commit -m "feat(workshop): add Logs tab with streaming structured logs and svelte-json-tree"
```

---

### Task 7: Replace JSON.stringify with JSONTree in existing tabs

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/DebugPanel.svelte`

**Step 1: Replace ML Predictions tab JSON dump**

Find (line ~220):
```svelte
<pre>{JSON.stringify(debugState.prediction.resolver_output.original_predictions, null, 2)}</pre>
```
Replace with:
```svelte
<JSONTree value={debugState.prediction.resolver_output.original_predictions} />
```

**Step 2: Replace Characters tab JSON dump**

Find (line ~240):
```svelte
<pre>{JSON.stringify(char, null, 2)}</pre>
```
Replace with:
```svelte
<JSONTree value={char} />
```

**Step 3: Verify the app renders correctly**

Run:
```bash
cd crates/storyteller-workshop && bun tauri dev
```
Expected: Characters and ML Predictions tabs show expandable JSON trees instead of raw text blocks.

**Step 4: Commit**

```bash
git add crates/storyteller-workshop/src/lib/DebugPanel.svelte
git commit -m "refactor(workshop): replace JSON.stringify with svelte-json-tree in inspector tabs"
```

---

### Task 8: Make the inspector panel resizable

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/DebugPanel.svelte`

**Step 1: Add resize state and handlers**

Add to the `<script>` section after existing state variables:

```typescript
let panelHeight = $state(0); // 0 means "use default 25%"
let resizing = $state(false);

function getDefaultHeight(): number {
    return Math.round(window.innerHeight * 0.25);
}

function startResize(e: MouseEvent) {
    e.preventDefault();
    resizing = true;
    if (panelHeight === 0) {
        panelHeight = getDefaultHeight();
    }

    const onMouseMove = (e: MouseEvent) => {
        const newHeight = window.innerHeight - e.clientY;
        const minHeight = 100;
        const maxHeight = Math.round(window.innerHeight * 0.6);
        panelHeight = Math.max(minHeight, Math.min(maxHeight, newHeight));
    };

    const onMouseUp = () => {
        resizing = false;
        window.removeEventListener("mousemove", onMouseMove);
        window.removeEventListener("mouseup", onMouseUp);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
}

function resetHeight() {
    panelHeight = 0;
}
```

**Step 2: Add the drag handle to the template**

In the template, replace:
```svelte
<div class="debug-panel">
```
With:
```svelte
<div
  class="debug-panel"
  style={panelHeight > 0 ? `height: ${panelHeight}px` : undefined}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="resize-handle"
    class:active={resizing}
    onmousedown={startResize}
    ondblclick={resetHeight}
  ></div>
```

**Step 3: Add resize handle styles**

Add to the `<style>` block:

```css
.resize-handle {
    position: absolute;
    top: -4px;
    left: 0;
    right: 0;
    height: 8px;
    cursor: row-resize;
    z-index: 10;
}

.resize-handle:hover,
.resize-handle.active {
    background: var(--accent-dim);
    opacity: 0.5;
}
```

Also update `.debug-panel` to add `position: relative`:

```css
.debug-panel {
    flex-shrink: 0;
    height: 25%;
    background: var(--bg-debug);
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text-debug);
    position: relative;
}
```

**Step 4: Verify resize works**

Run:
```bash
cd crates/storyteller-workshop && bun tauri dev
```
Expected:
- Hovering the top edge of the inspector shows a subtle highlight and row-resize cursor
- Dragging resizes the panel between 100px and 60% viewport
- Double-clicking the handle resets to 25%

**Step 5: Commit**

```bash
git add crates/storyteller-workshop/src/lib/DebugPanel.svelte
git commit -m "feat(workshop): add drag-to-resize handle on inspector panel"
```

---

### Task 9: Smoke test and final commit

**Step 1: Full rebuild and test**

Run:
```bash
cargo check --workspace --all-features
cd crates/storyteller-workshop && bun tauri dev
```

Verify:
1. App launches, LLM tab shows Ollama reachable
2. Scene opens with narrator prose
3. All existing inspector tabs work (LLM, Context, ML Predictions, Characters, Events, Narrator)
4. Logs tab streams structured log entries in real-time
5. Clicking a log entry expands it with JSON tree
6. Characters and ML Predictions tabs show JSON trees
7. Inspector panel is drag-resizable
8. Double-click handle resets to 25%
9. Clear button in Logs tab works
10. Auto-scroll pauses when scrolled up, "Resume scroll" button appears

**Step 2: Push**

```bash
git push
```

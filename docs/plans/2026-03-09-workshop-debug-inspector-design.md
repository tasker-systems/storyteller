# Workshop Debug Inspector

**Date:** 2026-03-09
**Branch:** `jcoletaylor/workshopping-even-more`
**Status:** Implemented

## Problem

The storyteller-workshop Tauri app provides a working gameplay loop but no real-time observability into the turn pipeline. During development of the initial prototype, debugging issues (like Ollama timeouts) required guesswork because there was no visibility into which phase was executing or had failed. Session data logs to JSONL on disk but isn't surfaced in the UI.

## Solution

A collapsible debug inspector panel at the bottom of the workshop window — Chrome DevTools-style — with tabbed views for each pipeline phase. Real-time updates via Tauri events emitted during turn processing.

## Layout

- **Window size:** 1400x900 (up from 1200x800)
- **Structure:** Story pane (top) + collapsible debug panel (bottom, ~40% when open)
- **Toggle:** Header button + Cmd+D keyboard shortcut
- **Collapsed state:** Story pane reclaims full height — pure player experience
- **Story pane:** Unchanged, 680px max-width centered layout
- **Debug panel:** Full window width (debug data benefits from horizontal space)

## Debug Panel Tabs

| Tab | Content | Populates on event |
|-----|---------|-------------------|
| Context | Preamble text, journal entries, retrieved context — the actual strings the LLM sees. Token counts per tier with bar visualization | `turn:context-assembled` |
| ML Predictions | ResolverOutput — sequenced character actions, behavior predictions, scene dynamics. Disabled when no models loaded | `turn:prediction-complete` |
| Characters | Character sheets, tensor summary, emotional markers extracted from player input | `turn:characters-updated` |
| Events | Event classification results — how player input was typed/categorized. Disabled when no classifier loaded | `turn:events-classified` |
| Narrator | Raw CompletionRequest (system prompt + messages) sent to Ollama, raw response. Temperature, model name, timing | `turn:narrator-complete` |

Each tab displays a **phase indicator**: pending (grey), processing (pulsing), complete (green), skipped (dimmed). During a turn, you see at a glance which phases have completed.

Tabs show data from the **most recent turn**. Historical data remains in JSONL logs.

## Tauri Event System

All debug events are emitted on a single channel (`"workshop:debug"`) with a consistent envelope structure. The Rust backend uses a `#[serde(tag = "type")]` enum so each event is a JSON object with a `type` discriminator field and phase-specific payload fields alongside common metadata.

**Envelope structure:**
```json
{
  "type": "phase_started | prediction_complete | context_assembled | characters_updated | events_classified | narrator_complete | error",
  "turn": 1,
  ...phase-specific fields
}
```

**Event types and their payloads:**

| type | Additional fields |
|------|------------------|
| `phase_started` | `phase` |
| `prediction_complete` | `resolver_output`, `timing_ms`, `model_loaded` |
| `context_assembled` | `preamble_text`, `journal_text`, `retrieved_text`, `token_counts`, `timing_ms` |
| `characters_updated` | `characters`, `emotional_markers` |
| `events_classified` | `classifications`, `classifier_loaded` |
| `narrator_complete` | `system_prompt`, `user_message`, `raw_response`, `model`, `temperature`, `max_tokens`, `tokens_used`, `timing_ms` |
| `error` | `phase`, `message` |

**Design constraints:**
- Single Tauri event name (`"workshop:debug"`) — one `listen()` on frontend, one enum on backend
- Events are additive — the existing `submit_input` command still returns `TurnResult`
- The story pane's async/await flow is unchanged
- Single listener registered once on mount, dispatches on `type` field
- Events fire regardless of panel visibility (cheap IPC, no rendering cost when collapsed)
- `error` type provides immediate visibility into which phase failed
- Adding new phases means adding a variant to the Rust enum and a case to the frontend switch

## Frontend Components

**New:**
- `DebugPanel.svelte` — Container with tab bar and collapsible wrapper. Manages active tab, listens to Tauri events, routes payloads to tab data stores
- `DebugTab.svelte` — Generic tab content wrapper (phase indicator + scrollable content)
- `ContextTab.svelte` — Preamble/journal/retrieved text blocks with token count bars
- `PredictionsTab.svelte` — ResolverOutput display (character actions, predictions, dynamics)
- `CharactersTab.svelte` — Character sheets, tensor summaries, emotional markers
- `EventsTab.svelte` — Event classification results
- `NarratorTab.svelte` — Raw prompt/response with model metadata

**Modified:**
- `+page.svelte` — Adds DebugPanel, toggle button in header, Cmd+D shortcut
- `app.css` — Debug panel CSS variables and theming

## Styling

The debug panel shares the dark theme but is visually distinct as tooling:

- **Background:** Slightly lighter (`#1a1a1a` vs `#111` for story area)
- **Font:** Monospace/Courier — terminal feel, distinct from narrative serif
- **Tabs:** Underline active, dim inactive. Minimal chrome
- **Phase indicators:** Small colored dots or badges (grey/pulsing/green/dimmed)
- **Data display:** Preformatted text blocks for raw data, structured tables for token counts and timing

## Backend Changes

- Pass `AppHandle` into turn processing to emit events at phase boundaries
- Single `DebugEvent` enum with `#[serde(tag = "type")]` for all event payloads
- Emit events from `commands.rs` between existing processing phases
- No changes to `EngineState`, `SessionLog`, or the `TurnResult` return type

## Non-Goals (for now)

- Draggable/resizable split (fixed 40% is fine for a workshop tool)
- Historical turn browser in the inspector (use JSONL logs)
- Filtering or search within tabs
- Export from inspector (JSONL already serves this)

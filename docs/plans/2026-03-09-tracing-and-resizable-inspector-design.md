# Tracing Layer & Resizable Inspector Design

**Date:** 2026-03-09
**Status:** Implemented
**Branch:** `jcoletaylor/workshopping-even-more`

## Overview

Add real-time structured log streaming to the workshop inspector via a custom `tracing_subscriber::Layer`, and make the inspector panel resizable via drag handle.

## 1. Tracing Layer

### Rust: TauriTracingLayer

A custom `tracing_subscriber::Layer` installed in `lib.rs` alongside `dotenvy`, before `tauri::Builder`.

**Filtering:**
- INFO and above from `storyteller_engine` and `storyteller_workshop`
- DEBUG from `storyteller_engine::inference` and `storyteller_engine::agents`

**Event serialization:** Each tracing event becomes a JSON object:
```json
{
  "timestamp": "2026-03-09T20:15:00.123Z",
  "level": "DEBUG",
  "target": "storyteller_engine::inference::external",
  "message": "sending request to Ollama",
  "fields": { "model": "qwen2.5:14b", "url": "http://127.0.0.1:11434/api/chat" }
}
```

**Emission:** Via `app_handle.emit()` on channel `"workshop:logs"` — separate from `"workshop:debug"` to avoid mixing high-frequency log events with phase-boundary debug events.

### Frontend: Logs Inspector Tab

New "Logs" tab in the inspector, chrome devtools console style:

- Listens on `"workshop:logs"` channel
- Appends entries to a bounded ring buffer (500 entries max, drop oldest)
- Each entry renders as a single collapsed line: `[timestamp] [level] target — message`
- Clicking/expanding a line shows full JSON fields via `svelte-json-tree`
- Auto-scrolls to bottom unless user has scrolled up (implicit pause)
- Clear button to reset the buffer

## 2. Resizable Inspector

### Drag Handle

A 4px-tall div at the top edge of the debug panel. Cursor changes to `row-resize` on hover. Hit target padded to 8-10px for comfort.

### Behavior

- `mousedown` starts tracking, `mousemove` updates height, `mouseup` stops
- Panel height stored as pixels (not percentage — avoids jitter on window resize)
- Constraints: min 100px, max 60% viewport height
- Default: 25% viewport (current behavior)
- Double-click handle to reset to default 25%
- Collapse toggle preserves last dragged height on re-open

### Implementation

Pure Svelte pointer events on the handle div. `panelHeight` state variable drives inline style. No library needed.

## 3. svelte-json-tree Integration

**Package:** `svelte-json-tree` via `bun add svelte-json-tree`.

**Usage:**
- Logs tab: expanded log entries render JSON fields
- Existing inspector tabs: replace `JSON.stringify` in `<pre>` blocks (Characters, ML Predictions) with `<JSONTree>` for consistent expand/collapse

**Theming:** Styled via CSS custom properties to match inspector dark/monospace aesthetic.

## Future Considerations (not in scope)

- Configurable log level filters from the UI
- Multi-view shell (Play / Characters / Graph / ...) — separate design
- Setting and location awareness panel

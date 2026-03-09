# Workshop UI Design: Tauri+Svelte Prototype

**Date**: March 8, 2026
**Branch**: `jcoletaylor/lemme-see`
**Goal**: A playable prototype UI for the storyteller engine — something to learn and think with, not a production interface.

---

## Motivation

The storyteller engine has a working turn pipeline (ML classification, context assembly, narrator LLM rendering) accessible only through a stdin/stdout binary (`play_scene_context`). The project is stalled on theoretical work (graph theory, character tensors, persistence design) because there's no interactive surface for experiencing what the system actually does.

This prototype unblocks iteration by providing a visual interface for the existing gameplay loop. It is explicitly not the production UI — it's a workbench.

---

## Technology Choices

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **App shell** | Tauri v2 | Rust backend integrates directly with engine code. Desktop app feel. Native async command support. |
| **Frontend** | Svelte + TypeScript | Compiler-first philosophy aligns with project values. Fast iteration with Vite. |
| **Bundler** | Vite | Standard Tauri+Svelte toolchain. Fast HMR for UI iteration. |
| **Package manager** | bun | Already in active use. Fast installs and script execution. |
| **Backend communication** | Tauri commands (direct) | Engine functions called in-process via Tauri's command system. No HTTP/WebSocket plumbing needed. |
| **Engine integration** | Direct function calls | Modeled after `play_scene_context.rs`. No Bevy ECS loop — call pipeline functions directly. |

### What We Considered and Set Aside

- **ratatui TUI**: Terminal-native but limited for collapsible sections, rich text, and the Claude Desktop-like interaction model described.
- **Full Bevy UI**: Would require adding the rendering pipeline to a headless-only Bevy setup. Designed for game UIs, not text-heavy document layouts.
- **Axum-served web app**: Simpler setup but loses desktop app feel and requires WebSocket/SSE plumbing for streaming.
- **Tauri as shell over separate engine process**: Over-architected for a prototype. Can always extract later.

---

## Project Structure

A new crate `crates/storyteller-workshop`, scaffolded via `create-tauri-app` and adjusted to fit the workspace:

```
crates/storyteller-workshop/
├── Cargo.toml              # Tauri app, depends on storyteller-core + storyteller-engine
├── tauri.conf.json         # Tauri configuration
├── src/
│   ├── main.rs             # Tauri entry point
│   ├── commands.rs         # Tauri commands (start_scene, submit_input, get_session_log)
│   └── session_log.rs      # Per-turn JSONL log writer
├── ui/                     # Svelte+Vite frontend
│   ├── package.json
│   ├── svelte.config.js
│   ├── vite.config.ts
│   ├── src/
│   │   ├── App.svelte      # Root layout
│   │   ├── lib/
│   │   │   ├── StoryPane.svelte    # Narrator prose + collapsed player turns
│   │   │   ├── InputBar.svelte     # Player text input
│   │   │   └── types.ts           # TypeScript types mirroring Rust structs
│   │   └── main.ts
│   └── index.html
└── sessions/               # Output directory for session logs (gitignored)
```

---

## Tauri Commands

Three commands for the initial prototype:

### `start_scene() -> SceneInfo`

Loads the workshop scene data (The Flute Kept, Bramblehoof, Pyotir), initializes the Ollama LLM provider, creates the scene journal, optionally loads ML models. Returns scene metadata (title, setting description, cast names, constraints) for the UI to render an opening state.

Engine state (scene data, journal, LLM provider, character sheets) is held in Tauri's managed state, initialized by this command.

### `submit_input(text: String) -> TurnResult`

Runs the turn pipeline:
1. ML character prediction (if model available)
2. Three-tier context assembly (preamble + journal + retrieved)
3. Single narrator LLM call (async, hitting Ollama)
4. Journal update with emotional markers
5. Session log append

Returns: narrator prose, player input echo, turn number, timing data, context assembly token counts, ML classification info.

### `get_session_log() -> Vec<LogEntry>`

Returns accumulated session log entries for debugging and shared review.

---

## Frontend Design

### Story Pane

A scrolling container rendering the narrative turn by turn:

- **Narrator prose**: Dominant visual element. Generous line height, readable font. Formatted text (plain text initially, eventually markdown-capable). This should feel like reading a story.
- **Player input**: Collapsible inline block between narrator sections. Collapsed by default after submission (one-line summary). Expandable on click. Visually subordinate — lighter color, smaller text, offset.
- **Loading state**: Subtle indicator while the narrator generates. Communicates "the story is being told" without feeling like a chat spinner.

### Input Bar

Fixed at bottom. A text area (not single-line — players may write paragraph-length action descriptions). Submit on Enter, Shift+Enter for newlines. Disabled during narrator generation.

### Initial State

Before the first turn, the pane shows scene opening from `start_scene` response: title, setting description, cast introduction.

### Scrolling Behavior

Auto-scrolls to newest content after narrator response. Pauses auto-scroll when user scrolls up to read history. Resumes when user scrolls back to bottom.

---

## Session Logging

Each turn appends a JSON record to `sessions/<session-id>.jsonl`:

```json
{
  "turn": 1,
  "timestamp": "2026-03-08T20:15:03Z",
  "player_input": "I approach the smallholding slowly...",
  "narrator_output": "The last light catches the stone wall...",
  "context_assembly": {
    "preamble_tokens": 650,
    "journal_tokens": 0,
    "retrieved_tokens": 380,
    "total_tokens": 1030
  },
  "timing": {
    "prediction_ms": 45,
    "assembly_ms": 12,
    "narrator_ms": 2340
  },
  "classifications": [],
  "predictions": []
}
```

Session ID is timestamp-based and human-readable (e.g., `2026-03-08-2015-the-flute-kept`). The `sessions/` directory is gitignored.

This gives a shared artifact for collaborative debugging — session files can be read directly or shared for review.

---

## Explicit Non-Goals

To keep the prototype honest and right-sized:

- **No side panels** (entity reference, system tracing) — future iteration
- **No persistence or database** — everything in-memory per session
- **No scene-to-scene transitions** — single scene only
- **No Bevy ECS loop** — direct function calls like play-scene
- **No voice input, image upload, or streaming** — text in, text out
- **No production styling** — functional and readable, not polished
- **No tests for the UI layer** — this is a prototype for learning

---

## Future Possibilities (Not Commitments)

Once the prototype is working, the web renderer opens up visualization options that would be difficult in a terminal or custom Rust renderer:

- **D3.js** for graph topology visualization (relational web, narrative graph)
- **Entity reference panel** with live tensor state
- **Context assembly inspector** showing what the narrator actually sees
- **Turn rollback and replay** for iterating on context assembly
- **Voice-to-text input** via Web Speech API
- **Streaming narrator output** via Tauri events

These are noted as motivation for the technology choice, not as scope for this prototype.

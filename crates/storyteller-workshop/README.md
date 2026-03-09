# storyteller-workshop

Interactive playtesting UI for the storyteller engine — a Tauri 2 desktop app with a SvelteKit frontend.

## What This Is

The workshop provides a visual interface for running scenes against a local LLM (Ollama), observing the narrative pipeline in real time via a Chrome DevTools-style debug inspector, and iterating on prompts and models during development.

### Features

- **Scene playback** — load a scene, type narrative directions, see narrator prose rendered in real time
- **Debug inspector** (Cmd+D to toggle) — 7-tab panel showing pipeline internals:
  - **LLM** — Ollama connectivity status, configured model, available models
  - **Context** — three-tier context assembly (preamble, journal, retrieved) with token counts
  - **ML Predictions** — character behavior predictions and scene dynamics
  - **Characters** — character sheets and emotional markers (JSON tree view)
  - **Events** — event classification results
  - **Narrator** — raw LLM request/response, timing, token usage
  - **Logs** — real-time structured log stream from Rust via custom `tracing_subscriber::Layer`
- **Resizable inspector** — drag the panel edge to resize, double-click to reset
- **Session logging** — JSONL session logs written to `sessions/` for later analysis

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Bun](https://bun.sh/) (for the SvelteKit frontend)
- [Ollama](https://ollama.com/) running locally with a model pulled
- System dependencies for Tauri 2 (see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

## Getting Started

From the **workspace root** (`storyteller/`):

```bash
# 1. Copy and configure environment variables
cp .env.example .env
# Edit .env — set STORYTELLER_DATA_PATH and STORYTELLER_MODEL_PATH if you have trained models

# 2. Ensure Ollama is running with a model
ollama serve                    # Start the server (if not already running)
ollama pull qwen2.5:14b         # Pull the default model (or any supported model)

# 3. Install frontend dependencies
cd crates/storyteller-workshop
bun install

# 4. Launch the workshop (builds Rust backend + starts Vite dev server)
bun tauri dev
```

The app loads `.env` from the workspace root automatically via `dotenvy` — no need to copy environment files into the workshop directory.

## Development

```bash
# Frontend only (no Tauri shell)
bun dev

# Type checking
bun check

# Rust tests (from workspace root)
cargo test -p storyteller-workshop

# Full workspace quality checks (from workspace root)
cargo make check
```

## Architecture

```
storyteller-workshop/
├── src/                        # SvelteKit frontend
│   ├── routes/+page.svelte     # Main play view
│   └── lib/
│       ├── DebugPanel.svelte   # 7-tab debug inspector
│       ├── types.ts            # TypeScript types for debug events and logs
│       └── api.ts              # Tauri command wrappers
├── src-tauri/                  # Rust backend (Tauri 2)
│   └── src/
│       ├── lib.rs              # App setup, tracing subscriber
│       ├── commands.rs         # Tauri commands (start_scene, submit_input, check_llm)
│       ├── events.rs           # DebugEvent tagged enum
│       ├── tracing_layer.rs    # TauriTracingLayer — streams logs to frontend
│       ├── engine_state.rs     # Shared engine state across commands
│       └── session_log.rs      # JSONL session logging
├── package.json
└── svelte.config.js
```

### Event Channels

The backend communicates with the frontend via two Tauri event channels:

| Channel | Purpose | Frequency |
|---------|---------|-----------|
| `workshop:debug` | Pipeline phase events (predictions, context, narrator) | Per phase per turn |
| `workshop:logs` | Structured tracing log entries | High frequency |

Both use tagged JSON — `DebugEvent` variants carry a `"type"` discriminator for frontend dispatch.

## IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

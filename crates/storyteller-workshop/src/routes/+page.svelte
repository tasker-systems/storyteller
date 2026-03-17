<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { checkHealth, submitInput, resumeSession } from "$lib/api";
  import type { StoryBlock } from "$lib/types";
  import { GAMEPLAY_CHANNEL, type GameplayEvent } from "$lib/types";
  import type { SceneInfo, ResumeResult } from "$lib/generated";
  import { hydrateBlocks } from "$lib/logic";
  import StoryPane from "$lib/StoryPane.svelte";
  import InputBar from "$lib/InputBar.svelte";
  import DebugPanel from "$lib/DebugPanel.svelte";
  import SceneSetup from "$lib/SceneSetup.svelte";
  import SessionPanel from "$lib/SessionPanel.svelte";

  let view: "setup" | "playing" = $state("setup");
  let sceneInfo: SceneInfo | null = $state(null);
  let sessionId = $state("");
  let blocks: StoryBlock[] = $state([]);
  let loading = $state(false);
  let gameplayLoading = $state(false);
  let error: string | null = $state(null);
  let turnCount = $state(0);
  let debugVisible = $state(true);
  let sessionPanelVisible = $state(false);
  let sceneMetadata = $state<{
    player_character: string;
    player_intent: string | null;
    cast_names: string[];
  } | null>(null);

  onMount(() => {
    function handleKeydown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "d") {
        e.preventDefault();
        debugVisible = !debugVisible;
      }
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        sessionPanelVisible = !sessionPanelVisible;
      }
    }
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });

  $effect(() => {
    const unlisten = listen<GameplayEvent>(GAMEPLAY_CHANNEL, (event) => {
      const payload = event.payload;
      switch (payload.kind) {
        case "NarratorProse":
          appendProseChunk(payload.chunk, payload.turn);
          break;
        case "NarratorComplete":
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
          sceneMetadata = {
            player_character: payload.player_character,
            player_intent: payload.player_intent,
            cast_names: payload.cast_names,
          };
          break;
        case "ProcessingUpdate":
          // Future: update processing indicator
          break;
      }
    });
    return () => { unlisten.then(fn => fn()); };
  });

  function appendProseChunk(chunk: string, turn: number) {
    const existingIdx = blocks.findIndex((b) => {
      if (turn === 0 && b.kind === "opening") return true;
      if (b.kind === "narrator" && b.turn === turn) return true;
      return false;
    });
    if (existingIdx >= 0) {
      blocks[existingIdx] = {
        ...blocks[existingIdx],
        text: blocks[existingIdx].text + chunk,
      };
    } else if (turn === 0) {
      blocks.push({ kind: "opening", text: chunk });
    } else {
      blocks.push({ kind: "narrator", turn, text: chunk });
    }
  }

  function reconcileNarratorBlock(prose: string, turn: number) {
    const existingIdx = blocks.findIndex((b) => {
      if (b.kind === "opening") return turn === 0;
      if (b.kind === "narrator") return b.turn === turn;
      return false;
    });
    if (existingIdx >= 0) {
      blocks[existingIdx] = { ...blocks[existingIdx], text: prose };
    } else if (turn === 0) {
      blocks.push({ kind: "opening", text: prose });
    } else {
      blocks.push({ kind: "narrator", turn, text: prose });
    }
  }

  async function checkServerHealthy(): Promise<boolean> {
    try {
      const report = await checkHealth();
      if (report.status !== "Healthy") {
        const unhealthy = report.subsystems.filter((s) => s.status !== "Healthy");
        const details = unhealthy.map((s) => `${s.name}: ${s.message ?? s.status}`).join("; ");
        error = `Server unhealthy: ${details || report.status}`;
        return false;
      }
      return true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      return false;
    }
  }

  function transitionToPlaying(info: SceneInfo) {
    sceneInfo = info;
    sessionId = info.session_id;
    // Do not pre-populate blocks from opening_prose — the gameplay channel already
    // received NarratorProse/NarratorComplete events during compose_scene streaming
    // and built the opening block incrementally. Resetting blocks here would discard
    // that work. If the gameplay channel somehow missed the events, blocks stays []
    // and the scene still opens (just without prose until the next event or reload).
    turnCount = 0;
    error = null;
    loading = false;
    view = "playing";
  }

  async function handleSceneLaunched(info: SceneInfo) {
    transitionToPlaying(info);
  }

  function hydrateFromResumeResult(result: ResumeResult) {
    sceneInfo = result.scene_info;
    sessionId = result.scene_info.session_id;
    const hydrated = hydrateBlocks(result);
    blocks = hydrated.blocks;
    turnCount = hydrated.turnCount;
    error = null;
    loading = false;
    view = "playing";
  }

  async function handleResumeSession(sessionId: string) {
    loading = true;
    error = null;
    try {
      if (!(await checkServerHealthy())) {
        loading = false;
        return;
      }
      const result = await resumeSession(sessionId);
      hydrateFromResumeResult(result);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      loading = false;
    }
  }

  function handleNewScene() {
    view = "setup";
    sceneInfo = null;
    sessionId = "";
    blocks = [];
    turnCount = 0;
    sceneMetadata = null;
    error = null;
    loading = false;
  }

  async function handleSubmit(text: string) {
    turnCount += 1;
    const playerTurn = turnCount;

    blocks = [...blocks, { kind: "player", turn: playerTurn, text }];
    loading = true;
    error = null;

    try {
      // submitInput triggers server-side processing that emits gameplay channel events.
      // The gameplay channel listener (NarratorProse/NarratorComplete) handles narrator
      // block rendering incrementally. We do NOT append from the TurnResult return value
      // here — that would duplicate the narrator output already built by the channel.
      await submitInput(sessionId, text);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="app-layout">
  <header class="app-header">
    <button
      class="sidebar-toggle"
      onclick={() => (sessionPanelVisible = !sessionPanelVisible)}
      title={sessionPanelVisible ? "Hide sessions (\u2318S)" : "Show sessions (\u2318S)"}
    >
      {sessionPanelVisible ? "\u25C0" : "\u25B6"} Sessions
    </button>

    <h1 class="scene-title">
      {#if view === "playing"}
        {sceneInfo?.title ?? "Loading..."}
      {:else}
        Scene Setup
      {/if}
    </h1>

    <div class="header-actions">
      {#if view === "playing"}
        <button class="header-btn" onclick={handleNewScene}>New Scene</button>
      {/if}
      <button
        class="debug-toggle"
        onclick={() => (debugVisible = !debugVisible)}
        title={debugVisible ? "Hide inspector (\u2318D)" : "Show inspector (\u2318D)"}
      >
        {debugVisible ? "\u25BC" : "\u25B2"} Inspector
      </button>
    </div>
  </header>

  {#if error}
    <div class="error-banner">
      <span class="error-text">{error}</span>
      <button class="error-dismiss" onclick={() => (error = null)}>dismiss</button>
    </div>
  {/if}

  <div class="app-body">
    <SessionPanel
      visible={sessionPanelVisible}
      onNewSession={handleNewScene}
      onResumeSession={handleResumeSession}
    />

    <div class="main-content">
      {#if view === "setup"}
        <div class="setup-container">
          <SceneSetup onlaunch={handleSceneLaunched} />
        </div>
      {:else}
        <StoryPane {blocks} loading={loading || gameplayLoading} />
        {#if sceneMetadata?.player_intent}
          <div class="player-intent-bar">
            <span class="intent-label">Playing as {sceneMetadata.player_character}:</span>
            <span class="intent-text">{sceneMetadata.player_intent}</span>
          </div>
        {:else if sceneMetadata?.player_character}
          <div class="player-intent-bar">
            <span class="intent-label">Playing as {sceneMetadata.player_character}</span>
          </div>
        {/if}
        <InputBar disabled={loading || gameplayLoading} onsubmit={handleSubmit} />
      {/if}
    </div>
  </div>

  <DebugPanel visible={debugVisible && view === "playing"} />
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

  .sidebar-toggle {
    position: absolute;
    left: 1rem;
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

  .sidebar-toggle:hover {
    color: var(--text-primary);
    border-color: var(--accent-dim);
  }

  .header-actions {
    position: absolute;
    right: 1rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-btn {
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

  .header-btn:hover {
    color: var(--text-primary);
    border-color: var(--accent-dim);
  }

  .debug-toggle {
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

  .app-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .setup-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    overflow-y: auto;
    padding: 2rem 1rem;
  }

  .player-intent-bar {
    padding: 0.3rem 1.5rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    max-width: 816px;
    margin: 0 auto;
    width: 100%;
  }

  .intent-label {
    font-style: italic;
  }

  .intent-text {
    margin-left: 0.3rem;
  }

</style>

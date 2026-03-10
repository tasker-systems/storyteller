<script lang="ts">
  import { onMount } from "svelte";
  import { checkLlm, startScene, submitInput, resumeSession } from "$lib/api";
  import type { StoryBlock, SceneInfo } from "$lib/types";
  import StoryPane from "$lib/StoryPane.svelte";
  import InputBar from "$lib/InputBar.svelte";
  import DebugPanel from "$lib/DebugPanel.svelte";
  import SceneSetup from "$lib/SceneSetup.svelte";
  import SessionPanel from "$lib/SessionPanel.svelte";

  let view: "setup" | "playing" = $state("setup");
  let sceneInfo: SceneInfo | null = $state(null);
  let blocks: StoryBlock[] = $state([]);
  let loading = $state(false);
  let error: string | null = $state(null);
  let turnCount = $state(0);
  let debugVisible = $state(true);
  let sessionPanelVisible = $state(false);

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

  async function checkLlmReachable(): Promise<boolean> {
    try {
      const llm = await checkLlm();
      if (!llm.reachable) {
        error = `LLM unreachable at ${llm.endpoint}: ${llm.error ?? "unknown error"}. Start Ollama and reload.`;
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
    blocks = [{ kind: "opening", text: info.opening_prose }];
    turnCount = 0;
    error = null;
    loading = false;
    view = "playing";
  }

  async function handleSceneLaunched(info: SceneInfo) {
    transitionToPlaying(info);
  }

  async function handleResumeSession(sessionId: string) {
    loading = true;
    error = null;
    try {
      if (!(await checkLlmReachable())) {
        loading = false;
        return;
      }
      const info = await resumeSession(sessionId);
      transitionToPlaying(info);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      loading = false;
    }
  }

  async function handleClassicStart() {
    loading = true;
    error = null;
    try {
      if (!(await checkLlmReachable())) {
        loading = false;
        return;
      }
      const info = await startScene();
      transitionToPlaying(info);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      loading = false;
    }
  }

  function handleNewScene() {
    view = "setup";
    sceneInfo = null;
    blocks = [];
    turnCount = 0;
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
          <div class="classic-fallback">
            <span class="fallback-divider">or</span>
            <button
              class="classic-btn"
              onclick={handleClassicStart}
              disabled={loading}
            >
              {loading ? "Starting..." : "Classic: The Flute Kept"}
            </button>
          </div>
        </div>
      {:else}
        <StoryPane {blocks} {loading} />
        <InputBar disabled={loading} onsubmit={handleSubmit} />
      {/if}
    </div>
  </div>

  {#if view === "playing"}
    <DebugPanel visible={debugVisible} />
  {/if}
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

  .classic-fallback {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    margin-top: 1.5rem;
  }

  .fallback-divider {
    color: var(--text-secondary);
    font-size: 0.8rem;
    font-style: italic;
  }

  .classic-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    padding: 0.4rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    box-shadow: none;
  }

  .classic-btn:hover:not(:disabled) {
    color: var(--text-primary);
    border-color: var(--accent-dim);
  }

  .classic-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>

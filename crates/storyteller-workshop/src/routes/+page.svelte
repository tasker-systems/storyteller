<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { startScene, submitInput } from "$lib/api";
  import type { StoryBlock, SceneInfo } from "$lib/types";
  import StoryPane from "$lib/StoryPane.svelte";
  import InputBar from "$lib/InputBar.svelte";

  let sceneInfo: SceneInfo | null = $state(null);
  let blocks: StoryBlock[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let turnCount = $state(0);
  let diagnosticMsg: string | null = $state(null);

  onMount(async () => {
    // Diagnostic: test Ollama connectivity first
    try {
      const result = await invoke<string>("test_ollama");
      diagnosticMsg = result;
    } catch (e) {
      diagnosticMsg = `Ollama test FAILED: ${e}`;
      error = `Ollama connectivity failed: ${e}`;
      loading = false;
      return;
    }

    try {
      const info = await startScene();
      sceneInfo = info;
      blocks = [{ kind: "opening", text: info.opening_prose }];
      loading = false;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      loading = false;
    }
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
  </header>

  {#if diagnosticMsg}
    <div class="diagnostic-banner">
      <span class="diagnostic-text">{diagnosticMsg}</span>
    </div>
  {/if}

  {#if error}
    <div class="error-banner">
      <span class="error-text">{error}</span>
      <button class="error-dismiss" onclick={() => (error = null)}>dismiss</button>
    </div>
  {/if}

  <StoryPane {blocks} {loading} />

  <InputBar disabled={loading} onsubmit={handleSubmit} />
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

  .diagnostic-banner {
    background: #152a15;
    border-bottom: 1px solid #204a20;
    padding: 0.4rem 1.5rem;
    flex-shrink: 0;
  }

  .diagnostic-text {
    color: #8d8;
    font-size: 0.8rem;
    font-family: monospace;
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

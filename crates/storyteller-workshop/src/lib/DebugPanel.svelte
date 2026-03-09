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

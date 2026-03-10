<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import JSONTree from "svelte-json-tree";
  import type { DebugState, DebugEvent, PhaseStatus, LlmStatus, TracingLogEntry, EventDecomposedEvent, ActionArbitratedEvent } from "./types";
  import { DEBUG_EVENT_CHANNEL, LOG_EVENT_CHANNEL } from "./types";
  import { checkLlm } from "./api";

  let { visible }: { visible: boolean } = $props();

  const TABS = ["LLM", "Context", "ML Predictions", "Characters", "Events", "Arbitration", "Narrator", "Logs"] as const;
  type TabName = (typeof TABS)[number];
  const TAB_PHASE_MAP: Record<TabName, string> = {
    LLM: "llm",
    Context: "context",
    "ML Predictions": "prediction",
    Characters: "characters",
    Events: "events",
    Arbitration: "arbitration",
    Narrator: "narrator",
    Logs: "logs",
  };

  let activeTab: TabName = $state("LLM");
  let llmStatus: LlmStatus | null = $state(null);
  let llmChecking = $state(false);
  const MAX_LOG_ENTRIES = 500;
  let logEntries: TracingLogEntry[] = $state([]);
  let logAutoScroll = $state(true);
  let logContainer: HTMLDivElement | undefined = $state(undefined);
  let expandedLogIndices: Set<number> = $state(new Set());
  let debugState: DebugState = $state({
    turn: 0,
    phases: {},
    prediction: null,
    context: null,
    characters: null,
    events: null,
    decomposition: null,
    arbitration: null,
    narrator: null,
    error: null,
  });

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

  function resetForTurn(turn: number) {
    debugState = {
      turn,
      phases: {},
      prediction: null,
      context: null,
      characters: null,
      events: null,
      decomposition: null,
      arbitration: null,
      narrator: null,
      error: null,
    };
  }

  function phaseStatus(tab: TabName): PhaseStatus {
    if (tab === "LLM") {
      if (llmChecking) return "processing";
      if (!llmStatus) return "pending";
      return llmStatus.reachable ? "complete" : "error";
    }
    if (tab === "Events") {
      // Events tab combines classification + decomposition phases
      const evtStatus = debugState.phases["events"] ?? "pending";
      const decStatus = debugState.phases["decomposition"] ?? "pending";
      if (evtStatus === "error" || decStatus === "error") return "error";
      if (evtStatus === "processing" || decStatus === "processing") return "processing";
      if (evtStatus === "complete" && decStatus === "complete") return "complete";
      if (evtStatus === "complete" || decStatus === "complete") return "processing";
      return "pending";
    }
    const phase = TAB_PHASE_MAP[tab];
    return debugState.phases[phase] ?? "pending";
  }

  async function runLlmCheck() {
    llmChecking = true;
    try {
      llmStatus = await checkLlm();
    } catch (e) {
      llmStatus = {
        reachable: false,
        endpoint: "unknown",
        model: "unknown",
        provider: "Ollama",
        available_models: [],
        error: e instanceof Error ? e.message : String(e),
        latency_ms: 0,
      };
    }
    llmChecking = false;
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
      case "event_decomposed":
        debugState.decomposition = event as EventDecomposedEvent;
        debugState.phases["decomposition"] = event.error ? "error" : "complete";
        break;
      case "action_arbitrated":
        debugState.arbitration = event as ActionArbitratedEvent;
        debugState.phases["arbitration"] = "complete";
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
    const match = ts.match(/T(\d{2}:\d{2}:\d{2}\.\d{3})/);
    return match ? match[1] : ts;
  }

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

    runLlmCheck();

    return () => {
      unlistenDebug?.();
      unlistenLogs?.();
    };
  });
</script>

{#if visible}
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
      {#if activeTab === "LLM"}
        <div class="debug-tab-content">
          {#if llmChecking}
            <p class="debug-empty">Checking LLM connectivity...</p>
          {:else if llmStatus}
            <div class="debug-section">
              <h4>Status</h4>
              <pre class={llmStatus.reachable ? "llm-ok" : "llm-fail"}>{llmStatus.reachable ? "Reachable" : "Unreachable"} ({llmStatus.latency_ms}ms)</pre>
            </div>
            {#if llmStatus.error}
              <div class="debug-section">
                <h4>Error</h4>
                <pre class="llm-fail">{llmStatus.error}</pre>
              </div>
            {/if}
            <div class="debug-section">
              <h4>Configuration</h4>
              <pre>Provider: {llmStatus.provider}
Endpoint: {llmStatus.endpoint}
Model:    {llmStatus.model}</pre>
            </div>
            {#if llmStatus.available_models.length > 0}
              <div class="debug-section">
                <h4>Available Models ({llmStatus.available_models.length})</h4>
                <pre>{llmStatus.available_models.join("\n")}</pre>
              </div>
              {#if !llmStatus.available_models.some(m => m.startsWith(llmStatus!.model))}
                <p class="debug-notice">Configured model "{llmStatus.model}" not found in available models.</p>
              {/if}
            {/if}
            <button class="llm-recheck" onclick={runLlmCheck}>Re-check</button>
          {:else}
            <p class="debug-empty">No status yet.</p>
          {/if}
        </div>
      {:else if activeTab === "Context"}
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
                <JSONTree value={debugState.prediction.resolver_output.original_predictions} />
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
                <JSONTree value={char} />
              </div>
            {/each}
          {:else}
            <p class="debug-empty">Waiting for turn data...</p>
          {/if}
        </div>
      {:else if activeTab === "Events"}
        <div class="debug-tab-content">
          {#if debugState.events || debugState.decomposition}
            <!-- DistilBERT fast classification -->
            <div class="debug-section">
              <h4>Classification <span class="events-source">DistilBERT</span></h4>
              {#if debugState.events}
                {#if !debugState.events.classifier_loaded}
                  <p class="debug-notice">No event classifier loaded. Set STORYTELLER_MODEL_PATH or STORYTELLER_DATA_PATH.</p>
                {:else if debugState.events.classifications.length > 0}
                  <div class="classification-chips">
                    {#each debugState.events.classifications as cls}
                      <span class="classification-chip">{cls}</span>
                    {/each}
                  </div>
                {:else}
                  <p class="debug-empty">No classifications produced.</p>
                {/if}
              {:else}
                <p class="debug-empty">Waiting...</p>
              {/if}
            </div>

            <!-- LLM decomposition -->
            <div class="events-divider"></div>
            <div class="debug-section">
              <h4>Decomposition <span class="events-source">qwen2.5:3b-instruct</span>{#if debugState.decomposition} <span class="token-count">{debugState.decomposition.timing_ms}ms</span>{/if}</h4>
              {#if debugState.decomposition}
                {#if debugState.decomposition.error}
                  <pre class="llm-fail">{debugState.decomposition.error}</pre>
                {/if}
                {#if debugState.decomposition.raw_llm_json && !debugState.decomposition.decomposition}
                  <div class="debug-section">
                    <h4>Raw LLM Response</h4>
                    <JSONTree value={debugState.decomposition.raw_llm_json} />
                  </div>
                {/if}
                {#if debugState.decomposition.decomposition}
                  {@const decomp = debugState.decomposition.decomposition}
                  {#each decomp.events as event, i}
                    <div class="decomp-event">
                      <div class="decomp-kind">{event.kind} <span class="decomp-direction">{event.relational_direction}</span></div>
                      <div class="decomp-triple">
                        <span class="decomp-entity actor">{event.actor ? `${event.actor.mention} [${event.actor.category}]` : "(no actor)"}</span>
                        <span class="decomp-arrow">&rarr;</span>
                        <span class="decomp-action">{event.action}</span>
                        <span class="decomp-arrow">&rarr;</span>
                        <span class="decomp-entity target">{event.target ? `${event.target.mention} [${event.target.category}]` : "(no target)"}</span>
                      </div>
                      {#if event.confidence_note}
                        <div class="decomp-note">{event.confidence_note}</div>
                      {/if}
                    </div>
                  {/each}
                  {#if decomp.entities.length > 0}
                    <div class="decomp-entities-row">
                      {#each decomp.entities as entity}
                        <span class="entity-chip">{entity.mention} <span class="entity-cat">{entity.category}</span></span>
                      {/each}
                    </div>
                  {/if}
                {:else if !debugState.decomposition.error}
                  <p class="debug-empty">No decomposition produced.</p>
                {/if}
              {:else}
                <p class="debug-empty">Waiting for LLM...</p>
              {/if}
            </div>
          {:else}
            <p class="debug-empty">Waiting for turn data...</p>
          {/if}
        </div>
      {:else if activeTab === "Arbitration"}
        <div class="debug-tab-content">
          {#if debugState.arbitration}
            <div class="debug-section">
              <h4>Verdict</h4>
              <pre class={debugState.arbitration.result.verdict === "Permitted" ? "arb-permitted" : debugState.arbitration.result.verdict === "Impossible" ? "arb-impossible" : "arb-ambiguous"}>{debugState.arbitration.result.verdict}</pre>
            </div>
            {#if debugState.arbitration.result.verdict === "Impossible" && debugState.arbitration.result.reason}
              <div class="debug-section">
                <h4>Violation</h4>
                <pre>{debugState.arbitration.result.reason.constraint_name}: {debugState.arbitration.result.reason.description}</pre>
              </div>
            {/if}
            {#if debugState.arbitration.result.verdict === "Ambiguous" && debugState.arbitration.result.uncertainty}
              <div class="debug-section">
                <h4>Uncertainty</h4>
                <pre>{debugState.arbitration.result.uncertainty}</pre>
              </div>
              {#if debugState.arbitration.result.known_constraints && debugState.arbitration.result.known_constraints.length > 0}
                <div class="debug-section">
                  <h4>Known Constraints</h4>
                  <JSONTree value={debugState.arbitration.result.known_constraints} />
                </div>
              {/if}
            {/if}
            {#if debugState.arbitration.result.verdict === "Permitted" && debugState.arbitration.result.conditions && debugState.arbitration.result.conditions.length > 0}
              <div class="debug-section">
                <h4>Conditions</h4>
                <JSONTree value={debugState.arbitration.result.conditions} />
              </div>
            {/if}
            <div class="debug-section">
              <h4>Input</h4>
              <pre>{debugState.arbitration.player_input}</pre>
            </div>
            <div class="debug-section">
              <h4>Arbitration: {debugState.arbitration.timing_ms}ms</h4>
            </div>
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
    position: relative;
    flex-shrink: 0;
    height: 25%;
    background: var(--bg-debug);
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text-debug);
  }

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

  .llm-ok {
    color: var(--debug-green);
  }

  .llm-fail {
    color: #d88;
  }

  .llm-recheck {
    background: var(--bg-debug-tab);
    border: 1px solid var(--border-debug);
    color: var(--text-debug);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    padding: 0.3rem 0.75rem;
    border-radius: 3px;
    cursor: pointer;
    width: fit-content;
    box-shadow: none;
    margin-top: 0.25rem;
  }

  .llm-recheck:hover {
    border-color: var(--accent-dim);
    color: var(--text-primary);
  }

  .debug-error {
    background: #2a1515;
    color: #d88;
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    margin-top: 0.5rem;
    font-size: 0.75rem;
  }

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

  .events-source {
    font-weight: 400;
    color: var(--text-debug-dim);
    font-size: 0.65rem;
    font-style: italic;
  }

  .events-divider {
    border-top: 1px solid var(--border-debug);
    margin: 0.25rem 0;
  }

  .classification-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }

  .classification-chip {
    background: var(--bg-debug-tab);
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    font-size: 0.7rem;
    color: var(--text-debug);
  }

  .decomp-entities-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-top: 0.4rem;
  }

  .entity-chip {
    background: var(--bg-debug-tab);
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    font-size: 0.65rem;
    color: var(--text-debug);
  }

  .entity-cat {
    color: var(--text-debug-dim);
    font-size: 0.6rem;
  }

  .decomp-event {
    background: var(--bg-debug-tab);
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    margin-bottom: 0.4rem;
  }

  .decomp-kind {
    font-weight: 600;
    color: var(--accent);
    margin-bottom: 0.2rem;
    font-size: 0.75rem;
  }

  .decomp-direction {
    font-weight: 400;
    color: var(--text-debug-dim);
    font-size: 0.7rem;
  }

  .decomp-triple {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    font-size: 0.75rem;
    line-height: 1.5;
  }

  .decomp-entity {
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
    font-size: 0.7rem;
  }

  .decomp-entity.actor {
    background: #1a3a2a;
    color: var(--debug-green);
  }

  .decomp-entity.target {
    background: #2a2a3a;
    color: #aac;
  }

  .decomp-arrow {
    color: var(--text-debug-dim);
    font-size: 0.7rem;
  }

  .decomp-action {
    color: var(--text-primary);
    font-style: italic;
    font-size: 0.75rem;
  }

  .decomp-note {
    color: var(--text-debug-dim);
    font-size: 0.65rem;
    font-style: italic;
    margin-top: 0.2rem;
  }

  .arb-permitted {
    color: var(--debug-green);
    font-weight: 600;
  }

  .arb-impossible {
    color: #d55;
    font-weight: 600;
  }

  .arb-ambiguous {
    color: var(--debug-yellow);
    font-weight: 600;
  }
</style>

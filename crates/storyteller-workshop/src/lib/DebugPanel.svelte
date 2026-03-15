<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import JSONTree from "svelte-json-tree";
  import type { DebugState, PhaseStatus } from "./types";
  import { DEBUG_EVENT_CHANNEL, LOG_EVENT_CHANNEL } from "./types";
  import type { DebugEvent, HealthReport, LogEntry } from "./generated";
  import { checkHealth } from "./api";
  import { phaseStatus as computePhaseStatus, freshDebugState, applyDebugEvent } from "./logic";

  let { visible }: { visible: boolean } = $props();

  const TABS = ["LLM", "Context", "ML Predictions", "Characters", "Events", "Arbitration", "Goals", "Narrator", "Logs"] as const;
  type TabName = (typeof TABS)[number];
  const TAB_PHASE_MAP: Record<TabName, string> = {
    LLM: "llm",
    Context: "context",
    "ML Predictions": "prediction",
    Characters: "characters",
    Events: "events",
    Arbitration: "arbitration",
    Goals: "goals",
    Narrator: "narrator",
    Logs: "logs",
  };

  let activeTab: TabName = $state("LLM");
  let healthReport: HealthReport | null = $state(null);
  let healthChecking = $state(false);
  const MAX_LOG_ENTRIES = 500;
  let logEntries: LogEntry[] = $state([]);
  let logAutoScroll = $state(true);
  let logContainer: HTMLDivElement | undefined = $state(undefined);
  let expandedLogIndices: Set<number> = $state(new Set());
  let debugState: DebugState = $state({
    turn: 0,
    phases: {},
    prediction: null,
    context: null,
    decomposition: null,
    arbitration: null,
    intent_synthesis: null,
    goals: null,
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
    debugState = freshDebugState(turn);
  }

  function phaseStatus(tab: TabName): PhaseStatus {
    return computePhaseStatus(tab, debugState, healthReport, healthChecking);
  }

  async function runHealthCheck() {
    healthChecking = true;
    try {
      healthReport = await checkHealth();
    } catch (e) {
      healthReport = {
        status: "error",
        subsystems: [{
          name: "connection",
          status: "error",
          message: e instanceof Error ? e.message : String(e),
        }],
      };
    }
    healthChecking = false;
  }

  function handleDebugEvent(event: DebugEvent) {
    debugState = applyDebugEvent(debugState, event);
  }

  function handleLogEntry(entry: LogEntry) {
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
      unlistenLogs = await listen<LogEntry>(LOG_EVENT_CHANNEL, (e) => {
        handleLogEntry(e.payload);
      });
    })();

    runHealthCheck();

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
          {#if healthChecking}
            <p class="debug-empty">Checking server health...</p>
          {:else if healthReport}
            <div class="debug-section">
              <h4>Status</h4>
              <pre class={healthReport.status === "healthy" ? "llm-ok" : "llm-fail"}>{healthReport.status}</pre>
            </div>
            {#if healthReport.subsystems.length > 0}
              <div class="debug-section">
                <h4>Subsystems</h4>
                {#each healthReport.subsystems as sub}
                  <pre class={sub.status === "ok" ? "llm-ok" : "llm-fail"}>{sub.name}: {sub.status}{sub.message ? ` — ${sub.message}` : ""}</pre>
                {/each}
              </div>
            {/if}
            <button class="llm-recheck" onclick={runHealthCheck}>Re-check</button>
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
              <h4>Prediction Data</h4>
              <pre>{debugState.prediction.raw_json}</pre>
            </div>
            <div class="debug-section">
              <h4>Prediction: {debugState.prediction.timing_ms}ms</h4>
            </div>
          {:else}
            <p class="debug-empty">Waiting for turn data...</p>
          {/if}
        </div>
      {:else if activeTab === "Characters"}
        <div class="debug-tab-content">
          <p class="debug-empty">Character state will be available via GetSceneState RPC.</p>
        </div>
      {:else if activeTab === "Events"}
        <div class="debug-tab-content">
          {#if debugState.decomposition}
            <div class="debug-section">
              <h4>Decomposition <span class="events-source">{debugState.decomposition.model}</span> <span class="token-count">{debugState.decomposition.timing_ms}ms</span></h4>
              {#if debugState.decomposition.error}
                <pre class="llm-fail">{debugState.decomposition.error}</pre>
              {/if}
              {#if debugState.decomposition.raw_json}
                <div class="debug-section">
                  <h4>Raw Response</h4>
                  <pre>{debugState.decomposition.raw_json}</pre>
                </div>
              {:else if !debugState.decomposition.error}
                <p class="debug-empty">No decomposition produced.</p>
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
              <pre class={debugState.arbitration.verdict === "Permitted" ? "arb-permitted" : debugState.arbitration.verdict === "Impossible" ? "arb-impossible" : "arb-ambiguous"}>{debugState.arbitration.verdict}</pre>
            </div>
            {#if debugState.arbitration.details}
              <div class="debug-section">
                <h4>Details</h4>
                <pre>{debugState.arbitration.details}</pre>
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
      {:else if activeTab === "Goals"}
        <div class="debug-tab-content">
          {#if debugState.goals}
            {#if debugState.goals.scene_direction}
              <div class="debug-section">
                <h4>Scene Direction</h4>
                <pre>{debugState.goals.scene_direction}</pre>
              </div>
            {/if}
            {#if debugState.goals.character_drives.length > 0}
              <div class="debug-section">
                <h4>Character Drives</h4>
                {#each debugState.goals.character_drives as drive}
                  <pre class="goals-drive">{drive}</pre>
                {/each}
              </div>
            {/if}
            {#if debugState.goals.player_context}
              <div class="debug-section">
                <h4>Player Context</h4>
                <pre>{debugState.goals.player_context}</pre>
              </div>
            {/if}
            <div class="events-divider"></div>
            {#if debugState.goals.scene_goals.length > 0}
              <div class="debug-section">
                <h4>Scene Goals</h4>
                <div class="classification-chips">
                  {#each debugState.goals.scene_goals as goal}
                    <span class="classification-chip">{goal}</span>
                  {/each}
                </div>
              </div>
            {/if}
            {#if debugState.goals.character_goals.length > 0}
              <div class="debug-section">
                <h4>Character Goals</h4>
                <div class="classification-chips">
                  {#each debugState.goals.character_goals as goal}
                    <span class="classification-chip">{goal}</span>
                  {/each}
                </div>
              </div>
            {/if}
            <div class="debug-section">
              <h4>Intention Generation: {debugState.goals.timing_ms}ms</h4>
            </div>
          {:else}
            <p class="debug-empty">Waiting for scene setup...</p>
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

  .goals-drive {
    margin-bottom: 0.3rem;
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

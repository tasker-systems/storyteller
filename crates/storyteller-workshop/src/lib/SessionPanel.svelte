<script lang="ts">
  import { onMount } from "svelte";
  import { listSessions } from "$lib/api";
  import type { SessionInfo } from "$lib/generated";

  let { onNewSession, onResumeSession, visible }: {
    onNewSession: () => void;
    onResumeSession: (sessionId: string) => void;
    visible: boolean;
  } = $props();

  let sessions = $state<SessionInfo[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      sessions = await listSessions();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });
</script>

<aside class="session-panel" class:hidden={!visible}>
  <div class="panel-header">
    <span class="panel-title">Sessions</span>
  </div>

  <button class="new-session-btn" onclick={onNewSession}>+ New Session</button>

  <div class="session-list">
    {#if loading}
      <p class="status-msg">Loading sessions...</p>
    {:else if error}
      <p class="status-msg error">{error}</p>
    {:else if sessions.length === 0}
      <p class="status-msg">No sessions yet.</p>
    {:else}
      {#each sessions as session (session.session_id)}
        <div class="session-row">
          <div class="session-info">
            <span class="session-title">{session.title}</span>
            <span class="session-meta">{session.genre}</span>
            <span class="session-meta">{session.cast_names.join(" & ")}</span>
            <span class="session-meta">{session.turn_count} turn{session.turn_count === 1 ? "" : "s"}</span>
          </div>
          <button
            class="resume-btn"
            onclick={() => onResumeSession(session.session_id)}
          >
            Resume
          </button>
        </div>
      {/each}
    {/if}
  </div>
</aside>

<style>
  .session-panel {
    width: 260px;
    min-width: 260px;
    background: var(--bg-header);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    flex-shrink: 0;
  }

  .session-panel.hidden {
    display: none;
  }

  .panel-header {
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid var(--border);
  }

  .panel-title {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .new-session-btn {
    margin: 0.6rem 0.8rem;
    padding: 0.45rem 0.8rem;
    background: var(--accent);
    color: var(--bg);
    border: none;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    box-shadow: none;
  }

  .new-session-btn:hover {
    opacity: 0.9;
  }

  .session-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.4rem 0;
  }

  .status-msg {
    color: var(--text-secondary);
    font-size: 0.8rem;
    padding: 0.6rem 0.8rem;
    margin: 0;
  }

  .status-msg.error {
    color: #d88;
  }

  .session-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 0.5rem 0.8rem;
    border-bottom: 1px solid var(--border);
    gap: 0.4rem;
  }

  .session-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .session-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
    flex: 1;
  }

  .session-title {
    color: var(--text-primary);
    font-size: 0.82rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .session-meta {
    color: var(--text-secondary);
    font-size: 0.72rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .resume-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    padding: 0.2rem 0.5rem;
    border-radius: 3px;
    cursor: pointer;
    flex-shrink: 0;
    margin-top: 0.1rem;
    box-shadow: none;
  }

  .resume-btn:hover {
    color: var(--text-primary);
    border-color: var(--accent-dim);
  }
</style>

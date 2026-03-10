<script lang="ts">
  let { disabled, onsubmit }: { disabled: boolean; onsubmit: (text: string) => void } = $props();

  let text = $state("");

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  }

  function submit() {
    const trimmed = text.trim();
    if (!trimmed || disabled) return;
    onsubmit(trimmed);
    text = "";
  }
</script>

<div class="input-bar">
  <div class="input-container">
    <textarea
      class="input-textarea"
      placeholder={disabled ? "Waiting for the narrator..." : "What do you do?"}
      bind:value={text}
      onkeydown={handleKeydown}
      {disabled}
      rows="3"
    ></textarea>
    <button
      class="send-button"
      onclick={submit}
      disabled={disabled || !text.trim()}
      aria-label="Send"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 2L11 13" />
        <path d="M22 2L15 22L11 13L2 9L22 2Z" />
      </svg>
    </button>
  </div>
</div>

<style>
  .input-bar {
    border-top: 1px solid var(--border);
    padding: 0.75rem 1.5rem;
    background: var(--bg-input);
  }

  .input-container {
    max-width: 816px;
    margin: 0 auto;
    display: flex;
    gap: 0.5rem;
    align-items: flex-end;
  }

  .input-textarea {
    flex: 1;
    background: var(--bg-textarea);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.6em 0.8em;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.95rem;
    line-height: 1.5;
    resize: none;
    outline: none;
    box-shadow: none;
  }

  .input-textarea:focus {
    border-color: var(--accent-dim);
  }

  .input-textarea:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .input-textarea::placeholder {
    color: var(--text-secondary);
    opacity: 0.6;
  }

  .send-button {
    background: none;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.5em;
    color: var(--accent);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: none;
  }

  .send-button:hover:not(:disabled) {
    border-color: var(--accent);
    background: rgba(124, 156, 191, 0.1);
  }

  .send-button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
</style>

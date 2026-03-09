<script lang="ts">
  import type { StoryBlock } from "./types";

  let { block }: { block: StoryBlock } = $props();

  let expanded = $state(false);

  function truncate(text: string, max: number): string {
    if (text.length <= max) return text;
    return text.slice(0, max) + "...";
  }
</script>

{#if block.kind === "narrator" || block.kind === "opening"}
  <div class="story-block narrator-block">
    {#each block.text.split("\n\n") as paragraph, i}
      <p class="narrator-paragraph" class:first={i === 0}>{paragraph}</p>
    {/each}
  </div>
{:else if block.kind === "player"}
  <div class="story-block player-block">
    <button
      class="player-toggle"
      onclick={() => (expanded = !expanded)}
      aria-expanded={expanded}
    >
      <span class="player-label">You:</span>
      {#if expanded}
        <span class="player-text">{block.text}</span>
      {:else}
        <span class="player-text truncated">{truncate(block.text, 80)}</span>
      {/if}
    </button>
  </div>
{/if}

<style>
  .story-block {
    margin-bottom: 1.5rem;
  }

  .narrator-block {
    font-family: Georgia, "Times New Roman", serif;
    font-size: 1.05rem;
    line-height: 1.8;
    color: var(--text-primary);
  }

  .narrator-paragraph {
    margin: 0 0 0.8em 0;
  }

  .narrator-paragraph:not(.first) {
    text-indent: 1.5em;
  }

  .player-block {
    margin: 1rem 0;
  }

  .player-toggle {
    display: block;
    width: 100%;
    background: none;
    border: none;
    border-left: 2px solid var(--accent-dim);
    padding: 0.4em 0 0.4em 0.8em;
    text-align: left;
    cursor: pointer;
    font-family: inherit;
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--text-secondary);
    box-shadow: none;
    border-radius: 0;
  }

  .player-toggle:hover {
    border-left-color: var(--accent);
    color: var(--text-primary);
  }

  .player-label {
    font-style: italic;
    margin-right: 0.4em;
    color: var(--accent-dim);
  }

  .player-text.truncated {
    opacity: 0.7;
  }
</style>

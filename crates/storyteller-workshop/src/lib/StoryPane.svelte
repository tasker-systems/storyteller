<script lang="ts">
  import type { StoryBlock } from "./types";
  import StoryBlockView from "./StoryBlockView.svelte";

  let { blocks, loading }: { blocks: StoryBlock[]; loading: boolean } = $props();

  let container: HTMLElement | undefined = $state();
  let userScrolledUp = $state(false);

  function handleScroll() {
    if (!container) return;
    const threshold = 40;
    const atBottom =
      container.scrollHeight - container.scrollTop - container.clientHeight < threshold;
    userScrolledUp = !atBottom;
  }

  $effect(() => {
    // Re-run when blocks change length or content (for prose streaming)
    blocks.length;
    const lastBlock = blocks[blocks.length - 1];
    const _ = lastBlock?.text; // reactive dependency on content
    if (!userScrolledUp && container) {
      // Use tick-like delay to ensure DOM has updated
      requestAnimationFrame(() => {
        container?.scrollTo({ top: container.scrollHeight, behavior: "smooth" });
      });
    }
  });
</script>

<div class="story-pane" bind:this={container} onscroll={handleScroll}>
  <div class="story-content">
    {#each blocks as block, i (block.kind === "player" || block.kind === "narrator" ? `${block.kind}-${block.turn}` : `opening-${i}`)}
      <StoryBlockView {block} />
    {/each}
    {#if loading}
      <div class="loading-indicator">
        <span class="loading-text">The story unfolds</span>
        <span class="loading-dots">
          <span class="dot">.</span><span class="dot">.</span><span class="dot">.</span>
        </span>
      </div>
    {/if}
  </div>
</div>

<style>
  .story-pane {
    flex: 1;
    overflow-y: auto;
    padding: 2rem 2.5rem;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .story-content {
    max-width: 680px;
    margin: 0 auto;
  }

  .loading-indicator {
    font-family: Georgia, "Times New Roman", serif;
    font-style: italic;
    color: var(--text-secondary);
    padding: 1rem 0;
    font-size: 0.95rem;
  }

  .loading-dots .dot {
    animation: blink 1.4s infinite both;
  }

  .loading-dots .dot:nth-child(2) {
    animation-delay: 0.2s;
  }

  .loading-dots .dot:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes blink {
    0%,
    80%,
    100% {
      opacity: 0.2;
    }
    40% {
      opacity: 1;
    }
  }
</style>

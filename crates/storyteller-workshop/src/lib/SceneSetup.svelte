<script lang="ts">
  import { loadCatalog, getGenreOptions, composeScene } from "$lib/api";
  import type {
    GenreSummary,
    GenreOptionsResult,
    SceneInfo,
    CastSelection,
    DynamicSelection,
    SceneSelections,
    ProfileSummary,
  } from "$lib/generated";
  import {
    canAdvance as checkCanAdvance,
    nextStep,
    prevStep,
    usedNames as getUsedNames,
    nextUnusedName as findNextUnusedName,
    castPairs as computeCastPairs,
  } from "$lib/logic";

  let { onlaunch }: { onlaunch: (info: SceneInfo) => void } = $props();

  // ---------------------------------------------------------------------------
  // Wizard state
  // ---------------------------------------------------------------------------

  let step = $state(0);
  const stepLabels = ["Genre", "Profile", "Cast", "Dynamics", "Setting", "Launch"];

  // Step 0: Genre
  let genres = $state<GenreSummary[]>([]);
  let genresLoading = $state(true);
  let genresError = $state<string | null>(null);
  let selectedGenreId = $state<string | null>(null);

  // Step 1: Profile
  let genreOptions = $state<GenreOptionsResult | null>(null);
  let optionsLoading = $state(false);
  let optionsError = $state<string | null>(null);
  let selectedProfileId = $state<string | null>(null);

  // Step 2: Cast
  let cast = $state<CastSelection[]>([]);
  let castSize = $state(2);

  // Step 3: Dynamics
  let dynamics = $state<DynamicSelection[]>([]);

  // Step 4: Setting
  let settingOverride = $state("");

  // Step 5: Launch
  let launching = $state(false);
  let launchError = $state<string | null>(null);

  // ---------------------------------------------------------------------------
  // Derived values
  // ---------------------------------------------------------------------------

  let selectedGenre = $derived(genres.find((g) => g.id === selectedGenreId) ?? null);

  let selectedProfile = $derived(
    genreOptions?.profiles.find((p) => p.id === selectedProfileId) ?? null,
  );

  let archetypes = $derived(genreOptions?.archetypes ?? []);
  let availableDynamics = $derived(genreOptions?.dynamics ?? []);
  let namePool = $derived(genreOptions?.names ?? []);

  let selectedArchetypeIds = $derived(cast.map((c) => c.archetype_id).filter((a) => a !== ""));

  // Build pairs for dynamics step
  let castPairs = $derived(computeCastPairs(cast));

  let canAdvance = $derived(
    checkCanAdvance(step, { selectedGenreId, selectedProfileId, cast, launching }, selectedProfile),
  );

  // ---------------------------------------------------------------------------
  // Load genres on mount
  // ---------------------------------------------------------------------------

  $effect(() => {
    loadCatalog()
      .then((result) => {
        genres = result;
        genresLoading = false;
      })
      .catch((err) => {
        genresError = String(err);
        genresLoading = false;
      });
  });

  // ---------------------------------------------------------------------------
  // Reload genre options when genre or archetypes change
  // ---------------------------------------------------------------------------

  let lastGenreId = $state<string | null>(null);
  let lastArchetypes = $state<string>("");

  $effect(() => {
    if (!selectedGenreId) return;

    const archetypeKey = selectedArchetypeIds.join(",");

    // Only refetch when genre or archetypes actually changed
    if (selectedGenreId === lastGenreId && archetypeKey === lastArchetypes) return;

    lastGenreId = selectedGenreId;
    lastArchetypes = archetypeKey;
    optionsLoading = true;
    optionsError = null;

    getGenreOptions(selectedGenreId, selectedArchetypeIds)
      .then((result) => {
        genreOptions = result;
        optionsLoading = false;
      })
      .catch((err) => {
        optionsError = String(err);
        optionsLoading = false;
      });
  });

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  function usedNames(): Set<string> {
    return getUsedNames(cast);
  }

  function nextUnusedName(): string {
    return findNextUnusedName(cast, namePool);
  }

  function initCast() {
    if (!selectedProfile) return;
    const min = selectedProfile.cast_size_min;
    castSize = min;
    const used = new Set<string>();
    cast = Array.from({ length: min }, () => {
      let name = "";
      for (const n of namePool) {
        if (!used.has(n)) {
          name = n;
          used.add(n);
          break;
        }
      }
      return { archetype_id: "", name, role: "cast" } as CastSelection;
    });
    // Default first character as player perspective
    if (cast.length > 0) {
      cast[0].role = "protagonist";
    }
    // Reset dynamics when cast changes
    dynamics = [];
  }

  function addCastMember() {
    if (!selectedProfile) return;
    if (cast.length >= selectedProfile.cast_size_max) return;
    cast = [...cast, { archetype_id: "", name: nextUnusedName(), role: "cast" }];
    castSize = cast.length;
    // Reset dynamics when cast changes
    dynamics = [];
  }

  function removeCastMember(index: number) {
    if (!selectedProfile) return;
    if (cast.length <= selectedProfile.cast_size_min) return;
    const wasPerspective = cast[index].role === "protagonist";
    cast = cast.filter((_, i) => i !== index);
    castSize = cast.length;
    // If we removed the player perspective, assign to first
    if (wasPerspective && cast.length > 0) {
      cast[0].role = "protagonist";
      cast = [...cast]; // trigger reactivity
    }
    // Reset dynamics when cast changes
    dynamics = [];
  }

  function setPlayerPerspective(index: number) {
    cast = cast.map((c, i) => ({ ...c, role: i === index ? "protagonist" : "cast" }));
  }

  function initDynamics() {
    // Pre-populate one dynamic entry per pair if not already set
    if (dynamics.length === 0 && castPairs.length > 0) {
      dynamics = castPairs.map((pair) => ({
        cast_index_a: pair.a,
        cast_index_b: pair.b,
        dynamic_id: "",
      }));
    }
  }

  function selectGenre(id: string) {
    if (selectedGenreId !== id) {
      selectedGenreId = id;
      // Reset downstream selections
      selectedProfileId = null;
      genreOptions = null;
      cast = [];
      dynamics = [];
      settingOverride = "";
    }
  }

  function selectProfile(id: string) {
    selectedProfileId = id;
    // Reset cast when profile changes (cast size bounds may differ)
    cast = [];
    dynamics = [];
  }

  function goNext() {
    if (!canAdvance) return;
    if (step === 1) {
      initCast();
    }
    if (step === 2 && cast.length >= 2) {
      initDynamics();
    }
    step = nextStep(step, cast.length);
  }

  function goBack() {
    step = prevStep(step, cast.length);
  }

  async function launch() {
    launching = true;
    launchError = null;
    try {
      const selections: SceneSelections = {
        genre_id: selectedGenreId!,
        profile_id: selectedProfileId!,
        cast,
        dynamics: dynamics.filter((d) => d.dynamic_id !== ""),
        setting_override: settingOverride.trim() || null,
        seed: null,
      };
      const result = await composeScene(selections);
      onlaunch(result);
    } catch (err) {
      launchError = String(err);
      launching = false;
    }
  }
</script>

<div class="scene-setup">
  <header class="wizard-header">
    <h2 class="wizard-title">Scene Setup</h2>
    <nav class="step-nav">
      {#each stepLabels as label, i}
        <span
          class="step-label"
          class:active={i === step}
          class:completed={i < step}
          class:disabled={i > step}
        >
          <span class="step-number">{i + 1}</span>
          {label}
        </span>
        {#if i < stepLabels.length - 1}
          <span class="step-separator">/</span>
        {/if}
      {/each}
    </nav>
  </header>

  <div class="wizard-body">
    <!-- Step 0: Genre -->
    {#if step === 0}
      <div class="step-content">
        <h3 class="step-title">Choose a Genre</h3>
        <p class="step-description">Select the genre that defines the world and tone of your scene.</p>

        {#if genresLoading}
          <div class="loading">Loading genres...</div>
        {:else if genresError}
          <div class="error">{genresError}</div>
        {:else}
          <div class="option-list">
            {#each genres as genre}
              <button
                class="option-card"
                class:selected={selectedGenreId === genre.id}
                onclick={() => selectGenre(genre.id)}
              >
                <div class="option-header">
                  <span class="option-name">{genre.display_name}</span>
                  <span class="option-meta">
                    {genre.archetype_count} archetypes, {genre.profile_count} profiles, {genre.dynamic_count} dynamics
                  </span>
                </div>
                <p class="option-description">{genre.description}</p>
              </button>
            {/each}
          </div>
        {/if}
      </div>

    <!-- Step 1: Profile -->
    {:else if step === 1}
      <div class="step-content">
        <h3 class="step-title">Choose a Scene Profile</h3>
        <p class="step-description">
          Profiles define the shape of a scene: its type, tension range, and cast size.
        </p>

        {#if optionsLoading}
          <div class="loading">Loading options...</div>
        {:else if optionsError}
          <div class="error">{optionsError}</div>
        {:else if genreOptions}
          <div class="option-list">
            {#each genreOptions.profiles as profile}
              <button
                class="option-card"
                class:selected={selectedProfileId === profile.id}
                onclick={() => selectProfile(profile.id)}
              >
                <div class="option-header">
                  <span class="option-name">{profile.display_name}</span>
                  <span class="option-badge">{profile.scene_type}</span>
                </div>
                <p class="option-description">{profile.description}</p>
                <div class="option-stats">
                  <span>Tension: {profile.tension_min}&ndash;{profile.tension_max}</span>
                  <span>Cast: {profile.cast_size_min}&ndash;{profile.cast_size_max}</span>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </div>

    <!-- Step 2: Cast -->
    {:else if step === 2}
      <div class="step-content">
        <h3 class="step-title">Assemble the Cast</h3>
        <p class="step-description">
          Assign archetypes and names. Select one character as the player perspective.
          {#if selectedProfile}
            <span class="cast-range">
              ({selectedProfile.cast_size_min}&ndash;{selectedProfile.cast_size_max} characters)
            </span>
          {/if}
        </p>

        {#if optionsLoading}
          <div class="loading">Loading archetypes...</div>
        {:else}
          <div class="cast-list">
            {#each cast as member, i}
              <div class="cast-row">
                <span class="cast-index">{i + 1}</span>

                <select
                  class="cast-select"
                  bind:value={cast[i].archetype_id}
                >
                  <option value="">-- archetype --</option>
                  {#each archetypes as arch}
                    <option value={arch.id}>{arch.display_name}</option>
                  {/each}
                </select>

                <input
                  class="cast-input"
                  type="text"
                  placeholder="Name"
                  bind:value={cast[i].name}
                />

                <label class="perspective-label">
                  <input
                    type="radio"
                    name="player-perspective"
                    checked={member.role === "protagonist"}
                    onchange={() => setPlayerPerspective(i)}
                  />
                  <span class="perspective-text">Player</span>
                </label>

                {#if selectedProfile && cast.length > selectedProfile.cast_size_min}
                  <button
                    class="remove-btn"
                    onclick={() => removeCastMember(i)}
                    aria-label="Remove character"
                  >&times;</button>
                {/if}
              </div>
            {/each}
          </div>

          {#if selectedProfile && cast.length < selectedProfile.cast_size_max}
            <button class="add-btn" onclick={addCastMember}>+ Add character</button>
          {/if}

          {#if cast.length > 0 && cast.filter((c) => c.role === "protagonist").length === 0}
            <div class="validation-hint">Select one character as the player perspective.</div>
          {/if}
        {/if}
      </div>

    <!-- Step 3: Dynamics -->
    {:else if step === 3}
      <div class="step-content">
        <h3 class="step-title">Assign Dynamics</h3>
        <p class="step-description">
          Define the relational dynamics between characters. Leave blank to skip a pairing.
        </p>

        {#if castPairs.length === 0}
          <p class="empty-note">No character pairs to configure.</p>
        {:else}
          <div class="dynamics-list">
            {#each castPairs as pair, i}
              <div class="dynamic-row">
                <span class="dynamic-pair">
                  {pair.labelA} &harr; {pair.labelB}
                </span>
                <select class="dynamic-select" bind:value={dynamics[i].dynamic_id}>
                  <option value="">-- none --</option>
                  {#each availableDynamics as dyn}
                    <option value={dyn.id} title={dyn.description}>
                      {dyn.display_name} ({dyn.role_a} / {dyn.role_b})
                    </option>
                  {/each}
                </select>
              </div>
            {/each}
          </div>
        {/if}
      </div>

    <!-- Step 4: Setting -->
    {:else if step === 4}
      <div class="step-content">
        <h3 class="step-title">Setting</h3>
        <p class="step-description">
          Optionally override the composed setting description, or leave blank to use the default.
        </p>

        <textarea
          class="setting-textarea"
          placeholder="Leave blank for the default setting, or describe your own..."
          bind:value={settingOverride}
          rows="6"
        ></textarea>
      </div>

    <!-- Step 5: Launch -->
    {:else if step === 5}
      <div class="step-content">
        <h3 class="step-title">Launch Scene</h3>

        <div class="launch-summary">
          <div class="summary-row">
            <span class="summary-label">Genre</span>
            <span class="summary-value">{selectedGenre?.display_name ?? "—"}</span>
          </div>
          <div class="summary-row">
            <span class="summary-label">Profile</span>
            <span class="summary-value">{selectedProfile?.display_name ?? "—"}</span>
          </div>
          <div class="summary-row">
            <span class="summary-label">Cast</span>
            <span class="summary-value">
              {cast.map((c) => {
                const arch = archetypes.find((a) => a.id === c.archetype_id);
                const suffix = c.role === "protagonist" ? " (player)" : "";
                return `${c.name ?? "?"} [${arch?.display_name ?? c.archetype_id}]${suffix}`;
              }).join(", ")}
            </span>
          </div>
          {#if dynamics.filter((d) => d.dynamic_id !== "").length > 0}
            <div class="summary-row">
              <span class="summary-label">Dynamics</span>
              <span class="summary-value">
                {dynamics
                  .filter((d) => d.dynamic_id !== "")
                  .map((d) => {
                    const dyn = availableDynamics.find((x) => x.id === d.dynamic_id);
                    const nameA = cast[d.cast_index_a]?.name ?? "?";
                    const nameB = cast[d.cast_index_b]?.name ?? "?";
                    return `${nameA}/${nameB}: ${dyn?.display_name ?? d.dynamic_id}`;
                  })
                  .join("; ")}
              </span>
            </div>
          {/if}
          {#if settingOverride.trim()}
            <div class="summary-row">
              <span class="summary-label">Setting</span>
              <span class="summary-value setting-preview">{settingOverride.trim()}</span>
            </div>
          {/if}
        </div>

        {#if launchError}
          <div class="error">{launchError}</div>
        {/if}

        <button class="launch-btn" onclick={launch} disabled={launching}>
          {#if launching}
            <span class="loading-dots">
              <span class="dot">.</span><span class="dot">.</span><span class="dot">.</span>
            </span>
            Composing scene...
          {:else}
            Begin Scene
          {/if}
        </button>
      </div>
    {/if}
  </div>

  <!-- Navigation -->
  <footer class="wizard-footer">
    {#if step > 0}
      <button class="nav-btn nav-back" onclick={goBack} disabled={launching}>Back</button>
    {:else}
      <span></span>
    {/if}

    {#if step < 5}
      <button class="nav-btn nav-next" onclick={goNext} disabled={!canAdvance}>
        Next
      </button>
    {/if}
  </footer>
</div>

<style>
  .scene-setup {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
    color: var(--text-primary);
  }

  /* ------- Header / Step Nav ------- */

  .wizard-header {
    padding: 1.25rem 2rem 1rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-header);
  }

  .wizard-title {
    margin: 0 0 0.75rem;
    font-family: Georgia, "Times New Roman", serif;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .step-nav {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .step-label {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }

  .step-label.active {
    color: var(--accent);
    font-weight: 600;
  }

  .step-label.completed {
    color: var(--text-primary);
  }

  .step-label.disabled {
    opacity: 0.4;
  }

  .step-number {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.3rem;
    height: 1.3rem;
    border-radius: 50%;
    border: 1px solid var(--border);
    font-size: 0.65rem;
    flex-shrink: 0;
  }

  .step-label.active .step-number {
    border-color: var(--accent);
    background: rgba(124, 156, 191, 0.15);
  }

  .step-label.completed .step-number {
    border-color: var(--accent-dim);
    background: rgba(124, 156, 191, 0.08);
  }

  .step-separator {
    color: var(--border);
    font-size: 0.7rem;
  }

  /* ------- Body ------- */

  .wizard-body {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem 2rem;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .step-content {
    max-width: 640px;
  }

  .step-title {
    margin: 0 0 0.5rem;
    font-family: Georgia, "Times New Roman", serif;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .step-description {
    margin: 0 0 1.25rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .cast-range {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--accent-dim);
  }

  /* ------- Option Cards (Genre, Profile) ------- */

  .option-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .option-card {
    display: block;
    width: 100%;
    text-align: left;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    cursor: pointer;
    color: var(--text-primary);
    font-family: inherit;
    font-size: inherit;
    transition: border-color 0.15s;
  }

  .option-card:hover {
    border-color: var(--accent-dim);
  }

  .option-card.selected {
    border-color: var(--accent);
    background: rgba(124, 156, 191, 0.06);
  }

  .option-header {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    margin-bottom: 0.25rem;
  }

  .option-name {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .option-meta {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-secondary);
  }

  .option-badge {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--accent);
    background: rgba(124, 156, 191, 0.1);
    padding: 0.1em 0.5em;
    border-radius: 3px;
  }

  .option-description {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .option-stats {
    display: flex;
    gap: 1.25rem;
    margin-top: 0.4rem;
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-secondary);
  }

  /* ------- Cast ------- */

  .cast-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .cast-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .cast-index {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-secondary);
    width: 1.5rem;
    text-align: center;
    flex-shrink: 0;
  }

  .cast-select {
    flex: 1;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.4em 0.6em;
    color: var(--text-primary);
    font-size: 0.85rem;
  }

  .cast-select:focus {
    border-color: var(--accent-dim);
    outline: none;
  }

  .cast-input {
    width: 8rem;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.4em 0.6em;
    color: var(--text-primary);
    font-size: 0.85rem;
  }

  .cast-input:focus {
    border-color: var(--accent-dim);
    outline: none;
  }

  .perspective-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
    flex-shrink: 0;
  }

  .perspective-text {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .remove-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 1rem;
    padding: 0.15em 0.5em;
    line-height: 1;
    flex-shrink: 0;
  }

  .remove-btn:hover {
    border-color: #a55;
    color: #d88;
  }

  .add-btn {
    background: none;
    border: 1px dashed var(--border);
    border-radius: 4px;
    color: var(--accent);
    cursor: pointer;
    padding: 0.4em 0.8em;
    font-size: 0.8rem;
    font-family: var(--font-mono);
  }

  .add-btn:hover {
    border-color: var(--accent-dim);
    background: rgba(124, 156, 191, 0.05);
  }

  .validation-hint {
    margin-top: 0.75rem;
    font-size: 0.8rem;
    color: #c97;
  }

  /* ------- Dynamics ------- */

  .dynamics-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .dynamic-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .dynamic-pair {
    font-size: 0.85rem;
    color: var(--text-primary);
    min-width: 10rem;
    flex-shrink: 0;
  }

  .dynamic-select {
    flex: 1;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.4em 0.6em;
    color: var(--text-primary);
    font-size: 0.85rem;
  }

  .dynamic-select:focus {
    border-color: var(--accent-dim);
    outline: none;
  }

  .empty-note {
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-style: italic;
  }

  /* ------- Setting ------- */

  .setting-textarea {
    width: 100%;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.6em 0.8em;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.9rem;
    line-height: 1.5;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }

  .setting-textarea:focus {
    border-color: var(--accent-dim);
  }

  .setting-textarea::placeholder {
    color: var(--text-secondary);
    opacity: 0.6;
  }

  /* ------- Launch Summary ------- */

  .launch-summary {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1rem;
    margin-bottom: 1.25rem;
  }

  .summary-row {
    display: flex;
    gap: 1rem;
    padding: 0.35rem 0;
    font-size: 0.85rem;
    line-height: 1.45;
  }

  .summary-row + .summary-row {
    border-top: 1px solid var(--border);
  }

  .summary-label {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-secondary);
    min-width: 5rem;
    flex-shrink: 0;
    padding-top: 0.1rem;
  }

  .summary-value {
    color: var(--text-primary);
  }

  .setting-preview {
    white-space: pre-wrap;
    font-style: italic;
    color: var(--text-secondary);
  }

  .launch-btn {
    background: rgba(124, 156, 191, 0.12);
    border: 1px solid var(--accent);
    border-radius: 6px;
    color: var(--accent);
    cursor: pointer;
    padding: 0.6em 1.5em;
    font-size: 0.95rem;
    font-family: Georgia, "Times New Roman", serif;
    font-weight: 600;
    transition: background 0.15s;
  }

  .launch-btn:hover:not(:disabled) {
    background: rgba(124, 156, 191, 0.2);
  }

  .launch-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ------- Shared ------- */

  .loading {
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-style: italic;
  }

  .error {
    font-size: 0.85rem;
    color: #d88;
    background: rgba(200, 80, 80, 0.08);
    border: 1px solid rgba(200, 80, 80, 0.2);
    border-radius: 4px;
    padding: 0.5em 0.75em;
    margin-bottom: 0.75rem;
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

  /* ------- Footer Nav ------- */

  .wizard-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 2rem;
    border-top: 1px solid var(--border);
    background: var(--bg-header);
  }

  .nav-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.4em 1.2em;
    font-size: 0.85rem;
    cursor: pointer;
    color: var(--text-primary);
  }

  .nav-btn:hover:not(:disabled) {
    border-color: var(--accent-dim);
  }

  .nav-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .nav-next {
    color: var(--accent);
    border-color: var(--accent-dim);
  }

  .nav-next:hover:not(:disabled) {
    border-color: var(--accent);
    background: rgba(124, 156, 191, 0.08);
  }
</style>

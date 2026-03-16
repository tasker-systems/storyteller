# Sequential Descriptor RPCs and Turn 0 Persistence — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the combined `GetGenreOptions` Tauri command with five per-step commands for lazy-loading wizard data, and fix the turn 0 persistence bug so opening narration survives session resume.

**Architecture:** The gRPC proto, server, and client already implement all six per-step RPCs. This work is entirely in the Tauri command layer (Rust), TypeScript API, Svelte frontend wizard, and a small persistence fix in the engine service. No new types, no proto changes, no server changes.

**Tech Stack:** Rust (Tauri commands, ts-rs), TypeScript (Svelte 5, Tauri invoke API), gRPC (existing client methods)

**Spec:** `docs/superpowers/specs/2026-03-16-sequential-rpcs-and-turn-zero-persistence-design.md`

---

## Chunk 1: A.2 — Turn 0 Persistence Bugfix

A.2 is a small, self-contained fix. Do it first so we can verify session resume works correctly before touching the wizard.

### Task 1: Persist Turn 0 Event and Turn Entry

**Files:**
- Modify: `crates/storyteller-server/src/grpc/engine_service.rs` (compose_scene method, after narrator rendering around line 362)

- [ ] **Step 1: Add turn 0 event persistence after narrator rendering**

In the `compose_scene` method, between the `update_runtime_snapshot` call (around line 395) and the `TurnComplete` event send (around line 398), add the following. This placement matches the `submit_input` ordering where persistence happens just before `TurnComplete`:

```rust
// Persist turn 0 to event log and turn index
let mut turn0_event_ids = Vec::new();
if let Ok(eid) = session_store.events.append(
    &session_id,
    "narrator_complete",
    Some(0),
    &serde_json::json!({"prose": opening_prose}),
) {
    turn0_event_ids.push(eid);
}

let turn0_entry = crate::persistence::TurnEntry {
    turn: 0,
    timestamp: chrono::Utc::now().to_rfc3339(),
    player_input: None,
    event_ids: turn0_event_ids,
};
let _ = session_store.turns.append(&session_id, &turn0_entry);
```

This mirrors the pattern from `submit_input` at lines 830-886.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p storyteller-server`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/storyteller-server/src/grpc/engine_service.rs
git commit -m "fix: persist turn 0 opening narration to events.jsonl and turns.jsonl

ComposeScene streamed turn 0 events to the client but never wrote them
to disk. On session resume, opening narration was lost because the
resume path reconstructs history from these files.

Mirrors the persistence pattern from submit_input for turns 1+."
```

### Task 2: Verify Turn 0 Persistence (Manual)

This task is a manual verification step — run the server and workshop to confirm the fix works. Skip if doing automated-only development.

- [ ] **Step 1: Start server and workshop**

Run: `cargo run -p storyteller-server` (terminal 1)
Run: `cd crates/storyteller-workshop && bun tauri dev` (terminal 2)

- [ ] **Step 2: Compose a new scene and check persistence files**

Compose any scene through the wizard. After the opening narration appears, check the session directory:

```bash
# Find the most recent session
ls -t .story/sessions/ | head -1
# Check turns.jsonl starts with turn 0
cat .story/sessions/<session_id>/turns.jsonl | head -1
# Expected: {"turn":0,"timestamp":"...","player_input":null,"event_ids":["..."]}

# Check events.jsonl has a narrator_complete for turn 0
cat .story/sessions/<session_id>/events.jsonl | head -1
# Expected: {"event_id":"...","event_type":"narrator_complete","session_id":"...","turn":0,...,"payload":{"prose":"..."}}
```

- [ ] **Step 3: Resume the session and verify opening narration appears**

Close the workshop, reopen, resume the session. The opening narration block should appear as the first entry.

---

## Chunk 2: A.1 — Tauri Commands and TypeScript API

### Task 3: Add Five Per-Step Tauri Commands

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

- [ ] **Step 1: Add `get_profiles_for_genre` command**

Add after the existing `load_catalog` command (around line 74):

```rust
#[tauri::command]
pub async fn get_profiles_for_genre(
    genre_id: String,
    client: State<'_, ClientState>,
) -> Result<Vec<ProfileSummary>, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let list = c
        .get_profiles_for_genre(&genre_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(list
        .profiles
        .into_iter()
        .map(|p| ProfileSummary {
            id: p.entity_id,
            display_name: p.display_name,
            description: p.description,
            scene_type: p.scene_type,
            tension_min: p.tension_min,
            tension_max: p.tension_max,
            cast_size_min: p.cast_size_min,
            cast_size_max: p.cast_size_max,
        })
        .collect())
}
```

- [ ] **Step 2: Add `get_archetypes_for_genre` command**

```rust
#[tauri::command]
pub async fn get_archetypes_for_genre(
    genre_id: String,
    client: State<'_, ClientState>,
) -> Result<Vec<ArchetypeSummary>, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let list = c
        .get_archetypes_for_genre(&genre_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(list
        .archetypes
        .into_iter()
        .map(|a| ArchetypeSummary {
            id: a.entity_id,
            display_name: a.display_name,
            description: a.description,
        })
        .collect())
}
```

- [ ] **Step 3: Add `get_dynamics_for_genre` command**

```rust
#[tauri::command]
pub async fn get_dynamics_for_genre(
    genre_id: String,
    selected_archetype_ids: Vec<String>,
    client: State<'_, ClientState>,
) -> Result<Vec<DynamicSummary>, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let list = c
        .get_dynamics_for_genre(&genre_id, selected_archetype_ids)
        .await
        .map_err(|e| e.to_string())?;

    Ok(list
        .dynamics
        .into_iter()
        .map(|d| DynamicSummary {
            id: d.entity_id,
            display_name: d.display_name,
            description: d.description,
            role_a: d.role_a,
            role_b: d.role_b,
        })
        .collect())
}
```

- [ ] **Step 4: Add `get_names_for_genre` command**

```rust
#[tauri::command]
pub async fn get_names_for_genre(
    genre_id: String,
    client: State<'_, ClientState>,
) -> Result<Vec<String>, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let list = c
        .get_names_for_genre(&genre_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(list.names)
}
```

- [ ] **Step 5: Add `get_settings_for_genre` command**

```rust
#[tauri::command]
pub async fn get_settings_for_genre(
    genre_id: String,
    client: State<'_, ClientState>,
) -> Result<Vec<SettingSummary>, String> {
    let mut guard = client.lock().await;
    let c = guard.as_mut().ok_or(NOT_CONNECTED)?;
    let list = c
        .get_settings_for_genre(&genre_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(list
        .settings
        .into_iter()
        .map(|s| SettingSummary {
            id: s.profile_id,
            name: s.name,
        })
        .collect())
}
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check -p storyteller-workshop`

Expected: Compiles with no errors (the old `get_genre_options` still exists — we'll remove it next).

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat: add five per-step Tauri commands for sequential descriptor RPCs

Each command calls the corresponding existing gRPC client method:
get_profiles_for_genre, get_archetypes_for_genre, get_dynamics_for_genre,
get_names_for_genre, get_settings_for_genre."
```

### Task 4: Remove Combined GenreOptions from Tauri Layer

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs` (remove `get_genre_options`)
- Modify: `crates/storyteller-workshop/src-tauri/src/types.rs` (remove `GenreOptionsResult`)
- Modify: `crates/storyteller-workshop/src-tauri/src/lib.rs` (update command registration)

- [ ] **Step 1: Remove `get_genre_options` command from `commands.rs`**

Delete the entire `get_genre_options` function (lines 76-134 approximately — the `#[tauri::command]` through the closing `}`). Also remove the `GenreOptionsResult` import from the `use crate::types::` block at the top of the file if it's imported there.

- [ ] **Step 2: Remove `GenreOptionsResult` struct from `types.rs`**

Delete the struct and its doc comment (lines 99-108 approximately):

```rust
/// Combined genre options for a wizard step.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/generated/")]
pub struct GenreOptionsResult {
    pub archetypes: Vec<ArchetypeSummary>,
    pub profiles: Vec<ProfileSummary>,
    pub dynamics: Vec<DynamicSummary>,
    pub names: Vec<String>,
    pub settings: Vec<SettingSummary>,
}
```

- [ ] **Step 3: Update command registration in `lib.rs`**

In `crates/storyteller-workshop/src-tauri/src/lib.rs`, replace the `invoke_handler` block (lines 101-111):

```rust
.invoke_handler(tauri::generate_handler![
    commands::check_health,
    commands::load_catalog,
    commands::get_profiles_for_genre,
    commands::get_archetypes_for_genre,
    commands::get_dynamics_for_genre,
    commands::get_names_for_genre,
    commands::get_settings_for_genre,
    commands::compose_scene,
    commands::submit_input,
    commands::list_sessions,
    commands::resume_session,
    commands::get_scene_state,
    commands::get_prediction_history,
])
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p storyteller-workshop`
Expected: Compiles. No references to `GenreOptionsResult` or `get_genre_options` remain in the Tauri crate.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/commands.rs \
       crates/storyteller-workshop/src-tauri/src/types.rs \
       crates/storyteller-workshop/src-tauri/src/lib.rs
git commit -m "refactor: remove combined GenreOptionsResult from Tauri layer

The combined get_genre_options command is replaced by five per-step
commands. The GenreOptionsResult wrapper type is removed from types.rs.
The gRPC-level GetGenreOptions RPC remains for CLI use."
```

### Task 5: Update TypeScript API and Generated Types

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/api.ts`
- Modify: `crates/storyteller-workshop/src/lib/generated/index.ts`
- Delete: `crates/storyteller-workshop/src/lib/generated/GenreOptionsResult.ts`

- [ ] **Step 1: Replace `getGenreOptions` with five per-step functions in `api.ts`**

Replace the `getGenreOptions` import and function with:

```typescript
import type {
  HealthReport,
  SceneInfo,
  TurnResult,
  GenreSummary,
  ProfileSummary,
  ArchetypeSummary,
  DynamicSummary,
  SettingSummary,
  SessionInfo,
  ResumeResult,
  SceneSelections,
} from "./generated";
```

Remove the `getGenreOptions` function and add these five:

```typescript
export async function getProfilesForGenre(genreId: string): Promise<ProfileSummary[]> {
  return invoke<ProfileSummary[]>("get_profiles_for_genre", { genreId });
}

export async function getArchetypesForGenre(genreId: string): Promise<ArchetypeSummary[]> {
  return invoke<ArchetypeSummary[]>("get_archetypes_for_genre", { genreId });
}

export async function getDynamicsForGenre(
  genreId: string,
  selectedArchetypeIds: string[] = [],
): Promise<DynamicSummary[]> {
  return invoke<DynamicSummary[]>("get_dynamics_for_genre", { genreId, selectedArchetypeIds });
}

export async function getNamesForGenre(genreId: string): Promise<string[]> {
  return invoke<string[]>("get_names_for_genre", { genreId });
}

export async function getSettingsForGenre(genreId: string): Promise<SettingSummary[]> {
  return invoke<SettingSummary[]>("get_settings_for_genre", { genreId });
}
```

- [ ] **Step 2: Remove `GenreOptionsResult` from barrel export**

In `crates/storyteller-workshop/src/lib/generated/index.ts`, remove the line:

```typescript
export type { GenreOptionsResult } from './GenreOptionsResult';
```

- [ ] **Step 3: Delete the generated type file**

```bash
rm crates/storyteller-workshop/src/lib/generated/GenreOptionsResult.ts
```

- [ ] **Step 4: Verify TypeScript type-checking passes**

Run from the workshop directory:
```bash
cd crates/storyteller-workshop && bun run check
```

Expected: This will FAIL because `SceneSetup.svelte` still imports `GenreOptionsResult` and calls `getGenreOptions`. That's expected — the frontend refactor in Task 6 will fix it.

- [ ] **Step 5: Commit**

```bash
git add crates/storyteller-workshop/src/lib/api.ts \
       crates/storyteller-workshop/src/lib/generated/index.ts
git rm crates/storyteller-workshop/src/lib/generated/GenreOptionsResult.ts
git commit -m "feat: replace getGenreOptions with five per-step API functions

TypeScript API now exposes getProfilesForGenre, getArchetypesForGenre,
getDynamicsForGenre, getNamesForGenre, getSettingsForGenre. The
GenreOptionsResult type and its generated .ts file are removed."
```

---

## Chunk 3: A.1 — Frontend Wizard Refactor

### Task 6: Refactor SceneSetup.svelte to Per-Step Lazy Loading

**Files:**
- Modify: `crates/storyteller-workshop/src/lib/SceneSetup.svelte`

This is the largest task. The changes are mechanical — replace the combined state and effect with per-step state and loading in `goNext()`.

- [ ] **Step 1: Update imports**

Replace the import block at the top of the `<script>` tag:

```typescript
import {
  loadCatalog,
  getProfilesForGenre,
  getArchetypesForGenre,
  getDynamicsForGenre,
  getNamesForGenre,
  getSettingsForGenre,
  composeScene,
} from "$lib/api";
import type {
  GenreSummary,
  SceneInfo,
  CastSelection,
  DynamicSelection,
  SceneSelections,
  ProfileSummary,
  ArchetypeSummary,
  DynamicSummary,
  SettingSummary,
} from "$lib/generated";
```

- [ ] **Step 2: Replace state declarations**

Remove the combined state (lines 37-40 approximately):

```typescript
// REMOVE these:
let genreOptions = $state<GenreOptionsResult | null>(null);
let optionsLoading = $state(false);
let optionsError = $state<string | null>(null);
```

Replace with per-step state:

```typescript
// Step 1: Profiles
let profiles = $state<ProfileSummary[]>([]);
let profilesLoading = $state(false);
let profilesError = $state<string | null>(null);

// Step 2: Cast data
let archetypes = $state<ArchetypeSummary[]>([]);
let namePool = $state<string[]>([]);
let castDataLoading = $state(false);
let castDataError = $state<string | null>(null);

// Step 3: Dynamics
let availableDynamics = $state<DynamicSummary[]>([]);
let dynamicsLoading = $state(false);
let dynamicsError = $state<string | null>(null);

// Step 4: Settings
let settings = $state<SettingSummary[]>([]);
let settingsLoading = $state(false);
let settingsError = $state<string | null>(null);
```

- [ ] **Step 3: Remove the combined $effect and dedup tracking**

Delete lines 99-124 approximately — the `lastGenreId`, `lastArchetypes` state variables and the `$effect` block that calls `getGenreOptions`.

- [ ] **Step 4: Remove derived values that read from genreOptions**

Delete these derived declarations (lines 62-68 approximately):

```typescript
// REMOVE these:
let selectedProfile = $derived(
  genreOptions?.profiles.find((p) => p.id === selectedProfileId) ?? null,
);
let archetypes = $derived(genreOptions?.archetypes ?? []);
let availableDynamics = $derived(genreOptions?.dynamics ?? []);
let namePool = $derived(genreOptions?.names ?? []);
```

Replace with a derived for `selectedProfile` that reads from the new `profiles` state:

```typescript
let selectedProfile = $derived(
  profiles.find((p) => p.id === selectedProfileId) ?? null,
);
```

(`archetypes`, `availableDynamics`, and `namePool` are now `$state` variables, not derived.)

- [ ] **Step 5: Add loading functions**

Add these async functions after the helpers section:

```typescript
// ---------------------------------------------------------------------------
// Per-step data loading
// ---------------------------------------------------------------------------

async function loadProfiles() {
  if (!selectedGenreId) return;
  profilesLoading = true;
  profilesError = null;
  try {
    profiles = await getProfilesForGenre(selectedGenreId);
  } catch (err) {
    profilesError = String(err);
  } finally {
    profilesLoading = false;
  }
}

async function loadCastData() {
  if (!selectedGenreId) return;
  castDataLoading = true;
  castDataError = null;
  try {
    const [archetypeResult, nameResult] = await Promise.all([
      getArchetypesForGenre(selectedGenreId),
      getNamesForGenre(selectedGenreId),
    ]);
    archetypes = archetypeResult;
    namePool = nameResult;
  } catch (err) {
    castDataError = String(err);
  } finally {
    castDataLoading = false;
  }
}

async function loadDynamics() {
  if (!selectedGenreId) return;
  dynamicsLoading = true;
  dynamicsError = null;
  try {
    availableDynamics = await getDynamicsForGenre(selectedGenreId, selectedArchetypeIds);
  } catch (err) {
    dynamicsError = String(err);
  } finally {
    dynamicsLoading = false;
  }
}

async function loadSettings() {
  if (!selectedGenreId) return;
  settingsLoading = true;
  settingsError = null;
  try {
    settings = await getSettingsForGenre(selectedGenreId);
  } catch (err) {
    settingsError = String(err);
  } finally {
    settingsLoading = false;
  }
}
```

- [ ] **Step 6: Update `goNext()` to trigger per-step loading**

Replace the existing `goNext()` function:

```typescript
function goNext() {
  if (!canAdvance) return;

  // Trigger data loading for the target step
  if (step === 0) {
    loadProfiles();
  } else if (step === 1) {
    initCast();
    loadCastData();
  } else if (step === 2 && cast.length >= 2) {
    initDynamics();
    loadDynamics();
  } else if (step === 3) {
    loadSettings();
  }

  step = nextStep(step, cast.length);
}
```

- [ ] **Step 7: Update `selectGenre()` to clear all downstream state**

Replace the existing `selectGenre()`:

```typescript
function selectGenre(id: string) {
  if (selectedGenreId !== id) {
    selectedGenreId = id;
    // Reset all downstream state and error flags
    selectedProfileId = null;
    profiles = [];
    profilesError = null;
    archetypes = [];
    namePool = [];
    castDataError = null;
    availableDynamics = [];
    dynamicsError = null;
    settings = [];
    settingsError = null;
    cast = [];
    dynamics = [];
    settingOverride = "";
  }
}
```

- [ ] **Step 8: Update template loading/error states**

In the template section, update the step 1 (Profile) loading check. Replace:

```svelte
{#if optionsLoading}
  <div class="loading">Loading options...</div>
{:else if optionsError}
  <div class="error">{optionsError}</div>
{:else if genreOptions}
```

With:

```svelte
{#if profilesLoading}
  <div class="loading">Loading profiles...</div>
{:else if profilesError}
  <div class="error">{profilesError}</div>
{:else if profiles.length > 0}
```

And update the profiles loop from `{#each genreOptions.profiles as profile}` to `{#each profiles as profile}`.

In step 2 (Cast), replace:

```svelte
{#if optionsLoading}
  <div class="loading">Loading archetypes...</div>
{:else}
```

With:

```svelte
{#if castDataLoading}
  <div class="loading">Loading archetypes and names...</div>
{:else if castDataError}
  <div class="error">{castDataError}</div>
{:else}
```

In step 3 (Dynamics), add loading state before the dynamics list if `dynamicsLoading` is true (the current template has no loading state for this step — add one).

In step 4 (Settings), add loading state similarly if `settingsLoading` is true.

**Note:** The settings step currently only shows a textarea for `settingOverride` — it does not render the `settings` array. The `getSettingsForGenre` call is wired up for future use but the data is not displayed. This is a pre-existing gap (settings data was already fetched but unused in the combined approach). No UI changes needed for settings rendering in this task.

- [ ] **Step 9: Verify TypeScript type-checking passes**

Run:
```bash
cd crates/storyteller-workshop && bun run check
```
Expected: PASS — no type errors.

- [ ] **Step 10: Run existing frontend tests**

Run:
```bash
cd crates/storyteller-workshop && bun run test
```
Expected: All tests pass. `logic.test.ts` does not reference `GenreOptionsResult` or the API layer.

- [ ] **Step 11: Commit**

```bash
git add crates/storyteller-workshop/src/lib/SceneSetup.svelte
git commit -m "refactor: SceneSetup wizard uses per-step lazy loading

Replace the combined getGenreOptions effect with per-step data loading
triggered on wizard step transitions. Each step loads its data when
entered: profiles on step 1, archetypes+names on step 2, dynamics on
step 3, settings on step 4. Back-navigation preserves loaded data."
```

---

## Chunk 4: Final Verification

### Task 7: Full Build and Quality Check

- [ ] **Step 1: Run Rust quality checks**

```bash
cargo make check
```

Expected: All clippy, fmt, test, and doc checks pass.

- [ ] **Step 2: Run frontend checks**

```bash
cd crates/storyteller-workshop && bun run check && bun run test
```

Expected: TypeScript type-checking and vitest tests pass.

- [ ] **Step 3: Verify no stale references**

Search for any remaining references to the removed items:

```bash
# Should find NO results in the workshop crate
grep -r "GenreOptionsResult" crates/storyteller-workshop/
grep -r "getGenreOptions" crates/storyteller-workshop/
grep -r "get_genre_options" crates/storyteller-workshop/
```

Expected: No matches. (The gRPC proto, server, and client still have `GetGenreOptions` — that's intentional.)

- [ ] **Step 4: Commit any fixups if needed**

If previous steps revealed issues, fix and commit.

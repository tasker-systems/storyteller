# Sequential Descriptor RPCs and Turn 0 Persistence

**Date:** 2026-03-16
**Branch:** `jcoletaylor/scene-setting-data-and-intent-driving`
**Status:** Design
**Meta-plan:** `docs/superpowers/specs/2026-03-16-scenes-chapters-stories-meta-plan.md`
**Work units:** A.1 (Sequential Descriptor RPCs), A.2 (Turn 0 Persistence Bugfix)

## Problem

### A.1: Combined RPC Flattens Cascading Wizard Constraints

The Phase 3 workshop conversion (PR #25) introduced a combined `GetGenreOptions` gRPC RPC that returns all archetypes, profiles, dynamics, names, and settings for a genre in a single response. The workshop's `SceneSetup.svelte` wizard calls this once and populates all steps from the result.

This lost the cascading constraint design from the original scene template generation spec (`docs/plans/2026-03-10-scene-template-generation-design.md`), where each wizard step constrains the next. The combined approach also won't scale — Tier B exploratory research will produce genre-contextualized expressions across ~20 genre regions with per-genre descriptor variants, potentially orders of magnitude more data than today. Loading everything eagerly on genre selection becomes untenable.

### A.2: Opening Narration Lost on Session Resume

`ComposeScene` in `engine_service.rs` renders turn 0 opening narration and streams it to the client, but never persists it to `turns.jsonl` or `events.jsonl`. On session resume, the opening narration block is missing because the resume path reconstructs history from these files.

Evidence: `.story/sessions/019ceee1-4602-7690-a69f-436c65106d9e/turns.jsonl` starts at turn 1; `events.jsonl` contains no turn 0 events.

## A.1 Design: Sequential Descriptor RPCs

### Approach

Replace the single combined Tauri command with five per-step commands, each backed by an existing discrete gRPC RPC. The proto definitions, server implementations, and client wrappers for all six per-step RPCs already exist — this work is entirely in the Tauri command layer, TypeScript API, and frontend wizard.

The wizard transitions from eager loading (all data on genre selection) to lazy loading (each step loads its data when entered). This enforces cascading constraints by construction and scales to larger descriptor datasets.

### Existing gRPC RPCs (No Changes)

The following RPCs in `proto/storyteller/v1/composer.proto` and their server/client implementations are untouched:

| RPC | Request | Response | Already Implemented |
|-----|---------|----------|---------------------|
| `ListGenres` | `Empty` | `GenreList` | Server, client, Tauri (`load_catalog`) |
| `GetProfilesForGenre` | `GenreRequest { genre_id }` | `ProfileList` | Server, client |
| `GetArchetypesForGenre` | `GenreRequest { genre_id }` | `ArchetypeList` | Server, client |
| `GetDynamicsForGenre` | `DynamicsRequest { genre_id, selected_archetype_ids }` | `DynamicsList` | Server, client |
| `GetNamesForGenre` | `GenreRequest { genre_id }` | `NameList` | Server, client |
| `GetSettingsForGenre` | `GenreRequest { genre_id }` | `SettingList` | Server, client |
| `GetGenreOptions` | `GenreOptionsRequest` | `GenreOptions` | Server, client — **kept for CLI use** |

`GetGenreOptions` remains in the proto and server for CLI utility. Only the Tauri-side wrapper is removed.

### New Tauri Commands

Five new commands in `crates/storyteller-workshop/src-tauri/src/commands.rs`, each following the same pattern as the existing `load_catalog`:

```
get_profiles_for_genre(genre_id: String) → Vec<ProfileSummary>
get_archetypes_for_genre(genre_id: String) → Vec<ArchetypeSummary>
get_dynamics_for_genre(genre_id: String, selected_archetype_ids: Vec<String>) → Vec<DynamicSummary>
get_names_for_genre(genre_id: String) → Vec<String>
get_settings_for_genre(genre_id: String) → Vec<SettingSummary>
```

Each acquires the client mutex, calls the corresponding `storyteller-client` method, maps proto types to the existing ts-rs-derived Tauri types, and returns.

### Tauri-Side Removals

- `get_genre_options` command from `commands.rs`
- `GenreOptionsResult` struct from `types.rs` (its `#[derive(TS)]` generated `GenreOptionsResult.ts` and barrel export line also removed)
- Command registration for `get_genre_options` in `lib.rs`

The individual types (`ProfileSummary`, `ArchetypeSummary`, `DynamicSummary`, `SettingSummary`) already have `#[derive(TS)]` with `#[ts(export)]` and are independently re-exported from `src/lib/generated/index.ts`. No new types needed.

### TypeScript API Changes

**`src/lib/api.ts` — remove:**
```typescript
getGenreOptions(genreId, selectedArchetypes) → GenreOptionsResult
```

**`src/lib/api.ts` — add:**
```typescript
getProfilesForGenre(genreId: string) → Promise<ProfileSummary[]>
getArchetypesForGenre(genreId: string) → Promise<ArchetypeSummary[]>
getDynamicsForGenre(genreId: string, selectedArchetypeIds: string[]) → Promise<DynamicSummary[]>
getNamesForGenre(genreId: string) → Promise<string[]>
getSettingsForGenre(genreId: string) → Promise<SettingSummary[]>
```

**`src/lib/generated/index.ts` — remove:**
```typescript
export type { GenreOptionsResult } from './GenreOptionsResult';
```

### Frontend Wizard Refactor

**File:** `crates/storyteller-workshop/src/lib/SceneSetup.svelte`

#### State Model

Replace the combined `genreOptions` monolith with per-step state:

**Remove:**
- `genreOptions: GenreOptionsResult | null`
- `optionsLoading` / `optionsError`
- `lastGenreId` / `lastArchetypes` dedup tracking
- The `$effect` that watches genre + archetypes and calls `getGenreOptions`

**Replace with:**
```
// Step 1: Profiles (loaded on entering step 1)
profiles: ProfileSummary[]
profilesLoading: boolean
profilesError: string | null

// Step 2: Cast (loaded on entering step 2)
archetypes: ArchetypeSummary[]
namePool: string[]
castDataLoading: boolean
castDataError: string | null

// Step 3: Dynamics (loaded on entering step 3)
availableDynamics: DynamicSummary[]
dynamicsLoading: boolean
dynamicsError: string | null

// Step 4: Settings (loaded on entering step 4)
settings: SettingSummary[]
settingsLoading: boolean
settingsError: string | null
```

#### Loading Triggers

Data loads in `goNext()` when transitioning to each step:

| Transition | Calls | Notes |
|------------|-------|-------|
| Step 0 → 1 | `getProfilesForGenre(genreId)` | |
| Step 1 → 2 | `getArchetypesForGenre(genreId)` + `getNamesForGenre(genreId)` | Parallel, both genre-only |
| Step 2 → 3 | `getDynamicsForGenre(genreId, selectedArchetypeIds)` | Filtered by cast archetypes |
| Step 3 → 4 | `getSettingsForGenre(genreId)` | |
| Step 4 → 5 | None | Summary from loaded state |

Each step renders a loading state until its RPC(s) complete. Back-navigation preserves already-loaded data — no refetch.

#### Derived Values

Current derived values that read from `genreOptions` become direct state:
- `archetypes` — was `$derived(genreOptions?.archetypes ?? [])`, becomes `$state`
- `availableDynamics` — was `$derived(genreOptions?.dynamics ?? [])`, becomes `$state`
- `namePool` — was `$derived(genreOptions?.names ?? [])`, becomes `$state`

#### Downstream Invalidation

Unchanged behavior, different implementation:
- `selectGenre()` clears: profiles, archetypes, namePool, availableDynamics, settings, cast, dynamics, settingOverride
- `selectProfile()` clears: cast, dynamics (archetypes and names stay — they're genre-scoped, not profile-scoped)
- Changing cast archetype selections: dynamics cleared (will reload on step 3 entry with new archetype IDs)

### Logic Module

`src/lib/logic.ts` — no changes expected. The `canAdvance`, `nextStep`, `prevStep`, `usedNames`, `nextUnusedName`, `castPairs` functions operate on wizard state, not on the data-fetching layer.

`src/lib/logic.test.ts` — no changes expected unless tests reference `GenreOptionsResult` directly.

---

## A.2 Design: Turn 0 Persistence Bugfix

### Root Cause

In `crates/storyteller-server/src/grpc/engine_service.rs`, the `compose_scene` method's opening narration phase (starting around line 274):
1. Renders opening narration via `narrator.render_opening()`
2. Streams `NarratorComplete` and `TurnComplete` events to the client
3. Updates the in-memory `RuntimeSnapshot` with turn 0 journal entry
4. **Never writes to `events.jsonl` or `turns.jsonl`**

The `submit_input` method (turns 1+) does persist to both files. Turn 0 was missed.

### Fix

After the narrator renders opening prose and before the `TurnComplete` event is sent, add two persistence calls. The `session_store` is already cloned into the spawned task (used for composition persistence earlier in the same closure), so no threading changes are needed.

1. **Persist narrator event to `events.jsonl`:**
   - Call `session_store.events.append()` (same pattern as `submit_input` at lines 830-837)
   - `event_type`: `"narrator_complete"`, `turn`: `0`, `payload`: `{ "prose": opening_prose }`
   - The `append()` method generates a UUIDv7 event ID internally and returns it

2. **Persist turn entry to `turns.jsonl`:**
   - Call `session_store.turns.append()` with `TurnEntry { turn: 0, timestamp, player_input: None, event_ids: [narrator_event_id] }`

This mirrors the persistence pattern in `submit_input`. The resume path (`resume_session`, lines 901-1046) already handles turn 0 correctly — it iterates all entries in `turns.jsonl`, looks up narrator prose from `events.jsonl` by event ID, and streams `NarratorComplete` events. It currently finds nothing for turn 0 because nothing was written.

### Verification

- Compose a new scene, check that `turns.jsonl` starts with a turn 0 entry with `player_input: null`
- Check that `events.jsonl` contains a `narrator_complete` event for turn 0
- Resume the session and verify the opening narration block appears in the UI

---

## File Impact Summary

### Modified Files

| File | Change |
|------|--------|
| `crates/storyteller-workshop/src-tauri/src/commands.rs` | Add 5 per-step commands, remove `get_genre_options` |
| `crates/storyteller-workshop/src-tauri/src/types.rs` | Remove `GenreOptionsResult` |
| `crates/storyteller-workshop/src-tauri/src/lib.rs` | Update command registrations |
| `crates/storyteller-workshop/src/lib/api.ts` | Replace `getGenreOptions` with 5 per-step functions |
| `crates/storyteller-workshop/src/lib/SceneSetup.svelte` | Refactor to per-step lazy loading |
| `crates/storyteller-workshop/src/lib/generated/index.ts` | Remove `GenreOptionsResult` export |
| `crates/storyteller-server/src/grpc/engine_service.rs` | Persist turn 0 to events.jsonl and turns.jsonl |

### Deleted Files

| File | Reason |
|------|--------|
| `crates/storyteller-workshop/src/lib/generated/GenreOptionsResult.ts` | ts-rs generated type no longer needed |

### Unchanged

- `proto/storyteller/v1/composer.proto` — all RPCs stay, including `GetGenreOptions`
- `crates/storyteller-server/src/grpc/composer_service.rs` — server implementations unchanged
- `crates/storyteller-client/src/client.rs` — client wrappers unchanged
- `crates/storyteller-composer/` — composer library unchanged
- `src/lib/logic.ts` and `src/lib/logic.test.ts` — wizard logic unchanged

## Relationship to Prior Work

| Prior design | Relationship |
|---|---|
| `docs/plans/2026-03-10-scene-template-generation-design.md` | A.1 restores the cascading wizard constraint design lost during Phase 3 gRPC conversion |
| `docs/superpowers/specs/2026-03-14-workshop-conversion-phase3-design.md` | Phase 3 introduced the combined RPC that A.1 replaces on the Tauri side |
| `docs/superpowers/specs/2026-03-16-scenes-chapters-stories-meta-plan.md` | A.1 and A.2 are the first two work units of Tier A |

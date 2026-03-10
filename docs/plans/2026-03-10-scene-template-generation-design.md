# Scene Template Generation Design

**Date:** 2026-03-10
**Branch:** `jcoletaylor/workshop-scene-template-generation`
**Status:** Approved

## Problem

The workshop prototype is limited to a single hardcoded scene (`the_flute_kept.rs`) with two characters (Bramblehoof and Pyotir). To test and refine the event classification, action arbitration, and narrative systems, we need a combinatorial matrix of scenes across genres, profiles, archetypes, and relational dynamics. The existing training data descriptors in `storyteller-data` already encode this matrix — we need to make it playable.

## Approach: Descriptor-Driven Composition

The descriptors are the scene templates. The existing JSON files in `storyteller-data/training-data/descriptors/` (archetypes, dynamics, profiles, genres, axes, cross-dimensions) encode the combinatorial matrix validated by 15,000 training examples. We build a Rust `SceneComposer` that loads these descriptors, exposes the valid option space to the frontend, and composes `SceneData` + `CharacterSheet` structs from user selections.

No new core types needed — the output is the existing type system. No LLM dependency for scene setup — composition is deterministic (with optional seed for reproducibility).

### Design Principles

- **Combinatorial ground truth**: Base layer works without any LLM. Deterministic composition from descriptors provides reproducible scenes for testing.
- **Descriptors as source of truth**: No new template abstraction — the selections themselves (genre + profile + archetypes + dynamics + names) are the template.
- **Session persistence for learning**: Flat-file session storage provides a design laboratory for what the eventual DB schema should look like.
- **Genre-gated validity**: Every downstream choice is constrained by genre validity matrices. Invalid combinations are structurally impossible.

## Section 1: Descriptor Extensions

The existing descriptors in `storyteller-data/training-data/descriptors/` are nearly complete. Three additions:

### 1a. Name Lists

New file `names.json` with genre-keyed name pools (~20-30 names per genre), harvested via a Python tool from fantasynamegenerators.com and manually curated.

```json
{
  "low_fantasy_folklore": {
    "names": ["Pyotir", "Ilyana", "Vasil", "Maren", "..."],
    "source": "fantasynamegenerators.com/slavic-names"
  },
  "sci_fi_noir": {
    "names": ["Kael", "Nyx", "Reeve", "Senna", "..."],
    "source": "fantasynamegenerators.com/cyberpunk-names"
  }
}
```

### 1b. Setting Templates

New file `settings.json`. Keyed by `genre x profile`, each entry provides prose fragments that the composer assembles into a `SceneSetting`. Genres have a `default_setting` that profiles override selectively — not every cell needs authoring.

```json
{
  "low_fantasy_folklore": {
    "default_setting": {
      "sensory_palette": ["woodsmoke", "turned earth", "distant livestock"],
      "time_options": ["late afternoon", "early morning mist", "dusk"]
    },
    "quiet_reunion": {
      "description_templates": [
        "A smallholding on the outskirts of {town}. {time}. {sensory}.",
        "The common room of a roadside inn. {time}. {sensory}."
      ],
      "affordances": [
        "One character arrives; the other is already here",
        "Physical space shapes conversation rhythm"
      ]
    }
  }
}
```

### 1c. Stub Genres

2-3 new genre entries in `genres.json` (e.g., `sci_fi_noir`, `cozy_ghost_story`) with validity matrices specifying which archetypes, dynamics, and profiles are valid. Minimal — enough to prove genre selection works and constraints filter correctly. `low_fantasy_folklore` serves as the reference for what a fully-fleshed genre looks like.

**Unchanged:** `archetypes.json`, `axis-vocabulary.json`, `dynamics.json`, `profiles.json`, `cross-dimensions.json` — genre-agnostic building blocks that genres reference.

### Genre Expansion Guidance (Future)

Adding a new genre requires:
1. A new entry in `genres.json` with validity matrices (which archetypes, dynamics, profiles are valid, which combinations are excluded)
2. A name pool in `names.json` (harvest via the name tool, curate manually)
3. Setting templates in `settings.json` (default_setting + profile-specific overrides)
4. Optionally, genre-specific `GenreConstraint` entries for the arbitration engine (`CapabilityLexicon` with forbidden/conditional capabilities)

The `low_fantasy_folklore` genre with its 15K training examples and fully-authored descriptors is the gold standard. New genres can start minimal (validity matrix + names + default settings) and be fleshed out incrementally.

## Section 2: Scene Composer (Rust)

New module in `storyteller-engine` that loads descriptors and composes scenes from selections.

### Module Structure

```
crates/storyteller-engine/src/scene_composer/
├── mod.rs              # Public API: SceneComposer struct
├── descriptors.rs      # Load & deserialize all descriptor JSONs
├── catalog.rs          # Filtered option queries (valid archetypes for genre, etc.)
├── compose.rs          # Selection -> SceneData + Vec<CharacterSheet>
└── names.rs            # Name pool selection with dedup
```

### SceneComposer API

Two categories of methods:

**Catalog queries** — drive the frontend's cascading selection UI:
- `genres()` — list available genres with metadata
- `profiles_for_genre(genre)` — valid scene profiles
- `archetypes_for_genre(genre)` — valid character archetypes
- `dynamics_for_genre(genre, archetypes)` — valid relational dynamics given selected archetypes

**Composition** — takes selections, produces playable scene:
- `compose(selections, seed?)` — returns `(SceneData, Vec<CharacterSheet>)`

### Composition Logic

Given a `SceneSelections` struct:
1. Sample tensor axis values within the archetype's defined ranges (using seed for determinism)
2. Assign temporal layers per the archetype's layer guidance
3. Build emotional state from the genre's grammar (Plutchik Western — all genres use it currently)
4. Compose setting prose from setting templates with variable substitution (time, sensory details, town name)
5. Generate scene constraints from the profile + genre
6. Build capability profiles from archetype defaults
7. Construct relational data (knows/does_not_know, self_edge) from the dynamic definitions
8. Assign names from genre name pool with deduplication

### SceneSelections

A plain struct capturing user choices. Lives in the composer module, not storyteller-core.

```rust
pub struct SceneSelections {
    pub genre: String,
    pub profile: String,
    pub cast: Vec<CastSelection>,
    pub dynamics: Vec<DynamicSelection>,
    pub setting_override: Option<String>,
    pub seed: Option<u64>,
}

pub struct CastSelection {
    pub archetype: String,
    pub name: String,
    pub is_player_perspective: bool,
}

pub struct DynamicSelection {
    pub character_a_index: usize,
    pub character_b_index: usize,
    pub dynamic: String,
}
```

### Descriptor Loading

Reads from `STORYTELLER_DATA_PATH/training-data/descriptors/`. Fails fast at workshop startup if descriptors are missing — this is development tooling, not graceful degradation territory.

### Determinism

Composition takes an optional `u64` seed. Same selections + same seed = same scene. No seed = random. Seed is persisted in `scene-selections.json` for reproducibility.

## Section 3: Session Persistence

### Directory Structure

```
crates/storyteller-workshop/.story/
├── .gitignore              # *
└── sessions/
    └── {session_uuidv7}/
        ├── scene-selections.json   # User's choices (genre, profile, archetypes, etc.)
        ├── scene.json              # Composed SceneData
        ├── characters.json         # Vec<CharacterSheet> as composed
        └── events.jsonl            # Append-only turn log
```

### Session Lifecycle

**New session:** User completes scene setup, composer produces scene + characters. All three JSON files written atomically. Session ID is a UUIDv7 (time-ordered, so `ls` gives chronological order).

**During play:** Each turn appends to `events.jsonl` with turn number, player input, narrator response, classification results, decomposition results, arbitration results, timing. Raw material for future training data extraction.

**Resume session:** Workshop lists existing sessions from the directory (reading `scene-selections.json` for display info like genre + title). User picks one, backend loads scene + characters + replays events.jsonl to rebuild journal state.

**Fork session:** "Start new session from this one's setup" copies `scene-selections.json` and `characters.json` into a new session directory, recomposes (optionally with a different seed), fresh events.jsonl.

### Design Intent

No DB dependency. Deliberately flat files. The structures discovered here inform the eventual DB schema, not the other way around. If sessions produce valuable training data, extraction into `storyteller-data` is a manual curation step.

## Section 4: Workshop Tauri Commands & UI Flow

### New Tauri Commands

```
load_catalog          # Load descriptors, return genre list
get_genre_options     # Given genre -> valid profiles, archetypes, dynamics
compose_scene         # SceneSelections -> compose, persist, return SceneInfo
list_sessions         # Scan .story/sessions/, return summaries
resume_session        # Load session by ID, rebuild state, return SceneInfo
```

`start_scene` (the current hardcoded command) stays as a thin wrapper that composes "The Flute Kept" selections implicitly. Eventually deprecated.

### Frontend Flow

**Left panel (collapsible):** Session navigator — new session, recent sessions list, resume/fork actions. Toggle pattern similar to debug inspector (bottom panel, cmd+D).

**Scene setup wizard (linear, new session):**

1. **Genre selection** — cards or list, genre description + completeness indicator (fully authored vs. stub)
2. **Profile selection** — filtered by genre, shows tension range + cast size + description
3. **Cast builder** — one slot per cast member. Each: archetype dropdown (filtered), name field (pre-filled from genre name pool, editable), "player perspective" radio button. Dynamic assignment between pairs appears once 2+ characters exist.
4. **Setting preview** — composed from templates, editable text area for authorial override
5. **Launch** — calls `compose_scene`, transitions to play view

Back-navigation preserves selections. Each step constrains the next.

**Session resume:** Skips the wizard — loads persisted state and drops into the play view with journal rebuilt from events.jsonl.

**Play view:** Unchanged — `StoryPane`, `InputBar`, `DebugPanel` continue to work exactly as they do today.

## Section 5: Name Harvesting Tool

### Structure

```
tools/name-harvester/
├── pyproject.toml              # uv-managed (httpx, beautifulsoup4)
├── src/name_harvester/
│   ├── __init__.py
│   ├── harvester.py            # Polite form POST + HTML parse
│   └── cli.py                  # CLI: genre mapping -> harvest -> JSON output
└── tests/
    └── test_harvester.py       # Parse logic tests against saved HTML fixtures
```

### Behavior

- Maps genre IDs to fantasynamegenerators.com category URLs (editorial mapping authored in script config)
- Submits the generator form, parses result HTML for name list
- Rate-limited: 2-3 second delay between requests, single-threaded
- Outputs to `storyteller-data/training-data/descriptors/names.json`
- Idempotent: re-running merges into existing file, doesn't duplicate

### Usage Pattern

Run once per genre, curate the output manually (remove anything that doesn't fit the genre aesthetic), commit the curated JSON. The tool exists for reproducibility and for when new genres are added. Not a runtime dependency.

## Section 6: Future Direction (Not Built)

### LLM Enrichment Pass

Optional step after composition that sends composed scene + characters to the structured LLM for aesthetic refinement: more specific setting prose, character-voice calibration, backstory details that fit the particular combination. Produces `enrichment.json` alongside composed data in the session directory. The enrichment output becomes training signal — comparing "combinatorial ground truth" against "LLM-refined" versions reveals where the LLM adds genuine value vs. where templates suffice.

### Tensor Slider UI

Per-character adjustment of axis values after archetype assignment. The archetype provides ranges; sliders let the user push within (or beyond) those ranges. Changes persist to `characters.json`.

### World-Agent / Game System Sets

Selectable rule packages that configure genre constraints, capability lexicons, and spatial zone behavior. Needs design work on what "game systems" means as a composable concept.

### Quickstart

Random valid selections -> immediate compose -> play. One click to a new scene. Natural affordance once the composer works — just "random valid selections + compose."

### Session-to-Training Pipeline

Tooling to extract session `events.jsonl` data into training examples for the character predictor or event classifier. Manual curation step required.

### Template Abstraction

If patterns emerge from sessions where specific selection combinations are repeatedly valuable, promote those into named, shareable templates. Let the data tell us when this abstraction earns its keep.

## File Impact Summary

### New Files (storyteller-data)
- `training-data/descriptors/names.json` — genre-keyed name pools
- `training-data/descriptors/settings.json` — genre x profile setting templates
- Updates to `training-data/descriptors/genres.json` — 2-3 stub genres

### New Files (storyteller-engine)
- `src/scene_composer/mod.rs` — SceneComposer public API
- `src/scene_composer/descriptors.rs` — descriptor loading/deserialization
- `src/scene_composer/catalog.rs` — filtered option queries
- `src/scene_composer/compose.rs` — selection -> scene composition
- `src/scene_composer/names.rs` — name pool management

### New Files (storyteller-workshop)
- `.story/.gitignore` — ignore all session data
- `src-tauri/src/commands.rs` — 5 new Tauri commands (extend existing file)
- `src/lib/SessionPanel.svelte` — left panel session navigator
- `src/lib/SceneSetup.svelte` — scene setup wizard (or split into step components)
- `src/lib/types.ts` — TypeScript types for catalog + selections (extend existing file)
- `src/lib/api.ts` — API wrappers for new commands (extend existing file)

### Modified Files
- `src/routes/+page.svelte` — integrate session panel + setup flow before play view
- `src-tauri/src/lib.rs` — register new commands
- `src-tauri/src/engine_state.rs` — session ID tracking, persistence paths

### New Files (tools)
- `tools/name-harvester/` — Python name harvesting tool (pyproject.toml, src/, tests/)

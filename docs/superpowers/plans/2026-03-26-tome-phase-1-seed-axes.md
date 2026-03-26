# Tome Phase 1: Seed Axes and Domain Clusters — Implementation Plan

> **For agentic workers:** This plan is collaborative elicitation work, not solo engineering. Tasks produce structured JSON artifacts through analytical dialogue. Execute sequentially — each pass builds on the previous. Use superpowers:executing-plans for inline execution with review checkpoints between passes.

**Goal:** Produce a complete axis inventory (~50-72 axes across 6 domains) with structured surfacing arguments, ready for Phase 2 mutual production graph mapping.

**Architecture:** Breadth-first passes — skeleton across all domains, then elaboration, then coherence review. Axes defined collaboratively (Claude + user), written to per-domain JSON files in `storyteller-data`.

**Spec:** `docs/superpowers/specs/2026-03-26-tome-phase-1-seed-axes-design.md`

---

### Task 1: Create Directory Structure

**Files:**
- Create: `storyteller-data/narrative-data/tome/domains/` (directory)

- [ ] **Step 1: Create the tome domains directory**

```bash
mkdir -p /Users/petetaylor/projects/tasker-systems/storyteller-data/narrative-data/tome/domains
```

- [ ] **Step 2: Commit scaffold**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/
git commit -m "chore: scaffold tome domain directory structure"
```

---

### Task 2: Pass 1 — Material Conditions Skeleton

**Files:**
- Create: `storyteller-data/narrative-data/tome/domains/material-conditions.json`

**Input:** `docs/foundation/tome_and_lore.md` (material conditions domain), `docs/foundation/world_design.md` (coherence model, material ingress), bedrock genre dimensions (`magic`, `technology`, `supernatural`, `violence`, `death`).

**Work:**
- [ ] **Step 1: Seed 4-6 axes for material conditions**

Elicit axes covering: geography/climate, natural resources, infrastructure, disease ecology, and world affordances (supernatural permeability, technological ceiling, physical law flexibility). Each axis needs: slug, name, description, axis_type, provisional values. Surfacing can be rough (agents + pathways only).

World affordance axes should note their bedrock genre dimension counterpart in `_commentary`.

- [ ] **Step 2: Write domain file**

Write the complete `material-conditions.json` with domain metadata, axes array, empty `_deferred`, and `_meta` with `pass: 1`.

- [ ] **Step 3: Validate skeleton gate**

Check: at least 4 axes, no obvious gaps from foundation docs, world affordance cluster represented.

---

### Task 3: Pass 1 — Economic Forms Skeleton

**Files:**
- Create: `storyteller-data/narrative-data/tome/domains/economic-forms.json`

**Input:** `docs/foundation/tome_and_lore.md` (economic forms domain).

**Work:**
- [ ] **Step 1: Seed 4-6 axes for economic forms**

Elicit axes covering: production mode, trade networks, labor organization, debt/obligation structures, currency systems, land tenure. Each axis: slug, name, description, axis_type, provisional values, rough surfacing.

- [ ] **Step 2: Write domain file**

Write `economic-forms.json` with domain metadata, axes, empty `_deferred`, `_meta` with `pass: 1`.

- [ ] **Step 3: Validate skeleton gate**

Check: at least 4 axes, no obvious gaps.

---

### Task 4: Pass 1 — Political Structures Skeleton

**Files:**
- Create: `storyteller-data/narrative-data/tome/domains/political-structures.json`

**Input:** `docs/foundation/tome_and_lore.md` (political structures domain).

**Work:**
- [ ] **Step 1: Seed 4-6 axes for political structures**

Elicit axes covering: power concentration, authority legitimation, legal system, enforcement capacity, institutional density. Each axis: slug, name, description, axis_type, provisional values, rough surfacing.

- [ ] **Step 2: Write domain file**

Write `political-structures.json` with domain metadata, axes, empty `_deferred`, `_meta` with `pass: 1`.

- [ ] **Step 3: Validate skeleton gate**

Check: at least 4 axes, no obvious gaps.

---

### Task 5: Pass 1 — Social Forms Skeleton

**Files:**
- Create: `storyteller-data/narrative-data/tome/domains/social-forms.json`

**Input:** `docs/foundation/tome_and_lore.md` (social forms domain).

**Work:**
- [ ] **Step 1: Seed 4-6 axes for social forms**

Elicit axes covering: kinship system, gender roles, class mobility, marriage/inheritance patterns, religious institutional power, education access. Each axis: slug, name, description, axis_type, provisional values, rough surfacing.

- [ ] **Step 2: Write domain file**

Write `social-forms.json` with domain metadata, axes, empty `_deferred`, `_meta` with `pass: 1`.

- [ ] **Step 3: Validate skeleton gate**

Check: at least 4 axes, no obvious gaps.

---

### Task 6: Pass 1 — History as Force Skeleton

**Files:**
- Create: `storyteller-data/narrative-data/tome/domains/history-as-force.json`

**Input:** `docs/foundation/tome_and_lore.md` (history as active force domain), `docs/foundation/world_design.md`.

**Work:**
- [ ] **Step 1: Seed 4-6 axes for history as force**

Elicit axes covering: historical memory depth, trauma transmission mode, legacy visibility, relationship to the past (reverence vs. amnesia), contested narratives. Each axis: slug, name, description, axis_type, provisional values, rough surfacing.

Note: history axes describe *how* the past operates as a force, not specific events (which are lore entities).

- [ ] **Step 2: Write domain file**

Write `history-as-force.json` with domain metadata, axes, empty `_deferred`, `_meta` with `pass: 1`.

- [ ] **Step 3: Validate skeleton gate**

Check: at least 4 axes, no obvious gaps. Verify axes describe mechanisms, not events.

---

### Task 7: Pass 1 — Aesthetic and Cultural Forms Skeleton

**Files:**
- Create: `storyteller-data/narrative-data/tome/domains/aesthetic-cultural-forms.json`

**Input:** `docs/foundation/tome_and_lore.md` (aesthetic/cultural forms domain).

**Work:**
- [ ] **Step 1: Seed 4-6 axes for aesthetic and cultural forms**

Elicit axes covering: artistic tradition strength, ritual density, architectural style, food culture complexity, speech/dialect variation, performative culture. Each axis: slug, name, description, axis_type, provisional values, rough surfacing.

- [ ] **Step 2: Write domain file**

Write `aesthetic-cultural-forms.json` with domain metadata, axes, empty `_deferred`, `_meta` with `pass: 1`.

- [ ] **Step 3: Validate skeleton gate**

Check: at least 4 axes, no obvious gaps.

---

### Task 8: Pass 1 Commit and Review

- [ ] **Step 1: Commit all Pass 1 domain files**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/domains/*.json
git commit -m "feat: Tome Phase 1 Pass 1 — skeleton axes across 6 domains"
```

- [ ] **Step 2: Review checkpoint**

Count total axes across all domains. Target: 24-36 (4-6 per domain). Review cross-domain observations: are any axes in the wrong domain? Are there obvious gaps visible now that the full skeleton is laid out? Note observations in domain-level `_commentary`.

**Decision point:** If the skeleton reveals structural issues (domains that don't work, axis types that don't fit, framework-level problems), address them before proceeding to Pass 2. If the skeleton looks solid, proceed.

---

### Task 9: Pass 2 — Elaborate All Domains

**Files:**
- Modify: all 6 domain files in `storyteller-data/narrative-data/tome/domains/`

**Work:**
- [ ] **Step 1: Elaborate material conditions to 8-12 axes**

For each existing axis: refine description, confirm axis_type and values, add full surfacing argument (agents, pipeline_stages, pathways, relevance, prose). Elicit new axes: "What other axes describe this space? What's missing?" Add `_commentary` and `_suggestions` per axis. New axes get `provenance: "elaborated"`.

- [ ] **Step 2: Write updated material-conditions.json**

Update `_meta.pass` to 2, `_meta.axis_count`, `_meta.last_updated`.

- [ ] **Step 3: Elaborate economic forms to 8-12 axes**

Same process: refine existing, elicit new, full surfacing arguments.

- [ ] **Step 4: Write updated economic-forms.json**

- [ ] **Step 5: Elaborate political structures to 8-12 axes**

Same process.

- [ ] **Step 6: Write updated political-structures.json**

- [ ] **Step 7: Elaborate social forms to 8-12 axes**

Same process.

- [ ] **Step 8: Write updated social-forms.json**

- [ ] **Step 9: Elaborate history as force to 8-12 axes**

Same process. Extra attention to keeping axes as mechanisms, not events.

- [ ] **Step 10: Write updated history-as-force.json**

- [ ] **Step 11: Elaborate aesthetic and cultural forms to 8-12 axes**

Same process.

- [ ] **Step 12: Write updated aesthetic-cultural-forms.json**

- [ ] **Step 13: Validate elaboration gate**

Every axis has a complete surfacing argument with relevance conditions. No axis has vague or placeholder surfacing. `_commentary` captured where design questions remain.

- [ ] **Step 14: Commit Pass 2**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/domains/*.json
git commit -m "feat: Tome Phase 1 Pass 2 — elaborated axes with full surfacing arguments"
```

---

### Task 10: Pass 3 — Genre-Region Coverage Test

**Work:**
- [ ] **Step 1: Folk-horror village test**

Attempt to sketch a folk-horror village's position on every axis across all 6 domains. For each axis, note:
- The village's position (specific value or range)
- Whether the axis feels relevant (high/medium/low/irrelevant)
- Any gaps — things the village wants to express that no axis captures

Expected: dense material conditions, strong history-as-force, tight social forms, thin economic complexity. Some axes should feel irrelevant — that's fine.

- [ ] **Step 2: Cyberpunk megacity test**

Same exercise. Expected: heavy economic/political/infrastructure, weak history visibility, fragmented social forms. Different axes should feel irrelevant than the village test.

- [ ] **Step 3: Epic-fantasy kingdom test**

Same exercise. Expected: full spread across all domains, high aesthetic-cultural weight. Should stress the world affordance axes (magic, divine responsiveness).

- [ ] **Step 4: Gap analysis**

Compile gaps from all three tests. If multiple test worlds independently identify the same missing expressiveness, that's a strong signal for a new axis. Add discovered axes (`provenance: "discovered"`) to the appropriate domain files.

---

### Task 11: Pass 3 — Cross-Domain Review

**Work:**
- [ ] **Step 1: Overlap check**

Review all axes across all domains. Are any axes redundant or near-duplicates in different domains? Merge or differentiate as needed.

- [ ] **Step 2: Distribution check**

Count axes per domain. Target: 8-12 each, ~50-72 total. Flag domains below 6 (under-specified) or above 15 (too granular).

- [ ] **Step 3: Provenance audit**

Review `discovered` axes from Task 10. Are they in the right domain? Do they have complete surfacing arguments?

- [ ] **Step 4: Commentary extraction**

Review `_commentary` and `_suggestions` across all domains. Extract cross-cutting patterns into domain-level `_meta._commentary`. Note emerging themes that may inform Phase 2 mutual production graph work.

- [ ] **Step 5: Write final domain files**

Update all 6 domain files with Pass 3 changes. Set `_meta.pass` to 3.

- [ ] **Step 6: Commit Pass 3**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/domains/*.json
git commit -m "feat: Tome Phase 1 Pass 3 — coherence review and coverage validation"
```

---

### Task 12: Final Validation and Session Save

- [ ] **Step 1: Final axis count and distribution report**

Print summary: total axes, per-domain count, deferred count, provenance breakdown (seed/elaborated/discovered).

- [ ] **Step 2: Commit design spec to storyteller repo**

The design spec is already committed. Verify it's up to date with any changes made during elicitation.

- [ ] **Step 3: Save session note**

```bash
temper session save "Tome Phase 1: Seed Axes and Domain Clusters" \
  --ticket 2026-03-25-tome-phase-1-seed-axes-and-domain-clusters \
  --state done \
  --project storyteller
```

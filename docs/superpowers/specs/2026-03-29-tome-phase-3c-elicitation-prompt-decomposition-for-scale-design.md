# Tome Phase 3c: Elicitation Prompt Decomposition for Scale — Design Spec

## Problem

The Phase 3b elicitation pipeline produces rich, coherent worlds but doesn't scale. Each
step sends a single large prompt (45-61 KB) to qwen3.5:35b, taking 5-10 minutes per call.
A full world requires 5 sequential calls (~40 minutes). For dozens of genre x setting
combinations, this is infeasible.

The dominant cost is the world preamble: 36.5 KB of raw edge-traversal traces from the
mutual production graph, repeated identically across all 5 pipeline stages. The LLM uses
axis values to ground its output but does not need the graph provenance chains.

## Approach: Fan-Out / Fan-In with Model Tiering

Decompose the pipeline into two phases per entity stage:

- **Fan-out**: Small model generates individual entities in parallel, each with a focused
  axis subset. Fast, structured, independently retryable.
- **Coherence**: Larger model takes the entity inventory and performs relational binding —
  connecting entities to each other and to upstream context. One call per stage, operating
  on entity summaries rather than raw axes.

This separates **entity generation** (embarrassingly parallel, needs focused context) from
**relational binding** (needs to hold multiple entities simultaneously, genuinely harder).

### Model Tiering

| Role | Model | Use Case |
|------|-------|----------|
| `fan_out_structured` | qwen2.5:7b-instruct | Reliable JSON output, fast. Places, orgs, substrate, Q1-Q2 characters. |
| `fan_out_creative` | qwen3.5:9b | Richer descriptions, archetype mapping. Q3-Q4 character skeletons. |
| `coherence` | qwen3.5:35b | Relational binding, boundary tensions, stated/operative gaps. All coherence passes. |

Model names are configured in a registry, not hardcoded per module.

## Pipeline Architecture

```
compose-world (existing, unchanged)
    |
compress-preamble (pure Python, no LLM)
    | -> world-summary.json
    |
planning-call (7b x 1)
    | -> entity-plan.json (counts and distribution per type)
    |
Stage 1: Places
    fan-out: 7b x N (parallel, one place per call)
    coherence: 35b x 1
    -> places.json
    |
Stage 2: Organizations
    fan-out: 7b x M (parallel, one org per call)
    coherence: 35b x 1
    -> organizations.json
    |
Stage 3: Social Substrate
    fan-out: 7b x K (parallel, one cluster per call)
    coherence: 35b x 1
    -> social-substrate.json
    |
Stage 4a: Mundane Characters (Q1-Q2)
    fan-out: 7b x C (parallel, one character per call)
    coherence: 35b x 1 (light touch)
    -> characters-mundane.json
    |
Stage 4b: Significant Characters (Q3-Q4)
    fan-out: 9b x C (parallel, one character per call)
    coherence: 35b x 1 (DEEP -- the load-bearing call)
    -> characters-significant.json
    |
python-compose (no LLM)
    -> world.json (aggregation of all per-file outputs)
```

### Sequential Dependencies Between Stages

Stages have sequential dependencies between entity types (orgs need place names, clusters
need org names, characters need cluster assignments). Entities within a type are parallel.

- Stage 1 (places): needs world-summary only
- Stage 2 (orgs): needs world-summary + places.json
- Stage 3 (substrate): needs world-summary + places.json + organizations.json
- Stage 4a (mundane chars): needs world-summary + substrate + places + orgs
- Stage 4b (significant chars): needs all of the above + mundane chars + archetypes + dynamics

### Fan-Out Width: One Entity Per Call

Each fan-out call generates a single entity. This maximizes:
- **Granularity of reuse**: Individual entities can be recombined, re-prompted, or
  selectively regenerated without touching siblings.
- **Prompt focus**: Each call gets only the axis subset and upstream context relevant
  to that specific entity.
- **Debuggability**: Which call failed? Which skeleton was weak?

Ollama serializes inference on a single GPU, so parallelism is about having requests
queued and ready (ThreadPoolExecutor with max_workers=4), not actual concurrent inference.

### Planning Call

A single 7b call reads the world-summary and outputs the entity plan:

```json
{
  "places": {
    "count": 12,
    "distribution": {
      "infrastructure": 3,
      "gathering-place": 4,
      "production-site": 3,
      "settlement": 2
    }
  },
  "organizations": { "count": 5 },
  "clusters": { "count": 4 },
  "characters_mundane": { "q1_count": 6, "q2_count": 4 },
  "characters_significant": { "q3_count": 3, "q4_count": 2 }
}
```

The planning call lets the world position drive entity distribution rather than static
genre defaults. A resource-scarce mountainous world should have more infrastructure places
and fewer gathering places than a fertile village.

The plan output drives fan-out spec generation. For places, each fan-out call receives
its assigned `place_type` from the distribution. For organizations, the plan may suggest
org types (governance, economic, religious, etc.) based on axis values. For clusters, the
plan assigns a basis hint (blood, occupation, belief, geography, affiliation) informed by
the `kinship-system` axis. For characters, the plan assigns cluster membership and
centrality tier. The `plan_*` functions in the orchestrator translate the entity-plan into
concrete `FanOutSpec` objects — each spec is a complete, self-contained prompt input for
one fan-out call.

## Compressed Preamble

The world preamble is rebuilt from world-position.json as a domain-grouped, values-only
summary. Edge-traversal justifications are dropped entirely.

**Current format (36.5 KB):**
```
- **trauma-transmission-mode**: embodied-somatic (confidence: 1.00) -- historical-memory-depth
  ->enables-> trauma-transmission-mode (w=0.7); relationship-to-past ->constrains-> ...
  [754 chars of edge traces per position]
```

**Compressed format (~3-4 KB):**
```markdown
### Material Conditions
- geography-climate: temperate-maritime [seed]
- resource-profile: soil-fertility:abundant, potable-water:moderate, ... [seed]
- disease-ecology: Clean [seed]
- supernatural-permeability: Permeable [seed]
- technological-ceiling: medieval-craft [seed]
- population-density: village-clusters [seed]
- biological-plasticity: transformable
- physical-law-flexibility: strict-realism
- divine-responsiveness: transactional

### Social Forms
- kinship-system: clan-tribal
- social-stratification: racial-ethnic
- gender-system: sacred-ritual
- religious-cosmological-framework: syncretic-plural
- community-cohesion: mid
- social-mobility: mid
- knowledge-system-structure: technical-vocational
- knowledge-access-stratification: high
- outsider-integration-pattern: cosmopolitan-indifferent

### Economic Forms
- production-mode: mercantile-trade
- trade-network-reach: post-scarcity-distribution
- labor-organization: household-subsistence
- exchange-and-obligation: tribute-extraction
- currency-and-exchange-medium: commodity-money
- wealth-concentration: mid
- land-tenure-system: state-owned-allocated
- economic-volatility: high

### Political Structures
- power-concentration: mid
- authority-legitimation: meritocratic-technocratic
- legal-system-formality: customary-oral
- enforcement-capacity: low
- institutional-density: high
- sovereignty-type: imperial-external
- corruption-institutional-decay: high
- political-violence-norm: endemic

### History as Force
- historical-memory-depth: total-recorded
- trauma-transmission-mode: embodied-somatic
- relationship-to-past: high
- legacy-visibility: high
- historical-instrumentalization: high
- temporal-experience-mode: eschatological-terminal
- founding-myth-strength: low
- historiographic-control: high

### Aesthetic-Cultural Forms
- ritual-density: mid
- oral-vs-literate-culture: low
- aesthetic-register: sacred-symbolic
- performative-culture-strength: low
- food-culture-complexity: high
- expressive-autonomy: mid
- language-register-complexity: low
- craft-and-material-culture: mid
```

Seeds are labeled. Domain grouping provides semantic structure. The LLM gets the axis
landscape without the graph provenance.

### Axis Subsetting Per Entity Type

Fan-out calls receive a subset of axes relevant to their entity type:

| Entity Type | Primary Axis Domains | Supplementary |
|-------------|---------------------|---------------|
| Places | material-conditions, aesthetic-cultural-forms | genre spatial functions |
| Organizations | economic-forms, political-structures, social-forms | -- |
| Clusters | social-forms, material-conditions | -- |
| Q1-Q2 characters | social-forms + cluster assignment | relevant place context |
| Q3-Q4 characters | social-forms + material-conditions + cluster + boundary | archetype data |

Coherence calls receive the full compressed preamble (all domains) since they need the
complete axis landscape to bind entities relationally.

## Character Coherence: Tiered Treatment

### Stage 4a: Mundane Characters (Q1-Q2) — Light Touch

The mundane coherence pass is **editorial**:
- Cluster distribution: are clusters evenly populated?
- Naming consistency: do names fit the world's cultural register?
- No duplicate roles within the same cluster
- Place associations: do characters connect to relevant places?

This is a review-and-adjust pass, not a generative one.

### Stage 4b: Significant Characters (Q3-Q4) — Deep Pass

The Q3-Q4 coherence pass is **generative**. This is the load-bearing call where the
design philosophy lands in the data.

Q3-Q4 characters are the heart of expressed agency turn by turn — the locus at which
entanglement-as-price-of-agency drives the rendering of multivalent intention and desire,
scene-to-arc-to-orbital goals.

The coherence prompt for this stage:

1. **Frames the design principle explicitly**: "Ascending centrality simultaneously
   increases capacity to act and constraints on action. The more a character can do,
   the more the world can do to them."

2. **Binds relational seeds into a web**: The 35b sees all significant characters
   simultaneously and binds them to each other and to the mundane roster using verb:slug
   format.

3. **Grounds stated/operative gaps in position**: The model needs the full social
   substrate to understand why a character's public role diverges from their actual
   function. Gaps emerge from position, not imagination.

4. **Inflects personality profiles by world-position**: A Warden in a mining community
   is materially different from a Warden in an agrarian village. The archetype baseline
   gets adjusted for this character's specific world-position and social entanglement.

5. **Creates vertical goal coherence**: Existential stakes ground arc goals, arc goals
   motivate scene goals, all three constrained by social position.

6. **Verifies cross-character tension**: Do the significant characters create productive
   tension with each other, not just with the substrate?

**Input to Q3-Q4 coherence call (~23-25 KB):**
- Compressed preamble (~4 KB)
- Entity summaries from all prior stages (~8-10 KB)
- Archetype data + dynamics (~3 KB)
- Q3-Q4 skeletons from fan-out (~4 KB)
- Coherence task instructions with design principle (~4 KB)

This is less than half the current significant character prompt (61 KB), and the context
is denser — entity summaries instead of raw axis traces.

## Output Structure

### Directory Layout

```
storyteller-data/narrative-data/tome/worlds/{world-slug}/
    world-position.json              (existing, unchanged)
    places.json                      (existing baseline — preserved)
    organizations.json               (existing baseline)
    social-substrate.json            (existing baseline)
    characters-mundane.json          (existing baseline)
    characters-significant.json      (existing baseline)
    decomposed/
        world-summary.json           (compressed preamble + domain-grouped axes)
        entity-plan.json             (planning call output)
        fan-out/
            places/
                instance-001.json    (single place skeleton)
                instance-002.json
                ...
            orgs/
                instance-001.json
                ...
            substrate/
                instance-001.json
                ...
            characters-mundane/
                instance-001.json
                ...
            characters-significant/
                instance-001.json
                ...
        places-draft.json            (aggregated fan-out skeletons)
        places.json                  (coherence output — final)
        orgs-draft.json
        organizations.json
        substrate-draft.json
        social-substrate.json
        characters-mundane-draft.json
        characters-mundane.json
        characters-significant-draft.json
        characters-significant.json
        world.json                   (python-composed aggregate)
```

Existing baseline files are preserved untouched for side-by-side comparison.

### Draft vs. Final

Each stage produces:
- **Draft**: Aggregated fan-out skeletons (pre-coherence). Kept for debugging, ablation,
  and coherence prompt iteration.
- **Final**: Coherence output. Same schema as current pipeline outputs. Feeds forward to
  downstream stages.

## Prompt Templates

### Fan-Out Prompts (6 new templates)

Small, focused prompts. Each generates a single entity:

| Template | Model | Input | Output |
|----------|-------|-------|--------|
| `place-fanout.md` | 7b | axis subset + spatial function + type hint | single place JSON |
| `org-fanout.md` | 7b | axis subset + place names + org type hint | single org JSON |
| `substrate-fanout.md` | 7b | axis subset + place/org names + cluster hint | single cluster JSON |
| `character-mundane-fanout.md` | 7b | cluster + place + social axes | single Q1/Q2 char JSON |
| `character-significant-fanout.md` | 9b | boundary position + archetype + axes | single Q3/Q4 skeleton JSON |
| `entity-plan.md` | 7b | world-summary + genre profile | entity-plan JSON |

### Coherence Prompts (5 new templates)

Larger prompts that operate on entity inventories:

| Template | Model | Input | Task |
|----------|-------|-------|------|
| `places-coherence.md` | 35b | place skeletons + world-summary | spatial relationships, grounding review |
| `orgs-coherence.md` | 35b | org skeletons + places + world-summary | org-place binding, power structure |
| `substrate-coherence.md` | 35b | cluster skeletons + places + orgs + world-summary | pairwise relationships, boundary tensions |
| `characters-mundane-coherence.md` | 35b | char skeletons + substrate + world-summary | cluster distribution, naming, associations |
| `characters-significant-coherence.md` | 35b | char skeletons + all prior + archetypes + dynamics + design principle | relational seeds, gaps, profiles, goals |

## Orchestration

### Module: `orchestrate_decomposed.py`

Owns the full pipeline. Key functions:

- `orchestrate_world(data_path, world_slug)` — run the full decomposed pipeline
- `compress_preamble(world_pos)` — pure Python, produces domain-grouped summary
- `plan_entities(world_summary, genre_profile)` — 7b planning call
- `fan_out(specs, model)` — parallel dispatch via ThreadPoolExecutor(max_workers=4)
- `aggregate(instances)` — collect fan-out results into draft array
- `cohere(draft, upstream_context, model)` — 35b coherence call
- `compose_world_json(...)` — pure Python aggregation of per-file outputs

### Retry and Failure Handling

- Fan-out calls that fail JSON parsing: retry once with "output valid JSON only" suffix
- If an instance still fails: log, skip, coherence works with N-1 entities
- Coherence call failure: hard failure, blocks the stage
- Atomic writes: write to temp file, rename on success
- Resumable: `--stage` flag allows re-running from a specific stage using existing upstream outputs

### CLI Interface

```bash
# Full decomposed pipeline
uv run narrative-data tome elicit-decomposed --world-slug mccallisters-barn

# Single stage
uv run narrative-data tome elicit-decomposed --world-slug mccallisters-barn --stage places

# Re-run coherence only (use existing fan-out drafts)
uv run narrative-data tome elicit-decomposed --world-slug mccallisters-barn --stage places --coherence-only

# Compare decomposed vs baseline
uv run narrative-data tome compare-worlds --world-slug mccallisters-barn
```

## Ablation Testing

### Strategy

Run the decomposed pipeline on McCallister's Barn first. Compare against the existing
Phase 3b baseline outputs. If quality is roughly on par, run the remaining 3 worlds.

### Comparison Dimensions

For each entity type, compare baseline vs. decomposed:

1. **Axis grounding** — do descriptions reference material conditions from the world position?
2. **Inter-entity references** — do orgs cite places? do characters cite clusters and
   mundane characters by slug?
3. **Material specificity** — are entities distinct to this world or generic to the genre?
4. **Relational density** — count and quality of relational seeds (especially verb
   diversity for Q3-Q4)
5. **Stated/operative gap quality** — are gaps grounded in social position or generic
   character flaws?

### Success Criteria

- At least 2x speedup per world (from ~40 min to ~20 min or better)
- No measurable loss in axis grounding, inter-entity references, or material specificity
- Relational seed quality in Q3-Q4 characters comparable to Phase 3b Round 2 baseline
- Decomposed pipeline produces comparable output on McCallister's Barn

## What We Are NOT Doing

- Changing the output schema or data model (decomposed outputs match current schema)
- Integrating spatial-topology/settings data into place generation (noted as seam for
  follow-on work — the decomposed architecture makes this incremental integration cheap)
- Building a streaming/async pipeline (orchestration is synchronous with parallel fan-out)
- Reducing output quality to gain speed (Phase 3b Round 2 is the quality bar)
- Changing the world composition step (compose-world is unchanged)

## Future Seams

- **Spatial-topology integration**: Genre settings data has rich per-setting narrative
  functions and communicability profiles. The fan-out architecture makes feeding individual
  setting archetypes to individual place generation calls straightforward.
- **Narrative landscape generation**: Per-file outputs (world.json components) serve as
  selective inputs to downstream narrative-landscape.json generation without requiring
  consumers to parse a monolith.
- **Selective regeneration**: One-per-call fan-out means individual entities can be
  re-prompted without re-running the full stage.

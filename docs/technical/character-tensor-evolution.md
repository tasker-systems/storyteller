# Character Tensor Evolution: From First-Pass to Genre-Constituted Prediction

**Date:** 2026-03-21
**Status:** Design exploration — bridging Tier B data generation and Tier C ML inference
**Context:** The narrative data corpus (~4.8MB across 30 genres: archetypes, settings, ontological posture, profiles, tropes, narrative shapes, dynamics, goals) has revealed that the character tensor's current form is insufficient for the richness of the data it must represent.

---

## The Problem

The current ML pipeline takes a character tensor (personality axes, relational state) and predicts behavioral outputs. This was always understood as a first approximation. The Tier B data generation has now made explicit *why* it is insufficient and *what* a fuller model requires.

### What the data shows

A character is not a personality profile that happens to exist in a genre context. A character is *constituted by* their genre context. The Earnest Warden in folk horror isn't a generic Keeper with a folk-horror skin — their warmth, authority, and morality are *produced by* the genre's locus of power (the Land), its epistemological stance (knowledge is punished), and its ontological posture (the Land has agency, humans are intermediaries). Remove the genre, and the archetype loses its defining tensions.

This means the character tensor cannot be a vector of personality values with genre as an external modifier. Genre is intrinsic to the tensor's meaning. The same `warmth: 0.8` value produces fundamentally different behavioral predictions depending on whether the genre transforms it through folk horror's "earnest stewardship that demands sacrifice" or cozy fantasy's "genuine nurturing that builds sanctuary."

---

## Five Threads

### Thread 1: The Tensor Is No Longer Just the Character

The input to the prediction model must include:

- **Genre dimensions** — the ~30 universal axes that position the story in narrative space, including thematic treatments as weighted tag sets
- **Archetype positioning** — where this character sits within their archetype's axes, expressed as midpoints and variance bounds `[n, m, y, x]` per axis
- **Narrative beat position** — where in the active narrative shape(s) this scene falls, including tension contribution and pacing function
- **Scene profile** — the dramatic situation type, its characteristic tension signature, information flow, and emotional register
- **Relational graph state** — not just dyadic trust values but the multi-scale dynamics (orbital, arc, scene) across all active edges, including nonhuman entity relationships
- **Goal-scale alignment** — the current state of the character's existential, arc, and scene goals, and where they are in tension with each other
- **Ontological posture context** — the genre's stance toward who counts, which determines how the character relates to nonhuman agents and where the self/Other boundary sits

This is a substantially larger and more structured input than personality axes plus scene content. But it is not unbounded — the data we've generated defines the vocabulary and the bounds.

### Thread 2: The Relational Representation Must Evolve

The current `trust_in_X` style rendering of relational state is a flat vector of trust values per relationship. The dynamics data shows this is inadequate:

- **Multi-scale state**: A relationship carries orbital (bedrock), arc (sediment), and scene (topsoil) state simultaneously. Two characters might have orbital enmity, arc-level forced cooperation, and scene-level grudging respect — all active at once, creating the dramatic friction that makes the scene interesting.
- **Edge properties**: Dynamics produce typed edges — debt-laden, parasitic, symbiotic, information-asymmetric, projection-heavy. These aren't reducible to a single trust value.
- **Directionality and asymmetry**: Each participant may experience the dynamic differently. The Warden's relationship to the Outsider is duty; the Outsider's relationship to the Warden is fear. Same edge, different tensors.
- **Nonhuman entities**: The character has a relational state with the Land, the System, the City, their own augmented body. These aren't optional metadata — they're load-bearing edges in the relational graph.
- **Graph topology**: A character positioned as a broker between disconnected groups has power that isn't captured by any single edge. Network position is data.

The relational tensor must be a structured representation of edges at multiple scales, not a flat vector. The geological strata metaphor applies: bedrock relations constrain what sediment relations are possible, which constrain what topsoil interactions emerge in a given scene.

### Thread 3: The Dimensional Heterogeneity Problem

Different genres activate different axis sets:

| Genre | Genre-Specific Axes |
|-------|-------------------|
| Cyberpunk | Heat, Net Access, Gear Integrity, Stability (identity) |
| Folk Horror | Integration Level, Time to Ritual, Sanity/Knowledge |
| Cozy Fantasy | Sanctuary Integrity, Harmony, Threat Level, Competence |
| Classical Tragedy | Hubris, Divine Favor, Fate Fulfillment |
| Survival Fiction | Health, Hunger, Thirst, Exhaustion, Supplies |
| Romance | Bond Level, Vulnerability, Self-Knowledge, Guardedness |

These are not all aliases of the same canonical variables (though some are — the renaming problem is solved with canonical IDs). Some are genuinely *different dimensions* that don't exist in other genres. Sanctuary Integrity has no meaningful analog in cyberpunk. Heat has no meaningful analog in pastoral fiction.

This means the tensor *shape* varies by genre. A folk-horror character tensor has different dimensionality than a cyberpunk character tensor. You cannot naively feed one into a model trained on the other.

**Dimension reduction** (PCA, autoencoders, or learned embeddings) is required to project genre-specific tensors into a shared latent space that preserves the principal predictive components regardless of which genre-specific axes contributed them. The shared latent space is where the model operates; the genre-specific axes are the input features that get projected into it.

This is a pre-processing step that must happen before training data generation. The pipeline:

```
Genre-specific tensor (variable dimensionality)
  → Dimension reduction (PCA / learned projection)
  → Shared latent representation (fixed dimensionality)
  → Model input
```

The 8 canonical state variables (V1-V8 from the axis inventory) provide a natural starting point for the shared dimensions, with genre-specific axes projecting additional variance into the latent space.

### Thread 4: Genre as Weights-and-Biases Transform

A character has a **base tensor** — their archetypal identity with personal variance. Formally:

```
base_tensor = archetype_midpoints + character_variance
```

Where `archetype_midpoints` are the typical axis positions for their archetype(s), and `character_variance` is the authored deviation that makes them a unique individual within their archetype.

The genre applies a **transform** — not replacing the character but weighting how their axes express:

```
effective_tensor = genre_transform(base_tensor)
```

This transform encodes the `profile_shift` concept from the type design as a mathematical operation. It is not a simple additive shift (though that's a first approximation). More accurately, the genre transform:

- **Reweights axes**: Warmth is load-bearing in folk horror and romance but incidental in survival fiction. The genre transform amplifies or attenuates each axis's contribution to prediction.
- **Activates conditional dimensions**: The genre's world affordances gate which axes exist. Heat only activates when Technology = Speculative AND setting includes surveillance infrastructure.
- **Constrains value ranges**: The genre's exclusions establish hard bounds. In cozy fantasy, violence axes cannot exceed the genre's non-lethal constraint. In romance, the bond level must reach the HEA threshold by resolution.
- **Establishes inter-axis constraints**: The genre's dynamics data defines how axes relate to each other (as integration_level rises, warmth drops — in folk horror specifically).

This means genre is not metadata applied after the tensor is composed. It is a **transformation matrix** applied during composition, shaping what the tensor can express before it ever reaches the prediction model.

For characters constituted by *multiple archetype influences* (which is the norm — few characters are a pure single archetype), the composition becomes:

```
base_tensor = weighted_sum(archetype_1_midpoints * influence_1,
                           archetype_2_midpoints * influence_2, ...)
              + character_variance

effective_tensor = genre_transform(base_tensor)
```

Where `influence_N` represents how strongly each archetype contributes to this character's identity, and the genre transform applies uniformly across the composed result.

### Thread 5: Training Data Generation

The generated narrative data provides the vocabulary and bounds for synthetic training data generation. The pipeline:

1. **Extract structured JSON from raw.md files** — use the existing 7b-instruct structuring pipeline to extract axis values, bounds, and relationships from the rich markdown corpus. This is the deferred Stage 2 work, now load-bearing.

2. **Define self-describing bounds per axis per archetype per genre** — from the extracted data, establish the range within which each axis can vary for a given archetype in a given genre. The Earnest Warden's warmth in folk horror has a range; the Heat-Marker's visibility in cyberpunk has a range. These ranges *are* the data.

3. **Apply dimensionality reduction** — project the heterogeneous genre-specific tensors into a shared latent space. PCA is the simplest approach; a learned projection (autoencoder trained on the full corpus) would capture nonlinear relationships between axes. The choice depends on how much of the variance is linear.

4. **Apply genre transform** — encode the genre's weights-and-biases effect as a transformation matrix that can be applied to any base tensor. This matrix is derived from the `profile_shift` data across all archetypes within the genre.

5. **Generate synthetic character instances** — sample within the defined bounds to produce character tensors that are archetypal-valid and genre-consistent. The sampling is bounded by the data, not infinite.

6. **Generate training pairs** — the prediction target is not "what action does this character take" (that's rules-engine thinking). It is **"what is this character's intentional orientation given their full contextual state"** — a behavioral disposition vector that the narrator or dramaturge translates into genre-appropriate expression. The training pair is:

```
Input:  (effective_tensor, scene_context, relational_state,
         narrative_beat_position, goal_alignment)
Output: (intentional_orientation — a disposition vector
         representing behavioral tendencies, emotional state,
         and relational stance in this moment)
```

The intentional orientation is what gets passed to the narrator for rendering into prose, or to the dramaturge for dramatic shaping.

---

## What This Means for Architecture

### The ML Model Changes

The current model (personality tensor → behavioral prediction) becomes:

```
Character-as-constituted-by-context
  (genre_transform(archetype_composition + variance),
   multi-scale relational state,
   narrative beat position,
   goal-scale alignment)
→ Intentional orientation
→ [Narrator renders] or [Dramaturge shapes]
```

This is a substantially larger model with a structured input, not a flat feature vector. The architecture likely needs:
- An embedding layer for genre dimensions (the ~30 universal axes)
- A tensor composition layer for archetype blending with genre transform
- A graph-aware layer for multi-scale relational state
- A temporal-position encoding for narrative beat and goal alignment
- A prediction head that outputs intentional orientation as a disposition vector

### The Data Pipeline

```
Tier B generated data (raw.md, ~4.8MB)
  → Stage 2 JSON structuring (7b-instruct)
  → Axis/bound extraction per archetype per genre
  → Dimensionality reduction to shared latent space
  → Genre transform matrix derivation
  → Synthetic character instance generation
  → Training pair generation (tensor + context → orientation)
  → Model training
  → Integration with context assembly pipeline
```

### Connection to Event Extraction

The prediction model's output (intentional orientation) must connect to the event extraction pipeline. Event extraction currently uses a small instruct LLM (qwen2.5:3b-instruct) for structured entity-and-event identification via constrained JSON output. (DistilBERT was removed in a prior branch — it provided no value compared to the structured instruct model output.)

The enriched prediction model should inform:
- **What the character is likely to do** (intentional orientation → probable actions)
- **How the character interprets what happened** (orientation shapes perception of events)
- **What the character notices and ignores** (orientation as attentional filter)

A critical insight: the current approach of "laddering atomic events up to formal narrative events" via deterministic mechanisms is likely insufficient. Identifying when a sequence of atomic events constitutes a narratively significant event (a betrayal, a revelation, a turning point) is fundamentally a guided-subjective evaluation about thresholds — not a rules-engine problem. Smaller instruct models are better at this kind of threshold judgment than deterministic pipelines that try to enumerate what counts. The entity-and-event identification pass remains valuable for structured extraction, but it will need to be complemented by model-driven significance assessment, potentially using the intentional orientation data to contextualize what "matters" for each character.

---

## Design Decisions (Mar 21 2026)

1. **Dimension reduction: learned embeddings preferred over PCA.** PCA assumes linear relationships between axes. The data shows nonlinear interactions (warmth and authority interact differently under different epistemological stances). Learned embeddings (autoencoder) can capture these. PCA remains a useful analytical tool for understanding the space, but the production pipeline should use learned projections. Decision to be validated empirically as we go.

2. **Archetype composition: one primary + one shadow.** Opinionated stance: a character has one primary archetype and at most one subverting-or-transforming archetype. Not because Jungian psychology holds together rigorously, but because the felt-sense of a Shadow gives the narrative engine the possibility of a conscious-or-unconscious Undertow — a counter-tendency that can be driven by the data itself. The primary archetype defines the character's intentional orientation; the shadow archetype defines the currents that pull against it. More than two risks losing distinctiveness into incoherence.

3. **Intentional orientation dimensionality: to be determined empirically.** The output vector needs enough dimensions for behavioral tendency, emotional state, relational stance, and communicative mode — but not so many that the narrator can't translate it. We'll figure this out as we build, which is the right approach for a research-stage system.

4. **Genre transform: dynamic, not static.** The genre transform must be re-evaluated as state variables change. Sanity degradation in cosmic horror doesn't just change the character's state — it changes *what the genre is doing* to them. The genre is a live physics engine that responds to state evolution, not a fixed initial condition. This connects to the event extraction question: a narratively significant event may be one that shifts the genre transform itself (a threshold crossing that changes the genre's relationship to the character).

5. **Training data volume: ~10K-50K pairs estimated, to be validated empirically.** The combinatorial space is bounded by the data we've generated. Empirical validation will determine the actual requirement.

6. **Inference deployment: ONNX as first target, burn/candle as fallback.** The current inference pipeline uses ort (ONNX Runtime). The new model's structured input may exceed ONNX's comfortable expressiveness. burn and candle are available as Rust-backed alternatives if we need more deeply customized architecture (custom graph layers, dynamic genre transforms). The door is open.

---

## The Turn Loop: Where Everything Converges

All of this work — genre modeling, archetype data, relational graphs, narrative topologies, character tensors, event extraction, ML prediction — is only valuable insofar as it is represented and available within the turn-over-turn gameplay loop. The turn loop is the architectural chokepoint where the system either delivers on its promises or fails.

### The Storykeeper as Deterministic Information Boundary

The Storykeeper is a meta-function for information boundary management. It is not an agent with subjective judgment (though it could be, and tools exist for that path). It operates deterministically because:
- **Performance**: a blocking LLM call in the information-gathering phase adds latency that compounds with every other agent's needs
- **Narrative coherence**: the boundaries between what agents know must be consistent and inspectable, not probabilistic

The Storykeeper needs to be able to:

1. **Query the event ledger** — run bounded recursive-CTE-DAG queries over the append-only event log. "What has happened that is relevant to this character in this scene?" is a graph traversal with depth limits, not a full-corpus search. The event ledger's structure (entity references, event types, temporal ordering) supports this.

2. **Traverse the correct graphs with correct weights** — the relational graph carries multi-scale dynamics (orbital, arc, scene) with friction and gravity weights per edge. The narrative graph carries scene connectivity with gravitational mass. The setting topology carries spatial adjacency with permeability. The Storykeeper must traverse the *right* graph with the *right* weights for the query at hand. "What is this character's relational state?" traverses the relational graph. "What scenes are gravitationally near?" traverses the narrative graph. "Can this character physically reach that location?" traverses the setting topology.

3. **Select the correctly persisted character tensor** — with the expanded bounds described in this document (genre-transformed archetype composition + multi-scale relational state + goal alignment). The tensor must be retrievable as a structured object, not reconstructed from scattered fields.

4. **Present to the correct aspects of context assembly** — different agents need different slices. The Narrator needs the rendered scene context and character intent. The Dramaturge needs narrative beat position and genre physics. Character Agents need their filtered tensor and the relational edges they can see. The World Agent needs environmental state and ontological posture data. The Storykeeper *selects and routes* — it doesn't interpret.

### Asynchronous Agent Integration

More than one agent operates outside the turn-based play loop:
- The **Dramaturge** runs asynchronously, layering per-turn dramatic directives into a channel that context assembly reads from. Its analysis of narrative beat position, pacing function, and genre-appropriate tension contribution should be *available* for context assembly but without a blocking expectation. If the Dramaturge hasn't finished analyzing the current turn's position when context assembly runs, the previous turn's directive is used.
- The **ML prediction pipeline** (character tensor → intentional orientation) runs on a dedicated thread pool. Predictions for the current scene's characters should be *available* when context assembly runs but without blocking on stragglers.
- **Graph maintenance** (updating relational edges, narrative positions, entity states after each committed turn) runs after the turn is committed but before the next turn's context assembly. This is the "committing previous" phase of the turn cycle.

The principle: **deterministic routing with async enrichment**. The Storykeeper's queries are deterministic and bounded. The enrichment from async agents (Dramaturge directives, ML predictions, graph updates) flows into the context assembly as it becomes available. Missing enrichment degrades gracefully — the system always has *something* to work with, even if the latest analysis hasn't landed yet.

### PostgreSQL + Apache AGE as the Unified Substrate

All of this data lives in PostgreSQL with Apache AGE for graph queries:
- **Event ledger**: append-only table with recursive CTE support for DAG traversal
- **Character tensors**: structured JSONB with genre-transform metadata, retrievable as single objects
- **Relational graph**: AGE graph with multi-scale edge properties (orbital, arc, scene layers)
- **Narrative graph**: AGE graph with gravitational mass and scene connectivity
- **Setting topology**: AGE graph with spatial adjacency and permeability
- **Flavor text**: TEXT columns with FK relationships to axis/variable records, pg_vector deferred for authoring-time semantic search

The Storykeeper's queries are SQL (event ledger, character tensors) and Cypher (graph traversals). Both are deterministic, bounded, and inspectable.

---

*This document bridges the Tier B narrative data generation (docs/foundation/data_driven_narrative_elicitation.md) and the Tier C engineering work. It assumes completion of the data generation pipeline and the axis inventory analysis (storyteller-data/narrative-data/analysis/2026-03-19-axis-inventory-and-type-design.md). The architectural decisions here will shape the storyteller-ml crate, the storyteller-storykeeper crate, and the context assembly pipeline in storyteller-engine.*

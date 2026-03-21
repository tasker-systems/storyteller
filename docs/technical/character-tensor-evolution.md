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

The prediction model's output (intentional orientation) must connect to the event extraction pipeline. Currently, event extraction uses DistilBERT for entity mentions and a small LLM for semantic role labeling. The enriched prediction model should inform:
- **What the character is likely to do** (intentional orientation → probable actions)
- **How the character interprets what happened** (orientation shapes perception of events)
- **What the character notices and ignores** (orientation as attentional filter)

This suggests refining the event extraction prompting to be character-perspective-aware — not just "what happened" but "what happened *as experienced by this character given their current orientation*."

---

## Open Questions

1. **PCA vs. learned embeddings for dimension reduction.** PCA is interpretable but assumes linearity. An autoencoder could capture nonlinear axis relationships (e.g., the way warmth and authority interact differently under different epistemological stances) but is less interpretable. Given the system's commitment to inspectability, PCA with residual analysis may be the right first pass.

2. **How many archetype influences per character?** The composition model allows any number, but practically, 1-3 primary archetypes with influence weights seems sufficient. More than 3 risks losing the character's distinctiveness into a blur of competing tendencies.

3. **Intentional orientation dimensionality.** What does the output vector look like? It needs enough dimensions to capture behavioral tendency, emotional state, relational stance, and communicative mode — but not so many that the narrator can't translate it into prose. The current system's intent statements (~200-400 tokens) are the right output *form*; the model needs to produce the right *input* to generate them.

4. **Genre transform as static or dynamic.** The description above treats genre as a fixed transform applied at character composition time. But some genre dimensions are state variables (emotional_contract can shift at midpoints in romance; narration_reliability degrades with sanity in cosmic horror). Should the genre transform be re-applied as state variables change?

5. **Training data volume.** How many synthetic character instances are needed? The combinatorial space is bounded by (30 genres × ~8 archetypes per genre × variance sampling × scene context variation). Rough estimate: 10K-50K training pairs might be sufficient for a model of moderate complexity, but this needs empirical validation.

6. **ONNX deployment.** The current inference pipeline uses ort (ONNX Runtime) for the character predictor. The new model's structured input (genre embedding + tensor composition + graph state + temporal position) needs to be ONNX-exportable, which constrains architecture choices (transformers export well; custom graph layers may not).

---

*This document bridges the Tier B narrative data generation (docs/foundation/data_driven_narrative_elicitation.md) and the Tier C engineering work. It assumes completion of the data generation pipeline and the axis inventory analysis (storyteller-data/narrative-data/analysis/2026-03-19-axis-inventory-and-type-design.md). The architectural decisions here will shape the storyteller-ml crate and the context assembly pipeline in storyteller-engine.*

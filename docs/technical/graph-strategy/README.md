# Graph Strategy

## Purpose

This directory documents the **mathematical foundations** for the storyteller system's four graph structures. Where [`knowledge-graph-domain-model.md`](../knowledge-graph-domain-model.md) defines *what* the Storykeeper reasons about, these documents define *how to compute it* — the algorithms, formulas, and data transformations that turn graph structure into narrative intelligence.

The documents here are **applied math**: intuitive explanations first, then formal notation, then Rust pseudocode using actual project types, then worked examples from "The Fair and the Dead" (TFATD). Each document is self-contained but builds on concepts from earlier documents.

### Relationship to Other Documentation

- **`knowledge-graph-domain-model.md`** — Defines the four graphs as domain concepts (vertices, edges, query patterns, cascade policies). This directory formalizes the math those operations require.
- **`gravitational-context-assembly.md`** — Defines how gravity drives context retrieval. `cross-graph-composition.md` here formalizes the composite scoring math.
- **`tales-within-tales.md`** — Defines sub-graph topology, permeability, prophetic cascades. `cross-graph-composition.md` and `traversal-friction.md` here formalize the cross-boundary math.
- **`ml_strategy/`** — Companion directory documenting the ML pipeline. Where ML strategy covers prediction and classification models, graph strategy covers the structural computations that feed those models.

---

## Documents

| Document | Description | Prerequisites |
|---|---|---|
| [relational-web-math.md](relational-web-math.md) | Substrate vectors, topological roles, centrality, clusters, triadic patterns, cascade propagation | None |
| [narrative-gravity.md](narrative-gravity.md) | Effective mass, multi-dimensional distance, gravitational pull, attractor basins, approach vectors | None |
| [setting-topology.md](setting-topology.md) | Traversal cost, weighted shortest path, conditional edges, bounded reachability | None |
| [event-dag.md](event-dag.md) | Typed DAG structure, topological sort, frontier maintenance, exclusion cascades, amplification | None |
| [traversal-friction.md](traversal-friction.md) | Unified friction model across all four graphs — attenuation, permeability, distortion, multi-path resolution | All four graph documents |
| [cross-graph-composition.md](cross-graph-composition.md) | Composite scoring, gravitational modifier, sub-graph mass, boundary permeability, prophetic cascades, budget allocation | All prior documents |

### Reading Order

```
relational-web-math ─┐
narrative-gravity ───┤
setting-topology ────┼──→ traversal-friction ──→ cross-graph-composition
event-dag ───────────┘
```

The first four documents can be read in any order — they cover independent graph structures. `traversal-friction` unifies signal propagation across all four. `cross-graph-composition` assumes familiarity with everything prior.

---

## Shared Notation

These conventions are used across all documents in this directory.

### Identifiers

| Symbol | Type | Description |
|---|---|---|
| `e`, `e_i` | `EntityId` | Entity (character, presence, condition, prop) |
| `s`, `s_i` | `SceneId` | Scene in the narrative graph |
| `σ`, `σ_i` | `SettingId` | Setting (location) in the setting topology |
| `n`, `n_i` | `EventConditionId` | Node in the event dependency DAG |
| `g`, `g_i` | `SubGraphId` | Narrative sub-graph (layer) |

### Graph Notation

| Symbol | Meaning |
|---|---|
| `V`, `E` | Vertex set, edge set |
| `(u, v)` | Directed edge from `u` to `v` |
| `w(u, v)` | Weight of edge `(u, v)` |
| `d(u, v)` | Distance from `u` to `v` (context-dependent metric) |
| `N(v)` | Neighborhood of `v` — all vertices adjacent to `v` |
| `N⁺(v)`, `N⁻(v)` | Out-neighborhood, in-neighborhood (directed graphs) |
| `deg(v)`, `deg⁺(v)`, `deg⁻(v)` | Degree, out-degree, in-degree |
| `π(u, v)` | A path from `u` to `v` |
| `\|π\|` | Length of path `π` (number of edges) |

### Domain-Specific Notation

| Symbol | Meaning | Range |
|---|---|---|
| `M(s)` | Effective mass of scene `s` | `[0.0, ∞)` |
| `D(s₁, s₂)` | Narrative distance between scenes | `(0.0, ∞)` |
| `G(s₁, s₂)` | Gravitational pull of `s₂` on `s₁` | `[0.0, ∞)` |
| `R(e₁, e₂)` | Substrate vector from `e₁` to `e₂` | `ℝ⁵` |
| `P(edge)` | Permeability of an edge | `[0.0, 1.0]` |
| `F` | Friction factor | `(0.0, 1.0)` |
| `Ψ(g)` | Boundary permeability of sub-graph `g` | `[0.0, 1.0]` |

### Conventions

- **Pseudocode** uses Rust syntax with actual project types (`EntityId`, `SceneId`, `RelationalSubstrate`, etc.) but omits error handling and ownership semantics for clarity.
- **Worked examples** use TFATD data inline: 6 characters (Sarah, Tom, Kate, John, Adam, Beth), 8 scenes (S1–S8), substrate values from `relational-web-tfatd.md`, scene masses from `narrative-graph-case-study-tfatd.md`.
- **Theoretical foundations** sections map our model to established graph theory, network science, and spatial analysis. We cite what we borrow and flag what we invent.

---

## TFATD Reference Data

These values appear throughout the worked examples. Source: [`relational-web-tfatd.md`](../relational-web-tfatd.md) and [`narrative-graph-case-study-tfatd.md`](../narrative-graph-case-study-tfatd.md).

### Characters

| ID | Name | Topological Role | Cluster |
|----|------|-----------------|---------|
| `e_sarah` | Sarah (12) | Traveler | Household |
| `e_tom` | Tom/Tommy (17) | Displaced Hub | Sacrifice |
| `e_kate` | Kate (mother) | Hidden Bridge | Household |
| `e_john` | John (father) | Mortal Periphery | Household |
| `e_adam` | Adam (the Gate) | Gate/Bridge | Gate |
| `e_beth` | Beth (grief) | Affective Periphery | Sacrifice |

### Scenes (Part I)

| ID | Title | Type | Authored Base Mass | Computed Mass |
|----|-------|------|--------------------|---------------|
| S1 | Tom Lies Still and Dying | Gravitational | 0.8 | 0.9 |
| S2 | Speaking with the Gate | Gate/Threshold | 0.7 | 0.9 |
| S3 | A Mother's Prayer | Gate/Gravitational | 0.85 | 0.95 |
| S4 | A Haunt on the Rise | Connective/Gate | 0.5 | 0.65 |
| S5 | Crossing a Stream | Connective | 0.3 | 0.3 |
| S6 | The Other Bank | Gravitational | 0.9 | 1.0 |
| S7 | The Abandoned Village | Gate | (est.) 0.4 | 0.5 |
| S8 | Meeting the Witch | Gravitational | (est.) 0.85 | 0.95 |

---

## Current Status

| Document | Status |
|----------|--------|
| `README.md` | Complete |
| `relational-web-math.md` | Complete |
| `narrative-gravity.md` | Complete |
| `setting-topology.md` | Complete |
| `event-dag.md` | Complete |
| `traversal-friction.md` | Complete |
| `cross-graph-composition.md` | Complete |

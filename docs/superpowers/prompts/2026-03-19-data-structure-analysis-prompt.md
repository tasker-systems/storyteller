# Data Structure Analysis: Genre and Archetype Axes

## Your Task

Analyze the generated genre region data and archetype extraction data to identify:
1. **System-consistent axes** that appear across genres and archetypes (the universal dimensional language)
2. **Cluster-specific axes** that only make sense within a genre family (horror, fantasy, sci-fi, etc.)
3. **Genre-specific axes** that are unique to a single genre or small set of genres
4. **A proposed polymorphic axis type** that allows genre/archetype-specific expressiveness without type bloat

## Data to Analyze

All data lives under `narrative-data/` in this repository:

### Genre Regions (30 files)
`narrative-data/genres/{slug}/region.raw.md` — each ~10-15KB, rich dimensional descriptions with axes like aesthetic, tonal, thematic, structural dimensions, world affordances, locus of power, temporal orientation, epistemological stance, state variables, exclusions, genre topology.

### Archetype Extractions (30 files)
`narrative-data/discovery/archetypes/{genre-slug}.raw.md` — each ~10-14KB, 5-8 archetypes per genre with personality axes, distinguishing tensions, overlap signals. These archetypes were discovered FROM the genre data, not seeded from convention.

### Cluster Syntheses (6 files)
`narrative-data/discovery/archetypes/cluster-{name}.raw.md` — each ~13-18KB, ~8-12 archetypes per cluster with canonical names, core identity, genre variants, uniqueness assessments. Clusters: horror, fantasy, sci-fi, mystery-thriller, romance, realism-gothic-other.

## Analysis Framework

### Part 1: Axis Inventory

Read through ALL 30 genre regions and ALL 30 archetype extractions. Catalog every distinct axis, dimension, or variable that appears. For each:
- **Name** (as used in the data — there may be multiple names for the same concept)
- **Type** (continuous scale, enum/categorical, boolean, composite)
- **Scope** (universal, cluster-specific, genre-specific)
- **Where it appears** (which genres/clusters reference it)
- **Example values** from the data

Pay special attention to:
- Axes that appear under different names in different genres (these need canonical naming)
- Axes that are continuous in some genres but categorical in others
- State variables (things that change over the course of a story) vs. static dimensions
- Axes that only make sense when another axis is present (conditional dimensions)

### Part 2: Structural Patterns

Identify recurring structural patterns in how axes are organized:
- Which axes always appear together (co-occurring clusters)?
- Which axes are independent?
- Are there axes that modify other axes (meta-dimensions)?
- Are there axes that only activate in certain genre contexts?

### Part 3: Type Design Proposal

Propose a data modeling approach that handles:
- **Core axes** (~8-12) present on every genre and archetype — these get first-class fields
- **Scoped axes** that exist within genre clusters or specific genres — these need a flexible container
- **State variables** that track change over narrative time — different from static positioning
- **The archetype-in-genre problem**: an archetype has base axes, but those axes SHIFT when placed in a genre context. The shift itself is data.

Think about:
- A polymorphic axis type (e.g., `Axis { name, value, scope, context?, ... }`) vs. typed axis families
- How to avoid a giant flat bag of optional fields while still supporting genre-specific expressiveness
- How the narrator would query this at runtime: "give me the warmth axis for The Keeper in folk-horror"
- Whether axes should be typed (warmth is always a float) or flexible (some axes are enums, some are scales, some are text descriptions)

## Output

Write your analysis to `narrative-data/analysis/2026-03-19-axis-inventory-and-type-design.md`. Structure it as:

1. **Axis Inventory Table** — every axis found, with scope, type, and occurrence data
2. **Axis Clusters** — groups of co-occurring axes
3. **Conditional Axes** — axes that only activate in specific contexts
4. **State Variables vs. Static Dimensions** — the temporal distinction
5. **Type Design Proposal** — concrete Rust/Python type sketches with rationale
6. **Open Questions** — things you noticed that need human judgment

Be thorough — read every file. This analysis drives the data modeling for the entire narrative engine.

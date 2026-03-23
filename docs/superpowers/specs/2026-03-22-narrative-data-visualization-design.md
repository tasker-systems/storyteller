# Narrative Data Visualization Site — Design Spec

## Purpose

A static visualization site for exploring and presenting the storyteller narrative data corpus. Three concerns, three tabs: **Home** tells the story of the project, **Explore** lets you navigate the data terrain interactively, **Analyze** provides comparative and statistical views. The site serves both as a presentation artifact (showing others what this work produced) and as an analytical tool (understanding the data's structure and relationships).

## Architecture

### Site Structure

```
Home        — Editorial narrative (existing index.html, evolved over time)
Explore     — Interactive D3.js data exploration, single scrolling page
Analyze     — Charts, distributions, cross-genre comparisons (deferred — needs more data)
```

Navigation is a minimal top bar: `Home | Explore | Analyze`. Shared design language across all three (Cormorant Garamond + JetBrains Mono, parchment palette, rust accents).

### Technical Approach

Static files, no build step. D3.js via CDN (ES module imports). Plain HTML/CSS/JS. Served via `python -m http.server` or any static server.

All structured JSON loaded at page init for instant interactions. Total data size is ~5MB structured JSON — acceptable for a standalone exploration app where fast interaction matters more than time-to-interactive.

If the site grows beyond what static files can handle cleanly, migrate to a static site generator (11ty or similar). That's a future decision, not a current one.

### File Layout

```
viz/
├── index.html              # Home — editorial narrative
├── explore.html            # Explore — interactive D3 page
├── analyze.html            # Analyze — statistical views (stub for now)
├── css/
│   └── style.css           # Shared styles (extracted from current index.html)
├── js/
│   ├── data-loader.js      # Load and index all JSON at init
│   ├── genre-landscape.js  # Force-directed genre proximity graph
│   ├── genre-detail.js     # Hub-and-spoke deep dive + radar chart
│   ├── family-network.js   # Cross-genre archetype/entity families
│   └── shared.js           # Color scales, tooltips, transitions
├── build-data.sh           # Copies structured JSON from $STORYTELLER_DATA_PATH into data/
└── data/                   # .gitignored — populated by build-data.sh
    ├── genres/             # region.json, tropes.json, narrative-shapes.json per genre
    └── discovery/          # archetypes/, settings/, etc. — aggregated JSON per genre + cluster
```

### Data Dependencies

The site reads structured JSON produced by the narrative-data extraction pipeline:

- **P1 (complete):** `genres/{genre}/region.json` — 30 genre dimension profiles with 18 continuous axes across 7 dimension groups (aesthetic, tonal, temporal, thematic, agency, epistemological, world_affordances), plus list-typed fields (locus_of_power, narrative_structure, narrative_contracts, active_state_variables, boundaries)
- **P2 (in progress):** `discovery/{type}/{genre}.json` — archetypes, settings, ontological-posture, profiles per genre (aggregated JSON, one file per genre per type). Cluster synthesis files are extracted as `discovery/{type}/cluster-{name}.json`.
- **P2 genre-native:** `genres/{genre}/tropes.json`, `genres/{genre}/narrative-shapes.json` — per-genre, no clusters. These are Stage 2 extraction outputs; the `.json` files are produced by the same pipeline as discovery types and may not exist for all genres until P2 completes.
- **P3/P4 (future):** dynamics, goals, archetype-dynamics, spatial-topology, place-entities

The Explore page can launch with P1 + P2 data (genre landscape + archetype/settings families). Analyze needs P3/P4 for cross-genre comparison depth.

**Cluster membership:** The 6 clusters (horror, fantasy, sci-fi, mystery-thriller, romance, realism-gothic-other) and their genre assignments are defined in the narrative-data pipeline config (`GENRE_CLUSTERS`). The data loader should include a hardcoded lookup table mapping each genre slug to its cluster. This is stable configuration, not dynamic data.

**Incomplete coverage:** Not all 30 genres will have discovery data for every type (P2 is still running). Genres that have region.json but lack discovery data should appear in the landscape graph but show an "extraction pending" state in the deep dive rather than empty spokes.

## Home Tab

The existing `viz/index.html` — a hand-authored editorial page with curated examples, blockquotes, and prose explaining the project's approach and emergent structures. No D3. Updated over time as new insights emerge. Stats in the hero section refreshed as the corpus grows.

No structural changes needed. Extract shared CSS into `css/style.css` and add the nav bar.

## Explore Tab

A single scrolling page with three interactive D3 sections. The sections are visually connected — the same graph metaphor (nodes and edges) repeats at different scales.

### Section 1: Genre Landscape

A force-directed network graph of 30 genres, clustered by region proximity. Nodes colored by cluster (6 clusters: horror, fantasy, sci-fi, mystery-thriller, romance, realism-gothic-other).

**Edge weights:** Cosine similarity across the 18 continuous axes in region.json (all normalized 0.0-1.0). Axis values are nested within dimension group objects (e.g., `aesthetic.sensory_density.value`, `tonal.dread_wonder.value`) — the data loader extracts these into a flat vector per genre for similarity computation. Draw edges between genres with similarity above a threshold (start with 0.7, tune visually). Edge opacity or thickness proportional to similarity strength. This produces natural clustering without forcing it — genres that share dimensional profiles end up close; the force layout does the rest.

**Interactions:**
- **Hover:** Highlight node and its connections, show genre name tooltip
- **Click:** Triggers descent into genre deep dive (see Graph Descent below)

**Copy block** above the graph: brief editorial text about genre-as-region, not category.

### Section 2: Genre Deep Dive (drill-down from Section 1)

Hub-and-spoke layout for one genre. Central node = genre, spokes = primitive types that have data (archetypes, settings, tropes, narrative-shapes, etc.). Click a spoke to fan out its entities as secondary nodes.

**Detail views per primitive type:**
- **Archetypes:** Personality profile bars (7 axes), state variable associations, distinguishing tension
- **Settings:** Communicability dimensions if available
- **State variables:** Gauge charts showing initial_value → threshold
- **Tropes/shapes:** Name + flavor text in cards

**Radar chart** alongside the hub-and-spoke showing the genre's dimensional profile across the 7 dimension groups. Each group contributes its axes as radar spokes (18 total spokes). Values are already normalized 0.0-1.0 so the radar is directly readable.

Color sets per primitive type, drawn from the existing palette (rust, deep-blue, sage, warm-gray, accent-horror through accent-realism).

### Section 3: Cross-Genre Families

Same network-graph pattern as Section 1, but organized by canonical entity family across clusters. For archetypes: "The Keeper/Warden" as center node, genre-specific variants radiating outward with cluster coloring.

Repeats for each primitive type that has cluster-level synthesis data (archetypes, settings, ontological posture, profiles). Types without clusters (tropes, narrative-shapes) are omitted from this section.

### Graph Descent Interaction

The defining interaction of the Explore page. When a user clicks a genre node in the landscape:

1. **Scatter:** Other genre nodes animate off-screen (fade + drift outward)
2. **Center:** Clicked node animates to center-screen, grows slightly
3. **Morph:** Node expands — the page scrolls smoothly to anchor a new section below where the hub-and-spoke and radar chart render
4. **Back:** A breadcrumb/header appears ("← Back to Landscape" + genre name). The primary back mechanism is clicking this breadcrumb. As a secondary affordance, scrolling back up past the detail section's top edge triggers the collapse — but the click target is the reliable path. This avoids scroll-fight edge cases while still feeling natural for users who instinctively scroll up.

The same pattern applies when clicking into a spoke (e.g., archetypes) — the spoke entities fan out, and clicking the spoke breadcrumb or scrolling back collapses them.

**Implementation:** D3 transitions for the node animations (`.transition().duration(600)`). Breadcrumb click handler manages the collapse and reassembly. `IntersectionObserver` on the detail section provides the scroll-triggered collapse as a progressive enhancement — if the interaction proves unreliable during implementation, the breadcrumb alone is sufficient. The graph and detail view share one container — no page navigation, just DOM state and scroll position.

## Analyze Tab

Deferred pending more structured data (P3/P4) and real usage of the Explore tab to inform what comparisons matter most.

**Planned panels** (structure only, not specced for implementation):
- **State Variable Atlas** — the renaming problem made interactive. Canonical variables mapped to genre-specific expressions, with initial/threshold gauges
- **Axis Explorer** — pick a dimension, see 30 genres distributed along it (strip plots or parallel coordinates)
- **Cross-Genre Comparison** — select 2-3 genres, overlay radar charts and compare entity profiles
- **Corpus Statistics** — coverage heatmaps, completeness dashboards, extraction quality metrics

## Design Language

Inherited from the existing `index.html`:

- **Typography:** Cormorant Garamond (body, headings), JetBrains Mono (data labels, stats, code)
- **Palette:** parchment (#f4f0e8), ink (#1a1714), rust (#8b4513), deep-blue (#2c3e50), sage (#5a6e5a)
- **Cluster colors:** horror (#6b2737), fantasy (#3d5a80), sci-fi (#4a7c59), mystery (#5c4a72), romance (#8b5e6b), realism (#6b6040)
- **Texture:** Parchment noise overlay (SVG fractal noise at 3% opacity)
- **Cards:** White background, 1px parchment-dark border, subtle hover lift
- **Transitions:** 200ms for hover states, 600ms for graph animations, ease-in-out

The nav bar should feel like part of the page, not a chrome element — same parchment background, Cormorant Garamond, minimal weight.

## Non-Goals

- No framework (React, Svelte, etc.) — vanilla JS + D3
- No build step — CDN imports, static files
- No server-side rendering or API
- No authentication or user state
- No mobile-first design (desktop exploration tool, responsive is nice-to-have)
- Analyze tab is not specced for implementation — structure only

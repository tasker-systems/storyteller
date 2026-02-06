# Storybook Content

Source material for the storytelling system — creative works that serve different roles in development.

## Content Strategy

Not all content here serves the same purpose:

**Analytical references** — creative works studied to validate and calibrate the system's modeling capabilities. We analyze these to understand what our tensor, narrative graph, and relational web models must be capable of capturing. We do not build playable system content from them directly.

**Workshop material** — content created for or adapted to the system, where iteration, rough handling, and experimentation are expected. This is where we build, break, and learn.

The distinction matters. Analytical references carry creative investment that deserves careful treatment. Workshop material is deliberately chosen to be safe for experimentation.

## Contents

### `extracted/` — Analytical References

Content extracted from existing creative works using the `doc-tools` package.

#### `extracted/the-fair-and-the-dead/`

A dark fantasy narrative about Sarah (12), who journeys into the Shadowed Wood to retrieve her dying brother's lost spirit. Part I ("To the Witch") contains 6 manuscript scenes, 6 character files, and 5 notes files.

**Extracted from**: A Scrivener project using `extract-scrivener`

**Structure**:
- `characters/characters/` — 6 character sheets (Sarah, Tom, Kate, John, Adam, Beth)
- `manuscript/to-the-witch/` — Prose scenes across two sections:
  - `in-the-fever-bed/` — Exposition and initiation (2 scenes)
  - `sarah-and-the-wolf/` — Journey into the Wood (4 scenes)
- `notes/notes/` — Conflicts, plot notes, a guide (the Wolf), charms/prayers, refugees
- `front-matter/` — Front matter content

**Role in development**: Primary analytical reference for character tensor modeling (Sarah, the Wolf), narrative graph structure, and relational web design. The technical case studies in `docs/technical/` are built from this material.

#### `extracted/vretil/`

A literary quest narrative across 20 chapters, mixing third-person prose with epistolary passages. A boy searches for his sister through a world where prophetic elements, embedded mystery, and primary plot interweave toward a mythopoetic ending.

**Extracted from**: A DOCX manuscript using `convert-docx`

**Role in development**: Secondary analytical reference. Demonstrates sophisticated narrative architecture (nested timelines, unreliable narration, slow reveal) useful for validating the narrative graph model at novel scale.

### `bramblehoof/` — Workshop Material

A satyr bard/warlock from a D&D campaign — Bramblehoof, a wandering musician who discovers a corruption spreading through the world (poisoned ley lines, a death cult, crushed creativity) and forms a pact with Whisperthorn (an archfey) to fight back.

**Structure**:
- `spec.md` — Character prologue and world specification

**Role in development**: Primary workshop material. This is where we build characters, map the world, experiment with narrative graphs, test agent personas, and iterate freely. The world has coherent material-spiritual conditions but is deliberately open for expansion and experimentation.

## The doc-tools Package

The `doc-tools/` directory at the repo root contains the Python package used to extract content into this directory. See the repo root README or `doc-tools/pyproject.toml` for usage.

# Foundation Documents

Design philosophy, system architecture, and the philosophical commitments that shape everything downstream. These documents are the root of the project — they precede and inform all technical specification and implementation.

They are written as essays, not specifications. The voice is deliberate: these are not just requirements documents but arguments for a particular way of thinking about interactive narrative. Implementation decisions should be traceable back to principles articulated here.

## Reading Order

The documents build on each other. For someone encountering the project for the first time:

### 1. [Design Philosophy](design_philosophy.md)

The seven core principles that define the system. Start here.

- **Narrative Gravity** — stories bend toward pivotal moments; scenes have mass
- **N-Dimensional Narrative Space** — position, emotional valence, information state, relational state, thematic resonance
- **Imperfect Information** — no agent has complete knowledge; richness emerges from partial perspectives
- **Character as Tensor** — multidimensional representation, not stat sheets
- **Three-Tier Constraints** — hard (world physics), soft (character capacity), perceptual (what can be sensed)
- **Mindedness and Respect** — agents are treated as minded beings, not functions
- **Emergence as Feature** — the system is designed for surprising, unplanned moments

### 2. [System Architecture](system_architecture.md)

The agents, their roles, their knowledge boundaries, and how information flows between them.

- The Narrator, Storykeeper, Character Agents, Reconciler, World Agent, Player
- Information flow diagrams
- The constraint framework in detail
- Why custom orchestration, not third-party agent frameworks

### 3. [Character Modeling](character_modeling.md)

How characters are represented — the tensor model, relational webs, and temporal dynamics.

- Personality axes with central tendencies, variance, and contextual triggers
- Motivational structure: surface / deep / shadow wants
- Values, beliefs, capacities
- The geological temporal model: topsoil (volatile), sediment (sustained), bedrock (foundational)
- The echo phenomenon and off-screen propagation
- Context-dependent activation

### 4. [Narrative Graph](narrative_graph.md)

How stories are structured — the gravitational model that replaces branching trees.

- Scenes as gravitational bodies with mass, approach vectors, departure trajectories
- Connective space between high-mass scenes
- Attractor basins, convergence points, hard/soft/delayed branches
- Scene design as landscape architecture

### 5. [World Design](world_design.md)

How worlds are built — the coherence principle, authorial ingress points, and the communicability of things.

- The mutual production of material conditions and social forms
- Five authorial ingress points: relational web, narrative graph, single scene, aesthetic expression, material condition
- Guideposts for simplification when modeling choices must be made
- The relationship between world coherence and character/narrative
- The communicability gradient: surface area, translation friction, timescale, capacity to turn toward
- Entity representation spectrum: characters, presences, conditions, props
- The World Agent as translator for the non-character world

### 6. [Project Organization](project_organization.md)

How the work is structured — eleven streams across three phases.

- Stream definitions and dependencies
- Phase 1 (Foundation and Architecture), Phase 2 (Core Systems), Phase 3 (Integration, Tooling, Content)
- The principle: build the smallest thing that teaches you something

### 7. [Open Questions](open_questions.md)

What we do not yet know — eleven major unresolved design challenges.

Covers: good faith, onboarding, authorial quality, persistent world, action granularity, unwind mechanics, emergence, model capability, story design tooling, multiplayer, and safety.

## Original Conversations

The `original-conversation/` directory contains transcripts of the design sessions that produced these documents. They provide historical context for architectural decisions and include exploratory thinking that didn't make it into the formal documents. See `transcript-journal.txt` for an index.

## Living Documents

These documents are not finished. They evolve as implementation surfaces questions the philosophy must address, and as the philosophy discovers implications the implementation must accommodate. The relationship is reciprocal — the documents guide the code, and the code tests the documents.

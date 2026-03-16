# Scenes, Chapters, and Stories

> This document begins with pragmatic near-term concerns discovered during Phase 3 workshop conversion and recent playtesting, then expands into broader architectural thinking about multi-scene narrative structure.

---

## Part I — Pragmatic Preamble

### 1. GenreOptions: Revert to Sequential Descriptor Calls

**Problem**: Phase 3 introduced a `GetGenreOptions` combined RPC that returns all wizard options in a single round-trip. This was expedient but wrong — the original workshop Tauri code made sequential calls because genre/archetype/profile/dynamic selections have **intersection and fallback logic** between steps. A "dark fantasy" genre constrains which archetypes are available; an archetype constrains which dynamics make sense. The combined struct flattens this into incommensurate option lists with no dependency awareness.

**What we lost**: The old wizard code in `commands.rs` (pre-thin-client conversion) handled per-step filtering — selecting a genre narrowed archetypes, selecting an archetype narrowed profiles, etc. When we collapsed this into `GetGenreOptions`, that conditional logic disappeared.

**Next step**: Replace `GetGenreOptions` with per-step RPCs that accept prior selections as input:
- `GetGenres()` — no input needed
- `GetArchetypes(genre)` — filtered by selected genre
- `GetProfiles(genre, archetype)` — filtered by prior selections
- `GetDynamics(genre, archetype, profile)` — filtered by all prior

Each RPC returns only the valid options given the current wizard state. The intersection and fallback logic lives server-side where the descriptor data is available, keeping the workshop a thin client.

### 2. `workshop:gameplay` Streaming Channel

**Problem**: Scene rendering currently collects the full narrator LLM response and returns it as a single gRPC response. The player sees nothing until the narrator finishes — potentially 10-30 seconds of silence.

**Agreed architecture**: A `workshop:gameplay` event channel (alongside existing `workshop:debug` and `workshop:logs` channels) that streams primary gameplay events to the frontend. This is the channel the player-facing UI renders from.

**Key events on this channel**:
- `NarratorTokens` — streaming prose tokens as they arrive from the LLM
- `NarratorComplete` — full rendered prose (for journal/history reconstruction)
- `SceneOpening` — initial scene-setting narration (see §4 below)
- `IntentProposal` — character intentions for player review (see §5 below)

**Rationale**: Beyond UX responsiveness, this channel enables the media pipeline (§3) — we need the narrator prose as a complete buffer to send to image generation, but we also need to stream it token-by-token to the player.

### 3. `workshop:media` Stream — Graphic Novel Panels

**Vision**: When the narrator completes a prose render, capture the full text along with scene setting and aesthetic metadata, then send it to a text-to-image model (FLUX2 Klein, 4B or 9B) to generate visual panels that layer alongside the primary text pane.

**Pipeline**:
1. Narrator prose streams to player via `workshop:gameplay` (tokens)
2. Simultaneously, prose accumulates in a server-side buffer
3. On `NarratorComplete`, the full prose + setting description + aesthetic directives are sent to the image generation model
4. Generated image(s) stream back via `workshop:media` channel
5. Frontend layers panels alongside the text — graphic novel style, charcoal illustrations, grainy photos, etc., depending on genre and player/author aesthetic choices

**Aesthetic direction is not hardcoded** — it's part of the scene/genre metadata. A folk horror story gets charcoal sketches; a cyberpunk story gets neon-lit panels; a literary fiction piece might get impressionist watercolors or nothing at all.

**Model considerations**: FLUX2 Klein runs locally (like Ollama for LLM). The 4B model is faster but lower fidelity; 9B is slower but more detailed. This is a post-scene-rendering pipeline — latency is less critical because the player is already reading prose while images generate.

### 4. Missing Turn 0: Opening Narration

**Problem**: Playtest session `019ceee1-4602-7690-a69f-436c65106d9e` shows `turns.jsonl` starting at turn 1 with player input. There is no turn 0 — no opening narration that sets the scene before the player acts.

**Evidence**: The events.jsonl confirms this — the first event is a decomposition (skipped) tied to the player's first input. The player-as-LLM went first, which masks the issue, but a human player would see a blank scene with an input prompt and no context.

**Expected behavior**: When a scene begins (either fresh composition or session resume), the narrator should render an opening passage — the "curtain rises" moment that establishes setting, mood, and the character's immediate situation. This becomes turn 0 in `turns.jsonl` with `player_input: null`.

**This is a prerequisite for the `workshop:gameplay` channel** — `SceneOpening` would be the first event on that channel for any new scene.

### 5. Player Intent Review

**Problem**: Scene composition generates rich per-character goals (visible in `composition.json` — e.g., Draža has `maintain_deception` [Hidden], `investigate_anomaly` [Signaled], `test_loyalty` [Hidden], `secure_resource` [Signaled]). These drive NPC character agent behavior. But the player character's goals are generated the same way and **never shown to the player**.

**What we're missing**: After scene composition and before the opening narration, the player should see their character's generated intentions and have the opportunity to:
1. **Review** — understand what narrative threads the system has identified for their character
2. **Modify** — adjust, replace, or add intentions before the scene begins
3. **Accept** — proceed with the generated intentions as-is

**Design principle**: The player is the only non-AI agent. Their character's tensor representation is maintained by the system, but their decisions are human. Presenting generated intentions as suggestions rather than directives honors this boundary — the system says "here's what your character might want" and the player says "yes, and..." or "no, instead..."

**UX sketch**: After the wizard completes scene composition but before the narrator renders the opening, an "Intent Review" step shows the player character's signaled goals as editable cards. Hidden goals (if any exist for the player character) are surfaced too — the player shouldn't have hidden goals from themselves.

**Impact on `workshop:gameplay` channel**: The event sequence becomes:
1. `SceneComposed` — scene is ready
2. `IntentProposal` — player character's generated intentions
3. Player reviews/modifies/accepts
4. `IntentConfirmed` — final player intentions
5. `SceneOpening` — narrator renders the opening with confirmed intentions as context

---

## Part II — The Dramaturge

### The Problem: Locally Coherent, Globally Stagnant

The system currently produces **locally coherent prose** — each turn is well-rendered, character dynamics are situated and distinct, ML predictions and intent synthesis give NPCs genuine agency. Playtesting confirms this: two characters with different tensors, different goals, different communicability profiles produce recognizably different voices and behaviors in interaction.

But after a few turns, the scene stalls. The characters orbit each other without progressing. Dialogue echoes. The dynamic shifts subtly but accumulates toward nothing. This is not a bug — it's the natural consequence of an architecture that renders moments without directing them.

**The root cause is a missing layer.** We have:

- **Turn-level mechanics** that work — ML predictions, intent synthesis, narrator rendering, character dynamics produce locally coherent prose
- **Scene-level metadata** that exists — goals, constraints, emotional arc targets, cast, scene type — but sits inert in `composition.json`
- **Narrative theory** that describes the desired macro behavior — attractor basins, gravitational landscapes, narrative mass

Nothing at runtime translates scene-level intent into turn-level directives. The narrator receives a snapshot (preamble + journal + retrieved context) but no **vector**. It knows where things *are* but not where they're *going*.

### Why the Narrator Can't Solve This Alone

The narrator is correctly scoped. It renders *this moment* given *this context* — that's its job, and it does it well. But "render this moment" without dramatic direction produces tableaux, not narrative. Consider:

> "Arthur does not want Margaret to know about the letter."

This is a static constraint. Arthur honors it by... never mentioning the letter. Forever. What makes fiction work is **pressure against the constraint** — Margaret is getting closer to finding out, a drawer was left open, a third party is about to reveal it. The *constraint* creates tension only when paired with a *force* that pushes against it.

The narrator has no way to know:
- That it's been rendering variations on the same dynamic for 4 turns
- That the scene is supposed to be building toward a revelation
- That "now" is the moment environmental pressure should increase
- That the player has been circling the secret without engaging it

This is not information the narrator should hold. It's a different kind of knowledge entirely.

### The Dramaturge Agent

In theater, the dramaturge doesn't direct, doesn't act, doesn't write the script. They hold the *meaning* of the piece and whisper to the director: "this scene is about loss, not anger" or "the audience needs to feel safe before you pull the rug out." They are the keeper of dramatic intent.

The dramaturge is a new agent that holds the **dramatic arc** of a scene and emits per-turn shaping signals. It occupies a unique knowledge position:

| Agent | Knows |
|-------|-------|
| **Storykeeper** | What happened. What's true. (narrative state, information ledger) |
| **Character agents** | What they want. (intentions, tensor-driven motivation) |
| **Reconciler** | Who goes when. (coordination, conflict resolution) |
| **Narrator** | How to say it. (voice, rendering, prose craft) |
| **Dramaturge** | **Why this moment matters.** Where it sits in the arc. What dramatic work it should accomplish. What forces should be tightening or releasing. |

### Architectural Placement: Async MPSC Channel Actor

The dramaturge operates **asynchronously** on a bounded MPSC channel. It does not block the turn cycle.

```
Turn N completes
    │
    ├──→ [turn data + event IDs] ──→ Dramaturge channel (async)
    │                                     │
    │                                     ▼
    │                              Pre-query Storykeeper
    │                              (scene entities, nearby gravity,
    │                               relational state, plot currents)
    │                                     │
    │                                     ▼
    │                              Dramaturge LLM deliberation
    │                                     │
    │                                     ▼
    │                              Write DramaticDirective
    │                              (tagged with applicable turn range)
    │                                     │
    ▼                                     ▼
Turn N+1 begins              Context assembly reads
    │                         latest DramaticDirective
    ├──→ [preamble + journal + retrieved + directive] ──→ Narrator
```

**Key properties:**

- **Non-blocking**: The turn cycle never waits for the dramaturge. If it hasn't finished processing (e.g., turns 1-2 of a scene), the narrator operates without dramatic directives — which is fine, because early turns are establishment.
- **One turn behind**: The dramaturge evaluates what just happened and shapes what comes next. This is correct — it reflects on the past and influences the future, rather than trying to intervene in the present.
- **Mutual ID pointers**: The dramaturge receives turn IDs and event IDs. Its output references these ("based on turns 1-3, guidance for turn 4+"). Context assembly checks for the most recent applicable directive.
- **Notes buffer**: The dramaturge writes to a known location (resource, table, or shared buffer). Context assembly reads from it. No direct coupling between the two.

### Pre-Invoked Context

Before the dramaturge's LLM call, its context is assembled from pre-queried Storykeeper data:

- **Scene metadata** — goals, constraints, cast, emotional arc targets, scene type and its expected shape
- **Confirmed player intentions** — what the player chose during intent review (§I.5)
- **Nearby narrative gravity** — plot events pulling on this scene from the broader story graph; pending revelations; unresolved tensions from prior scenes
- **Entity relationship state** — current relational web between cast members
- **Turn journal** — what has actually happened in the scene so far

This is a rich but **bounded** context. The dramaturge doesn't need the full narrative ledger — it needs the *relevant span*, pre-queried. This keeps its token budget manageable and its deliberation focused.

### Dramatic Directive Structure

The dramaturge's output must be **data-actionable** — not free-form prose advice, but structured signals that context assembly can reason about.

```
DramaticDirective {
    // Positioning
    arc_position: ArcPosition,        // rising_tension | climax_approach | breath | denouement
    applicable_from: TurnId,          // "act on this starting from turn N"
    supersedes: Option<DirectiveId>,  // replaces prior directive if present

    // Active forces — narrative pressures to surface
    active_forces: Vec<NarrativeForce>,
    // e.g., "the letter is about to fall from the coat pocket"
    // e.g., "Margaret just asked an uncomfortably direct question"
    // These get woven into the narrator's retrieved context.

    // Turn-level guidance — compact note for the narrator
    turn_directive: String,
    // e.g., "the secret should feel closer to the surface this turn"
    // e.g., "give the characters room to breathe before the next escalation"

    // Diagnostic signals
    staleness: Option<StalenessFlag>,
    // e.g., "dynamic between X and Y static for 3 turns — introduce movement"

    // Provenance
    based_on_turns: Vec<TurnId>,
    based_on_events: Vec<EventId>,
}
```

Context assembly folds the directive into the narrator's context — the `turn_directive` as a natural-language note in the preamble, the `active_forces` as retrieved context alongside character intents, the `arc_position` as a signal for how much dramatic pressure to encode.

### The Tai-Chi of Narrative Rhythm

Within each turn, within each scene, within each chapter — expansion and contraction, tension and relaxation, rising and falling. Space created and then filled. Space filled and then opened.

This is what the dramaturge tracks. Not plot mechanics, but **dramatic dynamics**:

- A scene of type "revelation" has a known shape — establishment, false comfort, cracks appearing, the reveal, aftermath. The dramaturge knows this shape because the scene composer selected it.
- Where we *are* in that shape depends on what the player has actually done. The dramaturge reads the accumulating journal and maps player actions against the expected arc.
- If the player is advancing toward the reveal faster than expected, the dramaturge adjusts — perhaps inserting a "breath" moment to let tension build rather than rushing the climax.
- If the player is circling without engaging, the dramaturge increases narrative pull — environmental pressure, NPC behavior shifts, the world conspiring to make avoidance harder.

The **shape** is known; the **content** is improvised. The dramaturge holds the shape and nudges the improv toward it.

### Fractal Application

The same expansion-contraction pattern operates at every scale:

| Scale | Unit | Dramaturge role |
|-------|------|-----------------|
| **Turn** | A single player input → narrator render | Micro-pacing: this moment's dramatic work |
| **Scene** | A bounded dramatic situation with cast, setting, stakes | Arc tracking: where are we in this scene's shape? |
| **Chapter** | A sequence of scenes that resolve or introduce tension | Macro-pacing: does this chapter end on release or cliffhanger? |
| **Story** | The full narrative arc | Thematic coherence: are we serving the story's meaning? |

The turn level is where the player *feels* it — that's where to solve it first. If the turn-by-turn has directionality, scene-level and chapter-level pacing can emerge from accumulation. If it doesn't, no amount of higher-level structure will save the experience.

Scene-level and chapter-level dramaturgy can be the same agent with broader context, or a hierarchy of agents. But the mechanism is the same: async evaluation, structured directives, context assembly integration.

---

## Part III — Genre as Hyperplane, Not Taxonomy

### The Limits of Flat Descriptors

The current descriptor system in `storyteller-data/training-data/descriptors/` includes genres, archetypes, profiles, dynamics, settings, goals, names, cross-dimensions, and an axis vocabulary. This was intentionally naive — enough structure to compose scenes and get to a first playable. But it models genre as a **discrete selection**: pick one genre, pick one archetype, proceed.

Genre doesn't work this way. Consider:

- **Romantasy** blends romance and fantasy in ways that neither genre alone describes — it has its own pacing expectations, its own tropes, its own audience contract
- **Nordic noir** shares structural DNA with hardboiled detective fiction but diverges sharply in tone, setting, moral framework, and resolution expectations
- **Cozy horror** and **eldritch horror** and **body horror** draw on common ancestral themes but have radically different affordances — what the world permits, what the audience expects to feel, what counts as a satisfying resolution
- A **murder mystery** can be the backbone of stories as different as *Twin Peaks* and *Father Brown*, with *Knives Out* occupying yet another distinct point in the space
- **Terry Pratchett** writes fantasy that is simultaneously mystery, fairy tale, social commentary, and philosophical reflection — and is tonally unlike any of those genres' "standard" expressions

The line between literary fiction and magical realism is itself instructive — is that line aesthetic? Cultural? Class-inflected? The fuzziness is the point. Genre boundaries are social conventions with gravitational centers, not hard taxonomic partitions.

### Genre as Weighted Position in Narrative Space

What we need is not a longer list of genre enums but a **multidimensional space** where genres are named regions — recognizable, useful for navigation, but not mutually exclusive. A story's genre identity is a weighted position across several dimensions:

**Aesthetic dimensions** — visual and sensory vocabulary, prose register, descriptive density
- Gritty realism ←→ Lyrical beauty
- Spare/minimalist ←→ Lush/ornate
- Grounded/mundane ←→ Heightened/mythic

**Tonal dimensions** — emotional contract with the reader/player
- Dread ←→ Wonder
- Cynicism ←→ Earnestness
- Irony ←→ Sincerity
- Intimacy ←→ Epic distance

**Thematic dimensions** — what the story is *about* beneath its surface
- Power and its corruption
- Identity and belonging
- Knowledge and its cost
- Connection and isolation

**Structural dimensions** — how the story moves, what shapes it takes
- Mystery (concealment → revelation)
- Romance (separation → union)
- Tragedy (ascent → fall)
- Comedy (disorder → restored order)
- Horror (safety → violation of safety)
- Quest (lack → fulfillment or failure)

**World affordances** — what is *possible* in this story's physics
- Magic: absent / subtle-felt / rule-bound / wild-mythic
- Technology: historical / contemporary / speculative / post-human
- Violence: consequence-laden / stylized / absent
- Death: permanent / negotiable / metaphorical
- The supernatural: nonexistent / ambiguous / matter-of-fact

A "genre" like "folk horror" is then a named cluster in this space — high dread, grounded aesthetic with mythic edges, subtle magic, consequence-laden violence, thematic focus on belonging and the cost of community. An author or player selects (or discovers) their position by navigating these dimensions, with named genres serving as **handholds** — recognizable starting points that can be adjusted, blended, or subverted.

### Tropes as Articulation Points

Genre names do more than categorize — they establish **tropes**, the shared narrative vocabulary that authors and audiences use to communicate expectations. Tropes are simultaneously:

- **Conventions to work with** — the reader expects them, and their presence creates comfort, recognition, anticipation
- **Conventions to subvert** — inverting a trope (the "final girl" who isn't pure, the detective who *is* the murderer) creates surprise, commentary, delight

Horror is the canonical example. The genre's tropes — the final girl, the creeping dread, the thing in the basement, the cursed knowledge — are so well-established that they function as a shared language. A horror story that reinforces these tropes delivers genre satisfaction. A horror story that inverts them delivers genre commentary. Both are valid; both depend on the audience *knowing* the tropes.

This means the system needs to model tropes not as fixed rules but as **expectations with directionality** — is this story working *with* or *against* this trope? The dramaturge needs this knowledge to shape the arc correctly. A noir mystery that's playing the detective-with-a-dark-past trope straight needs different pacing than one that's subverting it.

### Connection to the Dramaturge

The dramaturge's effectiveness depends directly on this richer genre model. Without it, the dramaturge can only reason about abstract arc positions (rising tension, climax, breath). With it, the dramaturge can reason about **genre-specific narrative expectations**:

- "In noir, the detective's moral compromise typically deepens through the middle act — we're in the middle act and the player character is still clean. Introduce temptation."
- "Folk horror builds dread through accumulation of mundane wrongness, not jump scares. The last three turns have been too directly threatening — pull back to unsettling normalcy."
- "This story is subverting the 'chosen one' trope — the dramaturge should resist pushing the player toward heroic action and instead create space for doubt and refusal."
- "Romantasy pacing needs alternation between relationship development and external threat. We've had four turns of pure dialogue — introduce an interruption from the world."

The genre model provides the dramaturge with a vocabulary of *what kind of dramatic work* is appropriate for this story, not just *how much tension* to apply.

---

## Part IV — Data Acquisition and Provenance

### The Breadth Problem

The current descriptor data is hand-authored — a handful of genres, a set of archetypes, some profiles and dynamics. This was the right starting point, but it can't scale to the combinatorial richness that genuine genre complexity demands. The number of meaningful genre intersections, trope variations, archetype expressions, and narrative shapes is vast.

However, the data exists. It lives in:

- **Public domain literature** — centuries of novels, stories, plays, and poetry with well-studied narrative structures, character archetypes, and genre conventions
- **CC-BY-SA RPG modules** — systems like d6, Cthulhu Reborn, and the many publishers on DriveThruRPG that moved to Creative Commons licensing after WotC's OGL debacle. These are particularly valuable because they already encode genre conventions as *playable mechanics* — world affordances, character archetypes, scene structures, tension mechanics
- **LLM training knowledge** — models already have extensive implicit knowledge of genre conventions, trope structures, and narrative shapes from their training data. This can be elicited through structured extraction rather than treated as a black box
- **Academic literary analysis** — narratology, genre theory, trope studies — much of which is available through open-access publications

### Extraction Strategy

The Python tooling in `tools/doc-tools/` already handles structured extraction from manuscripts (Scrivener projects, DOCX files). This pattern extends to a new extraction pipeline focused on **narrative structure and genre convention** rather than prose content:

1. **LLM-assisted elicitation** — Use local models (Ollama) to extract structured genre knowledge: "For the genre 'gothic romance,' what are the expected narrative beats? Common character archetypes? World affordances? Tonal markers? Tropes typically reinforced or subverted?" Run this across many genre intersections to build a broad initial map.

2. **Literature analysis** — Process public domain texts to extract empirical narrative structures. Not the prose itself, but the *shape* — chapter-level arc patterns, scene transition types, tension curves, character role distributions. This is structural analysis, not content reproduction.

3. **RPG module extraction** — CC-BY-SA game modules encode genre conventions explicitly: "In this setting, magic works like X, violence has Y consequences, the world affords Z." Extract these as structured world-affordance descriptors, archetype definitions, and scene-type templates.

4. **Cross-source synthesis** — Merge insights from multiple sources into a unified knowledge graph where nodes (tropes, archetypes, narrative shapes, genre conventions) are reinforced by convergent evidence from different origins.

### The Provenance Graph

This is where the ethical architecture matters. When synthesizing insights from multiple sources, the system must maintain:

**Multi-source attribution**: Every node in the genre/trope knowledge graph tracks which sources contributed to it. A node like "the detective's moral compromise deepens through the middle act" might be reinforced by analysis of Chandler (public domain), a noir RPG module (CC-BY-SA), and LLM-elicited genre knowledge. Each contribution is a weighted edge from source to node.

**Graceful degradation on takedown**: If a source needs to be removed — a CC-BY-SA license is revoked, an attribution claim is made, or an author requests removal — the system:
1. Removes all edges from that source
2. Decrements contribution weights on affected nodes
3. Prunes any node whose *sole* provenance was the removed source
4. Nodes with remaining multi-source support persist — the insight survives because it was independently reinforced

**No single-source dependency**: The graph's value comes from convergent insights, not from reproducing any single work. If a trope pattern is identified in Chandler, confirmed in a noir RPG module, and independently elicited from an LLM, that's three-source convergence. Removing any one source leaves the pattern intact. This is not just ethically sound — it produces more robust knowledge, because convergent insights are more likely to reflect genuine genre conventions rather than one author's idiosyncratic choices.

```
Source A (public domain novel) ──[contributed_to, weight: 0.3]──→ Node: "noir detective moral decline"
Source B (CC-BY-SA RPG module) ──[contributed_to, weight: 0.4]──→ Node: "noir detective moral decline"
Source C (LLM elicitation)     ──[contributed_to, weight: 0.3]──→ Node: "noir detective moral decline"

On removal of Source B:
  - Edge removed
  - Node weight recalculated: 0.3 + 0.3 = 0.6 (still above threshold)
  - Node persists with reduced confidence

On removal of Sources A and B:
  - Both edges removed
  - Node weight: 0.3 (single-source, LLM-only)
  - Policy decision: retain at low confidence, or prune as insufficiently grounded?
```

### Graph Structure

This provenance-aware knowledge graph is structurally similar to the relational web already in storyteller-core — directed edges with provenance, weighted relationships, topological roles. The difference is that this is **authoring infrastructure** rather than runtime narrative state. It lives in the data layer, informs scene composition and dramaturge reasoning, but doesn't participate in the turn cycle directly.

Candidate schema (conceptual):

```
GenreRegion {
    name: String,                           // "nordic noir", "cozy horror", "romantasy"
    position: Vec<(Dimension, f32)>,        // weighted position in narrative space
    tropes: Vec<TropeRef>,                  // associated tropes with reinforcement/subversion markers
    archetypes: Vec<ArchetypeRef>,          // character patterns common to this region
    narrative_shapes: Vec<NarrativeShape>,  // expected arc structures
    world_affordances: WorldAffordances,    // what's possible in this genre's physics
    provenance: Vec<ProvenanceEdge>,        // attribution chain
}

Trope {
    name: String,                           // "final girl", "chosen one", "noir moral decline"
    description: String,                    // what this trope is and why it resonates
    genre_associations: Vec<GenreRef>,      // where this trope appears
    subversion_patterns: Vec<Subversion>,   // known inversions and their effects
    narrative_function: NarrativeFunction,  // what dramatic work this trope does
    provenance: Vec<ProvenanceEdge>,
}

NarrativeShape {
    name: String,                           // "revelation arc", "descent", "romantic convergence"
    beats: Vec<Beat>,                       // expected progression with flexibility markers
    genre_associations: Vec<GenreRef>,
    tension_profile: TensionCurve,          // expected expansion/contraction rhythm
    provenance: Vec<ProvenanceEdge>,
}

ProvenanceEdge {
    source: SourceId,
    contribution_type: ContributionType,    // originated | reinforced | refined
    weight: f32,
    license: LicenseType,                   // public_domain | cc_by_sa | llm_elicited | hand_authored
    extractable: bool,                      // can this source's contribution be isolated for removal?
}
```

### Sequencing: Map Before You Pave

The instinct to explore breadth *before* hardening structures is exactly right. The risk of premature formalization is that early assumptions get baked into schemas, APIs, and runtime logic in ways that are expensive to evolve.

The proposed sequence:

1. **Broad LLM elicitation** — Use local models to generate a wide, rough map of genre space: dimensions, tropes, archetypes, narrative shapes, affordances. This is fast, cheap, and produces a landscape to reason about. Output is exploratory JSON, not production schema.

2. **Source extraction** — Process public domain literature and CC-BY-SA modules for structural patterns. Compare against the LLM-elicited map to find convergences and gaps. This grounds the map in empirical data.

3. **Identify articulation points** — Where does the space naturally cluster? What dimensions actually discriminate between genres in practice? Where are the useful boundaries and where are the false ones? This is the "find the joints" step.

4. **Formalize selectively** — Harden only the structures that have proven stable across multiple sources and use cases. Keep everything else as soft data (JSON descriptors, graph nodes) that can evolve without schema migrations.

5. **Wire into runtime** — Connect the formalized structures to scene composition and dramaturge reasoning. The soft data remains available for exploration and expansion without requiring code changes.

This sequence lets the combinatorial complexity *inform* the architecture rather than being constrained by premature decisions about what genres exist and how they relate.

---

## Part V — Narrative Spatial Topology and Place-as-Entity

### The Authored-Boundary Problem

The current system works within the confines of what was explicitly composed. A scene has a setting, characters, goals, constraints — and everything the narrator renders is grounded in that composed context. But the moment the player steps outside the authored boundary — walks into an unscripted room, leaves the meadow for the treeline, opens a door that nobody planned for — the system has nothing to fall back on except the LLM's generic extrapolation.

Generic extrapolation is exactly what produces the problems we've observed:

- **Direct echoing** of setting prompts — the narrator parrots back the authored description because it has no other source material for the space
- **Turn-over-turn repetition** — "his fingers play lightly over the circuits" appears in turn 1, turn 3, and turn 5 because the narrator doesn't know it already said that
- **Tonal drift** — an unscripted barn in a pastoral tale gets rendered with the narrator's generic "barn" rather than a barn *in this world, in this tone, in this story*

These are all symptoms of insufficient spatial and descriptive context. The narrator is doing its best with what it has, but what it has isn't enough.

### The Repetition Problem

The narrator currently has no memory of its own prose. It doesn't know it already described the candlelight, the dust motes, the character's nervous hands. The journal holds a compressed record of *what happened*, not *what was said about it*.

This connects to the dramaturge — part of its diagnostic role could include tracking **descriptive ground already covered** and signaling "find a new angle on this character's physicality" or "the room has been established through visual detail, shift to sound or smell." But it also suggests the narrator's context assembly needs a form of **negative constraint**: not just "here's what to render" but "here's what you've already rendered, don't echo it."

A lightweight approach: context assembly maintains a rolling **description ledger** — a compact log of sensory details, character descriptions, and setting elements already rendered in the current scene. This gets folded into the narrator's context as a "do not repeat" signal. The dramaturge can reference this ledger when identifying staleness.

### The Tonal Grounding Problem

A haunted mansion isn't just "a big old dark house." It's a *specific kind* of dark house whose rendering depends entirely on what the story is about:

- **Flanagan's Hill House** — grief architecture. Every room is a psychic space. The house is a character. Decay is emotional, not just physical. The Red Room isn't a room at all, it's the shape of each person's worst self.
- **A Ghost Hunters episode** — a physical space with measurable anomalies. EMF readings, cold spots, door 3 on the second floor. The house is a container for phenomena, not a participant in meaning.
- **Jackson's original** — the house is *wrong* in the way a face can be wrong. Not haunted so much as *malformed*. The horror is architectural, geometric, a place that shouldn't exist the way it does.

Same noun — "haunted mansion" — three completely different rendering vocabularies. The genre-as-hyperplane work from Part III feeds directly into this: the tonal dimensions and world affordances should inform *how* settings are described, not just *what* settings exist. A setting descriptor needs to carry not just physical facts but **aesthetic signature** — the sensory vocabulary, the descriptive register, the emotional valence that makes this place belong to *this* story.

### Place-as-Entity: Beyond Scenic Backdrops

The system already models characters as entities with tensor representations — multidimensional, context-dependent, capable of expressing different facets in different situations. Settings deserve the same treatment. A place is not a static backdrop; it's an **entity with narrative capacity**.

This connects to the system's deeper philosophical commitment: we do not impute consciousness as a categorical boundary. The entity model is built on **capacity-in-relationship** — what can this entity do, express, communicate, constrain, afford, *in relationship to* other entities? Characters have high communicability along verbal and emotional axes. A house has communicability along atmospheric, spatial, and sensory axes. A storm has communicability along physical constraint and emotional resonance axes. The difference is degree and dimension, not categorical kind.

This is not animism or anthropomorphism — it's recognizing that in narrative, *everything communicates*. The creaking floorboard, the too-bright kitchen, the warmth of the barn — these are the setting's communicative acts. The World Agent already exists to "translate the non-character world's communicability for the narrative." Place-as-entity gives it something richer to translate.

#### Compositional Entity Model

The existing entity framework in storyteller-core defines entities through:
- **EntityId** and **EntityOrigin** — identity and provenance
- **CommunicabilityProfile** — four dimensions of how an entity can participate in narrative exchange
- **PersistenceMode** — how the entity endures across scenes and sessions

Place-as-entity extends this without introducing a categorical split. A place-entity has:

**Communicability dimensions appropriate to place:**
- **Atmospheric** — mood, feeling, emotional valence (the oppressive warmth, the welcoming light)
- **Sensory** — what the place offers to sight, sound, smell, touch, taste
- **Spatial** — openness/enclosure, paths/barriers, above/below, transitions available
- **Temporal** — time-of-day sensitivity, seasonal change, historical layering (what this place was vs. what it is)

**Friction in communicability across boundaries** — the key insight. When a character moves from one place-entity to another, there is a communicative boundary crossing. The friction at that boundary is narratively meaningful:

- **Low friction** (meadow → farmhouse in a pastoral tale) — tonal continuity, the world feels coherent, the transition is unremarkable. The barn *belongs* to this landscape.
- **High friction** (mundane world → through the wardrobe) — tonal discontinuity is the *point*. The shift in the place-entity's communicability profile signals that something narratively significant has occurred. The world on the other side of the wardrobe communicates differently — more vividly, more mythically, with different affordances.
- **Uncanny friction** (well-lit hallway → subtly wrong kitchen in a horror story) — the communicability profile shifts in ways that are hard to articulate but palpable. The kitchen is *almost* right. The light is *almost* warm. This is the place-entity communicating threat through the gap between expectation and reality.

The friction model lets the system distinguish between mundane transitions (maintain continuity), transformative transitions (signal the shift), and unsettling transitions (exploit the gap). The World Agent can reason about this — "the player is crossing a low-friction boundary, maintain the tonal signature" vs. "this is a threshold crossing, the new space should feel fundamentally different."

### Narrative Spatial Topography

We need **place-to-place topology**, not map-based geography. The relationships between spaces are narrative, not geometric:

- **Adjacency** — what spaces are plausibly reachable from here? Not "100 meters north" but "through the door," "down the path," "across the square."
- **Tonal inheritance** — an unscripted adjacent space inherits the tonal signature of its parent location and the story's genre position. The barn near the meadow in a pastoral tale gets pastoral-barn-ness, not generic-barn-ness.
- **Narrative function** — spaces serve dramatic roles independent of their physical description:
  - **Sanctuary** — a place of safety, rest, vulnerability. Tension *decreases* here.
  - **Threshold** — a transitional space where the rules are about to change. Liminality.
  - **Arena** — where conflict unfolds. Tension *peaks* here.
  - **Labyrinth** — disorientation, being lost, the world conspiring against navigation.
  - **Forbidden zone** — accessible but wrong. The player shouldn't be here, and the place communicates that.
- **Permeability** — can you see/hear/smell/sense from one space into another? The kitchen where you can still hear the murmuring from the dark room. The garden wall over which the sounds of the party drift.

```
Place-Entity Topology (conceptual):

    [Dark Room]                    [Hallway]
    atmospheric: oppressive        atmospheric: transitional
    sensory: dim, murmuring        sensory: echo, coolness
    function: arena                function: threshold
        │                              │
        │ (high permeability:          │ (low friction,
        │  sound bleeds through)       │  tonal continuity)
        │                              │
        └──────── [Hallway] ───────────┘
                      │
                      │ (medium friction:
                      │  shift from tension to false safety)
                      │
                  [Kitchen]
                  atmospheric: bright, almost-warm
                  sensory: humming fridge, clean surfaces
                  function: sanctuary (or false sanctuary?)
                  tonal_note: "well-lit spaces in horror signal
                               vulnerability, not safety"
```

### Generating Unscripted Spaces

When the player moves to a space that wasn't pre-authored, the system generates it through **constrained extrapolation**:

1. **Inherit genre position** — the new space starts with the story's tonal, aesthetic, and world-affordance coordinates from the genre hyperplane (Part III)
2. **Inherit local topology** — adjacency to the current space determines physical plausibility (what kind of space connects to a hallway in a Victorian house?)
3. **Inherit tonal signature** — the parent space's atmospheric and sensory profile propagates with friction-appropriate continuity or discontinuity
4. **Assign narrative function** — the dramaturge (or World Agent) determines what dramatic role this space should serve given the current arc position. If tension is rising, the new space might be a threshold. If the player is fleeing, it might be a false sanctuary.
5. **Generate sensory palette** — draw from the genre-appropriate lexicon (Part III's tonal dimensions) to populate the space with specific sensory details. Not "a kitchen" but "a kitchen in *this* story" — warm flagstones and copper pots in the pastoral tale, buzzing fluorescent and cracked linoleum in the noir, impossible cleanliness in the horror.
6. **Register as entity** — the generated space becomes a place-entity in the topology graph, with its own communicability profile, available for the narrator to render and the World Agent to reason about in subsequent turns.

The key constraint: generated spaces should feel **discovered, not invented**. The player walks into the kitchen and it feels like the kitchen was always there, part of this world, waiting to be entered. Not a procedurally generated room that happens to be adjacent.

### The World Agent's Expanded Role

The World Agent is already defined as the "translator for non-character entities." Place-as-entity gives it substantially more to work with:

- **Spatial reasoning** — maintaining the topology graph, determining what spaces are plausibly adjacent, tracking permeability between spaces
- **Tonal enforcement** — ensuring generated spaces respect the genre position and tonal inheritance rules. The World Agent is the guardian of "this world feels coherent"
- **Environmental communicability** — translating the place-entity's atmospheric and sensory dimensions into narrative signals for the narrator. "The kitchen hums with a warmth that feels like reprieve" vs. "The kitchen light is too bright, too clean, a performance of normalcy"
- **Boundary friction** — determining how a transition between spaces should feel and signaling this to the narrator and dramaturge

This doesn't require the World Agent to become a cartographer. It requires it to become a **spatial dramatist** — understanding that every space in a story is narratively charged, and managing the spatial dimension of the player's experience with the same care that character agents manage the interpersonal dimension.

### World Agent Lifecycle: Scaffold, Then Enrich

Like the dramaturge (Part II), the World Agent benefits from an **async, non-blocking architecture** — though with an important distinction. The dramaturge is purely advisory (its directives shape context but never gate actions). The World Agent has moments where it must be **consulted synchronously** — if the player tries to break down a locked door or cross a river, the World Agent is the authority on whether that's possible and what it costs. But the majority of its spatial work — topology construction, tonal enrichment, sensory palette development — can and should happen asynchronously.

#### Phase 1: Initial Scaffold (Parallel with Scene Composition)

When a scene is composed, the World Agent receives the same inputs as the narrator — genre position, setting description, constraints, cast — and produces an **initial spatial scaffold**: the set of place-entities that plausibly exist in this setting.

This is not full authorial world-building. It's a structured inference: "We are in a gothic mansion. What rooms exist? How do they connect? What is each room's narrative function and tonal sketch?"

```
Scene Composition
    │
    ├──→ Narrator: receives composition, prepares for opening render
    │
    └──→ World Agent: receives same composition
              │
              ▼
         LLM scaffold call:
         "Given a gothic mansion in a grief-horror story,
          generate the plausible spatial topology."
              │
              ▼
         Structured response:
         [
           { name: "Entry Hall", function: threshold, adjacent_to: [front_door, staircase, drawing_room],
             atmosphere: "imposing, watchful", sensory: "dust, old wood, ticking clock" },
           { name: "Drawing Room", function: arena, adjacent_to: [entry_hall, library, conservatory],
             atmosphere: "faded grandeur", sensory: "heavy curtains, cold fireplace" },
           { name: "Library", function: sanctuary|labyrinth, adjacent_to: [drawing_room, study],
             atmosphere: "dense, close", sensory: "leather, paper, silence that isn't quite silence" },
           { name: "Cellar", function: forbidden_zone, adjacent_to: [kitchen_stairs],
             atmosphere: "below everything", sensory: "damp stone, one wall newer than the others" },
           ...
         ]
              │
              ▼
         Persist as place-entities with:
         - EntityId, CommunicabilityProfile
         - Adjacency edges in topology graph
         - Tonal sketch linked to genre-setting-tone lexicon
         - Status: scaffolded (not yet enriched)
```

The scaffold doesn't need to be exhaustive. It needs to cover the **likely spatial envelope** — the places a player might plausibly reach within the scene's scope. If the player reaches beyond the scaffold, the generation mechanism from §"Generating Unscripted Spaces" kicks in, but now with the scaffold as context (the new space is adjacent to *these* known spaces, not floating in a void).

**Crucially, the scaffold does not gate the first render.** The narrator can render the opening narration — the scene's starting location — while the World Agent is still scaffolding the broader topology. By the time the player's first input arrives, the World Agent has likely completed the scaffold. Even if it hasn't, the narrator has enough context to render the immediate space; the scaffold fills in the periphery.

This means that from the very first turn, when the narrator describes "the hallway stretches in both directions, a half-open door revealing the edge of a darkened library," that library *already exists* as a place-entity. The narrator isn't hallucinating a library — it's drawing on the World Agent's scaffold. The player can walk into it and find it coherent because it was generated as part of a spatial whole, not improvised at the moment of entry.

#### Phase 2: Async Enrichment (Ongoing During Play)

After the scaffold is in place, the World Agent continues to enrich place-entities asynchronously, similar to the dramaturge's ongoing evaluation:

- **Deepening sensory palettes** — the initial scaffold provides a tonal sketch ("leather, paper, silence that isn't quite silence"). As the scene progresses, the World Agent enriches this into a fuller sensory vocabulary, drawing from the genre lexicon, so the narrator has richer material when the player enters the library.
- **Responding to dramaturge signals** — if the dramaturge indicates that the scene needs a discovery space or a moment of false safety, the World Agent can enrich a previously-sketched room to serve that function. The cellar gains "one wall oddly newer than the others" because the dramaturge signaled that the scene's reveal needs a physical locus.
- **Tracking player attention** — if the player lingers near the drawing room but hasn't entered, the World Agent can proactively enrich it in anticipation. If the player seems to be heading upstairs, enrich the bedrooms.
- **Registering player-created spaces** — when the player opens an unexpected door or discovers a hidden passage, the World Agent generates and registers the new space, then asynchronously enriches it and updates the topology.

```
Turn N: Player enters the Entry Hall
    │
    ├──→ Turn cycle proceeds normally (narrator renders, dramaturge evaluates)
    │
    └──→ World Agent (async):
         - Enrich Drawing Room (player can see it from here, permeability: visual)
         - Enrich Staircase (player mentioned looking up)
         - Note: Cellar not yet relevant, defer enrichment

Turn N+1: Dramaturge signals "scene needs a confined space for confrontation"
    │
    └──→ World Agent (async):
         - Enrich Library as arena: "the shelves narrow toward the back,
           the single lamp creates a pool of light that doesn't reach the corners"
         - Update narrative function: sanctuary → arena
         - Persist enrichment, context assembly will pick it up
```

#### Synchronous Consultation Points

While most World Agent work is async, certain player actions require **synchronous spatial authority**:

- **Movement between spaces** — the World Agent confirms the transition is plausible and determines friction level. This can be fast (topology lookup, no LLM call) unless the destination doesn't exist yet (requires generation).
- **Physical interaction with the environment** — breaking a door, searching a room, climbing out a window. The World Agent determines feasibility based on the place-entity's affordances and the story's world-affordance constraints.
- **Boundary-crossing events** — passing through a threshold entity, entering a forbidden zone, crossing from the mundane world into the mythic. These transitions carry narrative weight and the World Agent must signal the tonal shift to both narrator and dramaturge.

The design principle: **async by default, sync only at decision points that change the spatial state.** This keeps the World Agent from becoming a bottleneck while ensuring spatial coherence when it matters.

#### Authored vs. Dynamic Settings

In a fully authored story (a published narrative being walked through as a reader), the spatial scaffold is pre-authored — every place-entity is written, enriched, and persisted before the first reader enters. The World Agent's role reduces to topology lookup and boundary-friction management.

In a dynamically composed story (a live playtest), the scaffold is generated at scene composition time and enriched during play. The same entity model, the same topology graph, the same communicability profiles — just populated differently.

This convergence is important: the data model doesn't distinguish between authored and generated place-entities. Both are place-entities with communicability profiles, tonal signatures, and narrative functions. The provenance tracks whether a space was hand-authored, scaffold-generated, or dynamically generated during play, but the runtime treats them identically. This means a story can be **partially authored** — key spaces pre-written, peripheral spaces generated — without architectural seams.

---

## Part VI — Scene Resolution and the Narrative Possibility Space

### Beyond Event Checklists

The naive model of scene resolution is a checklist: did the player find the key, did the conversation reach the reveal, did the character say the thing. This is adventure-game logic — necessary/sufficient conditions on a state machine. It works in systems where the world is fully authored and the player's agency is choosing between pre-defined paths.

In a generative system, the checklist collapses. There are too many ways to accomplish something — and more fundamentally, the "something" itself is fuzzier than a binary state change. If the scene's purpose involves a character entering a locked house, the player might find the key (authored path), kick in a window (improvised), convince a neighbor to let them through a shared wall (emergent from NPC interaction), or never enter at all — and the story is not "stuck" in that last case. It takes a different shape.

"Find the key to enter the house" is an artifact of a much less dynamic world. Even in video games, a physics engine would need to support kicking in windows, climbing trees to reach rooftops, or other lateral approaches. In literature — and this is a generative, interactive fiction engine — the metaphor of "find the key" dissolves entirely. The key is not a puzzle gate. It's a **narrative possibility**. Its presence or absence changes what story shapes remain available. The house that was never entered becomes a different kind of narrative object — a mystery that pulls from outside, an unresolved gravitational center.

### The Medium Is the Message

If this is an interactive, generative fiction engine, then **authoring itself looks different**. The traditional authored narrative graph — scene A leads to scene B if condition X, scene C if condition Y — assumes a finite, pre-enumerable set of paths. We've already identified the sparse-graph problem: between authored content nodes, whole scenes need to be dynamically generated. The playtesting work was driven in part by this realization — we need to understand what generated scenes *feel like* before we can design the connective tissue between authored nodes.

But the deeper insight is that the distinction between "authored content" and "generated content" is less important than the distinction between **narrative states that foreclose possibilities** and **narrative states that preserve them**. An authored scene and a generated scene both produce narrative graph state changes. What matters is whether those changes are tracked, meaningful, and consequential — not whether a human or a system wrote them.

This reframes authoring from "writing scenes and connecting them with conditions" to something more like **establishing narrative attractors and constraints**:

- An author defines the major gravitational centers — the pivotal events, the transformative revelations, the irreversible choices
- An author defines the world's affordances and constraints — what's possible, what's forbidden, what costs what
- An author defines character essences — who these people are at bedrock, what they want, what they fear
- The system generates the connective tissue — the scenes between pivots, the moment-to-moment unfolding, the player's improvised path through the possibility space

The author is less a screenwriter and more a **world-builder and dramatist** — establishing the conditions for stories to emerge rather than scripting the stories themselves.

### From Event Combinatorics to Possibility Foreclosure

The existing design documents discuss a threshold-based approach to event detection: authored events that change narrative course have a truth-value ("did or did not occur"), while gradual relational changes organically foreclose opportunities over time. These are related but operate differently:

**Threshold events** are inflection points — a secret revealed, a betrayal committed, a door opened that can't be closed. These produce **discrete state changes** in the narrative graph. After the secret is out, certain story shapes are no longer reachable and new ones become available. The question "did this event occur" still matters, but the answer doesn't need to be checked against a specific authored trigger. The event decomposition system (Part I.2's structured LLM extraction) can identify that a functionally equivalent event occurred even if it happened through an unplanned path.

**Gradual foreclosure** is more subtle and more interesting. Over the course of several turns, a relationship might cool, trust might erode, a character might become increasingly guarded. No single event is the "cause" — it's the accumulation. At some point, the possibility of a particular story shape (the reconciliation, the confession, the alliance) has been foreclosed not by a discrete event but by the relational substrate shifting beneath it. The ML predictions and character tensor evolution track exactly this kind of change.

**Rapid foreclosure** occupies a middle ground — a hurtful word, a violent action, something that isn't a pre-authored threshold event but produces an immediate, significant shift in the possibility space. The relational web updates sharply rather than gradually. The event decomposition flags it as high-impact. The dramaturge notes that the arc has been altered.

What unifies all three is that they're about **narrative graph topology change** — not "did X happen" but "what shapes are still reachable from where we are now." The scene's possibility space is a landscape, and events (threshold, gradual, rapid) reshape that landscape by raising barriers, opening valleys, or collapsing paths.

### Scene Resolution as Convergence Signal

A scene resolves not when a checklist is satisfied but when the **possibility space has been sufficiently narrowed or transformed** — when the scene's dramatic work is done, however it was accomplished.

This requires a convergence of signals from multiple system components:

| Component | Signal | What it reveals |
|-----------|--------|-----------------|
| **Event decomposition** | What actions occurred, what information moved | Discrete state changes in the narrative graph |
| **ML predictions** | Character tensor evolution, relational shifts | Gradual or rapid foreclosure of relational possibilities |
| **Dramaturge** | Arc position, dramatic work accomplished | Whether the scene's *purpose* has been served |
| **Scene goals** | Original dramatic intent | The benchmark against which resolution is measured |
| **Intent system** | Player and NPC intention fulfillment or frustration | Whether the motivational forces driving the scene have been spent |

Scene resolution is the dramaturge's determination that these signals have converged:

- The **dramatic work** has been accomplished — or definitively failed. The revelation happened (through whatever path), or the opportunity for revelation has been foreclosed (the secret-keeper left, the conversation moved irrevocably past it, trust eroded to the point where vulnerability is impossible).
- The **possibility space** has narrowed to the point where remaining in this scene produces diminishing returns. This is the echo-stagnation signal — the dramaturge detects that the scene's remaining possibilities are variations on what's already occurred, not new narrative territory.
- The **relational and informational state** has shifted enough that a new context — a new scene — is the right container for what comes next. The characters need a different setting, different cast, different stakes to continue the story's movement.

Crucially, "accomplished" doesn't mean the authored outcome occurred. It means the **narrative graph has been meaningfully altered**. If the scene's purpose was "Arthur's secret about the letter is exposed," resolution could look like:

- Margaret found the letter (authored path) → the secret is out, trust is shattered, the story moves to confrontation
- Arthur confessed under pressure (player-driven) → the secret is out, but Arthur chose to reveal it, the relational dynamic is different
- A third party revealed it accidentally (emergent from NPC intent) → the secret is out, but neither Arthur nor Margaret controlled the moment, the story moves to mutual vulnerability
- Arthur successfully concealed it but at visible relational cost (organic failure) → the secret is intact, but Margaret now suspects *something*, and trust has eroded. The scene's dramatic work was "done" even though the event didn't fire — the possibility space has been reshaped

All of these resolve the scene. They produce different narrative graph states — different downstream possibilities — but they all move the story forward. The dramaturge recognizes resolution not by checking a condition but by evaluating the **aggregate state of narrative movement**.

### The Dramaturge's Resolution Protocol

When the dramaturge evaluates scene resolution, it produces a **resolution assessment** as part of its ongoing directive output:

```
ResolutionAssessment {
    scene_id: SceneId,
    turn: TurnId,

    // How much of the scene's dramatic work is complete?
    dramatic_completion: f32,          // 0.0 → 1.0, not a simple threshold

    // What happened to the possibility space?
    foreclosed_paths: Vec<NarrativePath>,    // story shapes no longer reachable
    opened_paths: Vec<NarrativePath>,        // new shapes that became available
    active_tensions: Vec<Tension>,           // unresolved forces (carry to next scene)

    // Resolution recommendation
    resolution_signal: ResolutionSignal,     // continuing | approaching | ready | overdue
    recommended_exit: Option<ExitMode>,      // fade | cut | cliffhanger | denouement | rupture

    // What this scene accomplished (for the narrative ledger)
    narrative_state_change: NarrativeStateDelta,
    // e.g., "trust between A and B shifted from 0.7 → 0.3"
    // e.g., "information X moved from hidden → known by B"
    // e.g., "possibility of alliance path foreclosed by accumulated friction"
}
```

The `resolution_signal` is not a binary gate — it's a gradient:

- **Continuing** — the scene's dramatic work is in progress, the possibility space is actively being shaped. Stay here.
- **Approaching** — most of the dramatic work is done, the remaining possibilities are narrowing. The dramaturge begins shaping turns toward closure (breath moments, final exchanges, environmental cues that signal winding down).
- **Ready** — the scene has accomplished its dramatic work. A transition would feel natural and satisfying. The narrator can be given a closing-turn directive.
- **Overdue** — the scene has been resolved for multiple turns but hasn't transitioned. This is the echo-stagnation state. The dramaturge should escalate — introduce an interruption, a departure, a time-skip, something that breaks the loop.

The `recommended_exit` carries tonal information for the narrator:

- **Fade** — gentle transition, the scene dissolves. Appropriate for scenes that resolve in peace or exhaustion.
- **Cut** — sharp transition, the scene ends mid-action or mid-sentence. Appropriate for cliffhangers, interruptions, moments of violence.
- **Denouement** — the scene's tension has resolved and a moment of settling follows. Appropriate for revelations, reconciliations, arrivals.
- **Rupture** — something breaks the scene open — an arrival, a departure, an event that belongs to the *next* scene intruding on this one. Appropriate for escalation, for the plot pulling the characters forward against their will.

### Scene-to-Scene: The Narrative Graph Evolves

Each resolved scene produces a `NarrativeStateDelta` that gets committed to the event ledger. This delta is the scene's **contribution to the narrative graph** — what changed, what was foreclosed, what opened up, what tensions carry forward.

The next scene's composition draws on this delta:

- **Carry-forward tensions** become the new scene's starting dramatic material
- **Foreclosed paths** are removed from the possibility space the scene composer considers
- **Opened paths** become available as new scene goals or dramatic directions
- **Relational state changes** propagate to character tensors, influencing how character agents behave in the next context

This is how scenes compose into chapters and chapters into stories — not through pre-authored branching paths, but through **accumulated narrative state** that progressively shapes and constrains the possibility space. Each scene is a local exploration within a global landscape that the player's choices are continuously reshaping.

The sparse-graph problem dissolves under this model. There is no sparse graph to fill in. There is a **narrative state** that evolves continuously, and scenes are generated as the appropriate dramatic containers for the current state. Authored pivots — the major gravitational centers — still exist as attractors in the possibility space, but the paths between them are emergent, generated from the accumulated state rather than pre-connected.

### Fiction Has Endings

One final note: fiction is not a forever-running conversation. Stories end. The narrative possibility space doesn't just narrow scene by scene — it converges toward resolution at the story level. The same fractal pattern applies: just as a scene resolves when its dramatic work is done, a story resolves when its thematic questions have been answered (or deliberately left open, which is itself a resolution).

The dramaturge at the story level — whether the same agent with broader context or a higher-level evaluator — tracks this convergence. Are the major tensions resolving or intensifying toward a final confrontation? Are the thematic threads weaving toward a conclusion? Has the player's character arc reached a point of transformation or confirmed stasis?

This is the furthest horizon of the system's ambition: not just scenes that feel alive, but stories that feel **complete** — that have the shape and weight of authored fiction, achieved through the collaboration of human player, AI agents, and the accumulated state of a world that remembers everything that happened in it.

---

## Summary of Interconnections

The six parts of this document describe a coherent system, not independent features:

```
Part I: Pragmatic Preamble
  │
  ├── §1 Sequential RPCs ──→ enables richer scene composition
  ├── §2 workshop:gameplay ──→ streaming channel for all of the below
  ├── §3 workshop:media ──→ depends on §2 for narrator buffer
  ├── §4 Turn 0 opening ──→ first event on §2's channel
  └── §5 Player intent review ──→ feeds into dramaturge + scene goals
         │
         ▼
Part II: The Dramaturge
  │  Async agent that shapes turn-by-turn narrative direction.
  │  Depends on: scene goals (§I.5), genre model (III), event system (VI)
  │  Produces: DramaticDirectives consumed by context assembly
  │
  ├──→ Part III: Genre as Hyperplane
  │    Provides the dramaturge with genre-specific narrative vocabulary.
  │    Tropes, tonal dimensions, world affordances, narrative shapes.
  │         │
  │         ▼
  │    Part IV: Data Acquisition and Provenance
  │    Populates the genre hyperplane from public domain, CC-BY-SA,
  │    and LLM elicitation. Provenance graph for ethical attribution.
  │
  ├──→ Part V: Narrative Spatial Topology
  │    Place-as-entity, tonal inheritance, World Agent lifecycle.
  │    The dramaturge signals what spatial narrative functions are needed;
  │    the World Agent provides the spaces. Genre model (III) informs
  │    tonal signatures. Compositional entity model unifies characters
  │    and places under capacity-in-relationship.
  │
  └──→ Part VI: Scene Resolution
       The dramaturge determines when a scene's dramatic work is done.
       Event decomposition + ML predictions + arc tracking converge
       into a resolution signal. Possibility foreclosure replaces
       event checklists. Scene deltas feed the next scene's composition.
       Stories, like scenes, have endings.
```

Each part depends on and enriches the others. The pragmatic fixes (Part I) unblock the architectural work. The dramaturge (Part II) needs genre knowledge (Part III) and spatial awareness (Part V) to shape scenes effectively. The data pipeline (Part IV) feeds both genre modeling and spatial generation. Scene resolution (Part VI) is the dramaturge's culminating judgment, informed by everything else.

The unifying thread: **the turn-by-turn scene unfolding is the primary experiential surface**. Everything in this document serves making that surface feel rich, directed, alive, and — when the time is right — resolved.

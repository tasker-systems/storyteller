# Authoring Economics: Gravity as a Design Surface

## Purpose

This document addresses a practical question: **what does a story designer actually spend their time on?**

Interactive narrative systems face a combinatorial authoring problem. A kind word in one place changes the whole course of a relationship, which changes the whole course of a story. Every branch multiplies the authoring burden. Branching-tree authoring — the dominant paradigm in interactive fiction and game narrative — requires designers to enumerate paths, and the number of paths grows exponentially with the number of meaningful player choices. This leads to one of two outcomes: either the designer constrains choices to keep the tree manageable (producing an illusion of agency), or the tree becomes sparse and the vast majority of paths feel under-authored.

The storyteller system's gravitational model offers a different approach. Instead of authoring paths, the designer authors a **gravitational landscape** — a field of narrative attractors with mass, approach vectors, and event dependencies. The system then generates trajectories through that landscape, guided by the authored forces. The designer creates the stars; the system computes the orbits.

This document explores what that means for the practice of writing interactive fiction.

**Prerequisites**: Familiarity with narrative gravity ([`narrative-gravity.md`](../technical/graph-strategy/narrative-gravity.md)), event dependencies ([`event-dag.md`](../technical/graph-strategy/event-dag.md)), and the scene model ([`scene-model.md`](../technical/scene-model.md)).

---

## The Combinatorial Problem

### Why Branching Trees Fail

Consider TFATD Part I with 8 scenes. If each scene has just 3 meaningful outcomes, a branching tree requires authoring 3^8 = 6,561 distinct paths. Even with aggressive path merging (converging branches at Gate scenes), the designer faces hundreds of distinct scene variants.

In practice, designers respond by:

1. **Constraining choices** — reducing player agency to 2-3 predetermined paths with cosmetic variation
2. **Making choices cosmetic** — different dialogue, same outcome
3. **Front-loading and back-loading** — rich opening and conclusion, sparse middle
4. **Leaving gaps** — minor scenes get minimal authoring attention, creating tonal inconsistency

All of these are compromises. The player feels them. The "sparse middle" of a branching tree is where pacing lives — the quiet moments, the character development, the transitions that make the big moments earn their weight. When these are under-authored, the story lurches between set pieces.

### The Inverse Relationship: Effort vs. Combinatorics

The scenes that matter most narratively are the ones where player choice has the greatest impact — and these are precisely the scenes that multiply the authoring burden most aggressively. A pivotal scene with 4 meaningfully different outcomes creates 4× the downstream work. This inverse relationship means designers are economically incentivized to reduce the impact of player choice at the very moments it should matter most.

---

## The Gravitational Alternative

### Author the Field, Not the Paths

In the storyteller model, the designer's primary creative artifact is not a branching tree but a **gravitational landscape**:

- **Scenes with mass** — pivotal moments that pull the narrative toward them
- **Approach vectors** — the narrative predicates that a scene wants satisfied as the story approaches
- **Event dependencies** — the structural requirements and exclusions that gate scene accessibility
- **Departure trajectories** — how exit from a scene reshapes the landscape for what follows

The designer does not specify what happens *between* the authored scenes. Instead, the gravitational landscape implies trajectories: the narrative bends toward high-mass scenes, approach vectors define what needs to happen along the way, and the system generates connective tissue that serves those structural needs.

### What the Designer Writes

The designer's creative energy concentrates on work that benefits from human aesthetic judgment:

**Gate scenes** — always hand-authored. These are the structural commitments that reshape the possibility space. Adam's threshold, Kate's blessing, the Wolf's summoning. Gate scenes define the story's architecture. They require full creative attention because they establish voice, tone, stakes, and the felt experience of narrative transformation.

**Heavy-mass scenes with multiple outcomes** — the pivotal moments where different player choices produce genuinely different narrative experiences. The designer authors 2-4 outcome variants for each, defining how each outcome reshapes the gravitational landscape (which approach vectors are satisfied, what events are resolved, how departure trajectories differ). This is where the designer's effort has maximum leverage — each variant creates a different downstream gravitational field.

**Tonal calibration** — voice, atmosphere, genre contract. The aesthetic parameters that make this story distinct from any other story with similar structure. This calibration propagates through generated content: the system generates connective scenes that match the authored tone because the tonal parameters are part of the generation context.

**Character definition** — tensors, relational substrates, contextual triggers. The personality data that character agents use to improvise in-context. The designer writes the characters; the system lets them perform.

### What the System Generates

The system generates content that is structurally determined but aesthetically variable:

**Texture scenes** — pacing, atmosphere, character development. The quiet moments between major events. A walk through the woods where Sarah notices the light changing. A conversation between Tom and Beth that reveals their dynamic without advancing the plot. These scenes exist because the gravitational model says "the narrative needs a beat here" — the approach vector for the next Gate scene requires emotional preparation, and a texture scene serves that preparation.

**Minor threshold scenes** — small transitions that the designer didn't explicitly author but that the event DAG requires. "Sarah finds the path" is a minor threshold that enables "Sarah enters the Shadowed Wood." The designer authored the destination but not every step of the approach.

**Connective space** — travel, transitions, world-texture. The diminishing-returns mass model from [narrative-gravity.md](../technical/graph-strategy/narrative-gravity.md) ensures connective content accumulates weight without overwhelming. Three paragraphs of travel description contribute atmosphere; three pages would exhaust the reader. The system generates within the budget that the gravitational model allocates.

**Variation in minor scenes** — when the same structural beat can be satisfied multiple ways (different characters present, different setting, different mood), the system selects based on current relational state, entity proximity, and tonal parameters. Each playthrough gets a slightly different connective tissue, maintaining replayability without requiring the designer to author every variant.

---

## Gravitational Guidance for Generation

### Why This Isn't "Generate Anything"

A naive generative system produces content that is plausible but structurally unguided — it might generate a scene that is well-written but serves no narrative function. The gravitational model provides remarkably specific constraints for generation:

**What the scene should accomplish**: Approach vector satisfaction tells the system which narrative predicates need advancement. If the next high-mass scene requires `sarah_trusts_wolf = true` and that predicate is currently at 0.3, the generated scene should create conditions where Sarah's trust in the Wolf can develop. The scene has a *job*.

**What tone it should carry**: Gravitational proximity determines atmospheric saturation. Far from an attractor, the generated scene carries faint thematic echoes. Close to an attractor, the atmosphere is dense with foreshadowing, tension, or whatever quality the attractor represents. The gradient from distant to imminent is the system creating the subjective experience of "building toward something."

**Who should be present**: Entity proximity (from [setting-topology.md](../technical/graph-strategy/setting-topology.md)) determines which characters are physically available. Relational web state determines which pairings are dramatically interesting — characters with high substrate tension or recent relational shifts are more narratively charged. The system selects cast based on both availability and dramatic potential.

**What can and cannot happen**: The event DAG knows what has been enabled, what is excluded, and what gates remain. A generated scene cannot resolve an event whose prerequisites are unmet. It cannot introduce information that the Storykeeper's information boundaries forbid. The constraint space is tight enough that generation operates more like constrained improvisation than open-ended creation.

**How long it should be**: The token budget allocation from [cross-graph-composition.md](../technical/graph-strategy/cross-graph-composition.md) determines how much narrative space the scene receives. Minor connective scenes get small budgets. Scenes near a gravitational attractor get larger budgets as the system allocates more attention to high-gravity regions.

### The Constraint Gradient

The strength of generative guidance varies by proximity to authored content:

| Proximity to Authored Scene | Guidance Strength | Generation Character |
|----------------------------|-------------------|---------------------|
| Adjacent (1 scene away) | Very high | Almost fully constrained — approach vectors, cast, tone, and event state leave little ambiguity |
| Near (2-3 scenes away) | High | Strong structural guidance, some aesthetic latitude |
| Mid-range (4-5 scenes away) | Moderate | Clear direction (gravitational pull) but room for character-driven improvisation |
| Far (6+ scenes away) | Low | General tonal guidance only — the system knows the *kind* of scene needed but not the specifics |

This gradient naturally concentrates quality where it matters most. Scenes adjacent to authored pivotal moments — the dramatic approaches and immediate aftermaths — have the tightest constraints and therefore the most reliable generation. Scenes in the connective middle have more latitude, which is appropriate: these are the pacing beats where variety enhances rather than undermines the experience.

---

## The Authoring Workflow

### How a Designer Uses the System

**Phase 1: Write the anchor scenes**

The designer writes the scenes they care most about — the moments of transformation, revelation, confrontation, departure. These are fully authored prose with multiple outcome variants where player choice matters. The designer works as a fiction writer here, focusing on voice, character, imagery, emotional truth.

For TFATD Part I, this might be 4-5 scenes: Adam's threshold (S2), Kate's blessing (S3), the Wolf's summoning (S3b), the stream crossing (S6), and Sarah's homecoming (S8). Each with 2-3 outcome variants.

**Phase 2: Define the gravitational landscape**

The designer assigns mass, approach vectors, and departure trajectories to the authored scenes. They define the event dependency graph — what requires what, what excludes what, what enables what. This is structural work, not prose. It's the narrative architecture that the system uses to generate trajectories.

This phase also includes defining setting topology (where things are, how they connect, what traversal costs) and relational web initial state (how characters relate at the story's opening).

**Phase 3: System generates candidate connective scenes**

Given the authored anchor scenes and the gravitational landscape, the system generates the connective tissue. The designer reviews, selects, adjusts. Some generated scenes are good enough as-is. Some need prose polish. Some reveal that the gravitational landscape needs adjustment — a generated scene feels wrong because an approach vector was miscalibrated or a mass was too high.

**Phase 4: Curate and override**

The designer replaces generated content that doesn't meet their aesthetic standard. They adjust masses and vectors when the system's trajectories don't match their intent. They might author an additional scene that the system couldn't generate because it required a creative leap the constraints didn't imply.

**Phase 5: Iterate**

As the designer sees what the system generates across multiple playthroughs, they refine the landscape. Masses get tuned. Approach vectors get sharpened. Some connective scenes that the designer initially left to generation get promoted to fully authored — the designer realizes this beat matters more than they thought. Others get demoted — an authored scene turns out to be structurally redundant and the system's generated alternative is better.

### The Key Difference from Branching Trees

In branching-tree authoring, the designer's effort scales with the number of paths. Adding a new choice at a branch point multiplies downstream work. The designer is fighting combinatorics.

In gravitational authoring, the designer's effort scales with the number of **attractor scenes**, not the number of paths between them. Adding a new possible outcome to a pivotal scene changes the gravitational landscape, and the system recomputes trajectories through it. The paths are emergent, not enumerated. The designer is defining a field, and the field has properties (continuity, smooth gradients, attractor basins) that make the paths through it well-behaved without explicit authoring.

This means:

- **More meaningful choices, not fewer** — the designer can offer 4-5 genuinely different outcomes at a pivotal scene without 4-5× the downstream authoring cost
- **Richer connective tissue** — the middle of the story is generated with structural guidance rather than left sparse
- **Tonal consistency** — generated content inherits tonal parameters from the authored anchor scenes, maintaining voice across the full experience
- **Replayability from trajectory variation** — different playthroughs traverse different paths through the same gravitational field, experiencing different connective tissue while hitting the same (or different) attractors

---

## The Sparse Graph Problem

### What Prompted This Insight

Interactive narratives built on rich domain models — character tensors, relational webs, event dependencies, gravitational landscapes — have a paradox. The model is sophisticated enough to support enormous narrative variation. But the authored content to fill that variation doesn't exist. The narrative graph is **sparse**: the author has populated the heavy-mass nodes and a few key connections, but the space between them is empty.

A player traversing a sparse narrative graph hits the authored peaks and then falls into voids — moments where the system knows structurally what should happen but has no authored content to express it. The mismatch between model sophistication and content coverage produces an uncanny experience: the system clearly *knows* what the story should feel like, but it can't *say* it.

### The Gravitational Solution to Sparsity

Narrative gravity reframes sparsity as a feature rather than a failure. In a gravitational field, the space between massive bodies isn't empty — it's filled with the field itself. The gravitational influence of nearby scenes provides:

- **Direction** — the narrative is always being pulled toward something
- **Intensity gradient** — the pull strengthens as approach vectors are satisfied, creating rising tension
- **Tonal field** — the atmosphere of nearby high-mass scenes bleeds outward, coloring the space between them

Generated connective content doesn't need to be as crafted as authored content. It needs to be **tonally correct** and **structurally functional** — advancing the right predicates, maintaining the right atmosphere, placing the right characters in proximity. The gravitational model provides exactly these constraints.

The sparse graph isn't incomplete. It's a skeletal structure that the system clothes in generated tissue. The author provides the bones; the system provides the muscle, skin, and movement.

---

## Implications for Story Designers

### The New Skill Set

Gravitational authoring requires a different skill set than branching-tree authoring:

**Traditional interactive fiction skills** (still essential):
- Writing compelling scenes with strong voice
- Creating memorable characters with clear motivations
- Crafting dialogue that reveals character and advances plot

**New gravitational authoring skills**:
- **Mass assignment** — intuiting which scenes should have high narrative gravity and calibrating their pull. Too much mass on too many scenes creates a jerky narrative; too little mass creates drift.
- **Approach vector design** — defining what the story needs to accomplish on the way to each pivotal moment. This is structural storytelling: understanding what emotional, informational, and relational groundwork makes a scene land.
- **Departure trajectory thinking** — how each outcome reshapes the landscape. The designer thinks not just "what happens if the player does X" but "how does X change what the story wants next."
- **Letting go of connective tissue** — trusting the system to generate the moments between peaks. This is the hardest skill. Writers want to control every sentence. Gravitational authoring asks them to control the forces and trust the trajectories.

### The Writer's Experience

A story designer working in this system would spend their time:

1. **Writing fiction** — the anchor scenes, the pivotal moments, the transformative experiences. This is the work writers love and are best at.
2. **Designing structure** — masses, vectors, dependencies. This is closer to game design than fiction writing, but it's informed by the writer's narrative instinct.
3. **Curating generation** — reviewing what the system produces, keeping what works, replacing what doesn't. This is editorial work: selection and refinement rather than creation from scratch.
4. **Tuning the field** — adjusting the gravitational landscape based on observed trajectories. This is iterative: play through, notice where the narrative feels off, adjust the forces, play through again.

The balance shifts from "write everything" to "write what matters most and shape the forces that generate everything else." For a writer who finds branching-tree authoring tedious and combinatorially exhausting, this is a liberation. For a writer who wants control of every word, it requires a new kind of trust.

---

## Relationship to Other Documents

This document describes the design philosophy and practical workflow for using the storyteller system. The mathematical foundations that make it possible are formalized in:

- [**Narrative Gravity**](../technical/graph-strategy/narrative-gravity.md) — how mass, distance, and pull are computed
- [**Event DAG**](../technical/graph-strategy/event-dag.md) — how event dependencies constrain the possibility space
- [**Traversal Friction**](../technical/graph-strategy/traversal-friction.md) — how information propagates and attenuates between scenes
- [**Cross-Graph Composition**](../technical/graph-strategy/cross-graph-composition.md) — how multiple graph signals combine for context retrieval and budget allocation
- [**Setting Topology**](../technical/graph-strategy/setting-topology.md) — how spatial constraints bound entity availability

The system architecture documents describe how the agents implement these mathematical models:

- [**System Architecture**](../foundation/system_architecture.md) — agent roles and information boundaries
- [**Narrator Architecture**](../technical/narrator-architecture.md) — how the Narrator renders generated and authored content
- [**Storykeeper API Contract**](../technical/storykeeper-api-contract.md) — how the Storykeeper curates context using gravitational retrieval

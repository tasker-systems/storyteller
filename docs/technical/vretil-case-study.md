# Vretil: Tales-Within-Tales Case Study

## Purpose

This document applies the tales-within-tales framework (`tales-within-tales.md`) to Vretil, a 20-chapter novel with three interleaved narrative layers. Where the TFATD case study (`narrative-graph-case-study-tfatd.md`) validated the single-layer narrative graph model against a compact story with 6 characters and 8 scenes, this case study validates the multi-layer sub-graph model against a full-length novel with complex structural properties: a flash-forward prologue, three distinct narrative threads that progressively converge, an unreliable narrator, prophetic sub-graphs whose events predict parent-graph scenes, and a boundary permeability curve that moves from firm separation to complete merger.

Vretil is authored fiction — there are no "what if" alternate paths or emergent events to model. The value of this case study is not in branching possibilities but in structural complexity: how the storyteller system represents nested narrative layers, tracks entity projections across those layers, manages cascade-on-exit semantics, and models the temporal evolution of the graph as the story unfolds.

### Relationship to Other Documents

- **`tales-within-tales.md`** — Defines the conceptual framework. This document instantiates it with concrete data.
- **`knowledge-graph-domain-model.md`** — Defines the four graphs. This document shows how they compose across sub-graph layers.
- **`narrative-graph-case-study-tfatd.md`** — Single-layer validation. This document extends to multi-layer validation.
- **`storykeeper-api-contract.md`** — Defines the Storykeeper's operations. This document exercises the sub-graph extensions.

---

## The Novel's Structure

### Synopsis

Chris, a restaurant worker whose life revolves around caring for his mentally ill sister Mallory, receives a box containing photographs of himself as a child with an unknown woman across dozens of locations, plus journals filled with prophetic writing. Mallory maps the photographs to an incomplete geographic journey and proposes Chris finish it. They depart on a road trip south and west across America.

The journey takes them through encounters with a series of people — Henri (a mechanic with spiritual perception), Isabella (a glass artist), Stephanie (a nurse from Mallory's hospital) — each revealed to be a node in a network orchestrated by the unknown woman (the "Prophet"). At the novel's midpoint, Chris learns Mallory has been dead for two months; his entire experience of traveling with her was grief-psychosis or genuine haunting.

The journey ends in a New Mexico cave where Chris enacts a ritual self-sacrifice — choosing to die by his own hand rather than be consumed by the ritual's forces. The sacrifice births a goddess (Vretil). Chris survives, and the novel ends with him walking out of the cave into sunlight.

Three narrative threads interleave throughout: the **main narrative** (Chris's first-person journey), numbered **epistles** (prophetic writings that frame each chapter), and a **fairy tale** ("The Very Long Journey," a children's story about a boy searching for his lost sister). These three threads progressively converge until they merge completely in the final chapter.

### Chapter-Thread Distribution

| Chapter | Thread(s) | Structural Role |
|---|---|---|
| 1 | Epigraph (Jung) | Meta-frame |
| 2 | Main (flash-forward) | Prologue — the cave destination |
| 3 | Main + Epistle (embedded) | Inciting incident — the box arrives |
| 4 | Epistle + Main | The woman appears; Mallory analyzes the journey |
| 5 | Epistle + Main (+ fairy tale dream) | Departure; first fairy tale intrusion |
| 6 | Epistle + Main (+ fairy tale invoked) | Cemetery; medication confrontation; fairy tale as object |
| 7 | **Fairy tale only** | First standalone fairy tale chapter |
| 8 | Epistle + Main | Henri; recovered memory; New Orleans |
| 9 | Epistle + Main | The cabin; Mallory departs |
| 10 | Epistle + Main | **Mallory revealed dead**; Isabella |
| 11 | **Fairy tale only** | The boy's mountain crossing and the old woman |
| 12 | Epistle + Main | Necromantic ritual; cosmic vision |
| 13 | Epistle + Main + Dream | Rebuilding; the wolf/raven/lamb dream |
| 14 | Epistle + Main | Destruction of normalcy; Chris resumes journey |
| 15 | **Fairy tale only** | The boy's sacrifice in the cave |
| 16 | Epistle + Main | New Mexico; Sarah; the photographs |
| 17 | Epistle + Main | The apparition; entering the cave |
| 18 | Epistle + Main (boundaries shatter) | Visions cascade; ritual death |
| 19 | **Fairy tale only** | Recovery; the goddess named |
| 20 | Epistle + Main + Fairy tale (merged) | All threads converge; resolution |

**Pattern**: The fairy tale occupies standalone chapters at positions 7, 11, 15, 19 — every fourth chapter, creating a structural rhythm. Chapters 11 and 15 form symmetric bookends around the main narrative triptych (12-13-14). The epistles appear in every non-fairy-tale chapter. The dream sequences (chapters 5, 13) serve as bridge layers.

---

## The Three Sub-Graphs

### Sub-Graph 1: Main Narrative (Root Story-Graph)

**Type**: Root — not a sub-graph but the parent graph that contains the others.

**Narrative voice**: First person, past tense, contemporary realism — until chapter 18, when the register shatters into visionary/ritual language.

**Scenes**: Approximately 65-75 discrete scenes across 16 chapters (counting distinct setting + character configurations).

**Key scene clusters**:
- **Domestic** (ch 3-5): Chris's apartment, John's house, Mallory's apartment, the restaurant
- **Journey South** (ch 5-9): Motels, rest areas, cemeteries, Henri's shop, the cabin
- **Interlude** (ch 10, 12-14): Isabella, the necromantic ritual, rebuilding in the hometown, Stephanie
- **Journey West** (ch 16-18): New Mexico, Sarah, the Prophet's house, the cave
- **Resolution** (ch 20): Hospital, empty cave, sunlight

**Gravitational landscape**: The cave (introduced in chapter 2's flash-forward prologue) has maximum narrative mass from the beginning. The entire main narrative is a single attractor basin pulling toward the cave. Intermediate scenes accumulate mass through relational and informational weight, but the destination was shown before the journey began.

### Sub-Graph 2: The Epistles

**Type**: Embedded Text — structural portals that fire automatically at chapter openings.

**Narrative voice**: Second person ("you"), elevated, prophetic, poetic. No named characters; a disembodied voice addressing the protagonist.

**Portal type**: Structural. Epistles appear at chapter openings, separated from the main narrative by a horizontal rule (`-----`). The player/reader does not choose to enter the epistle sub-graph — the system transitions them there as part of narrative rhythm.

**First appearance evolution**:
- Chapter 3: Embedded within the main narrative (Chris reads a journal entry). The epistle is a document-within-the-story.
- Chapter 4 onward: Autonomous. The epistle stands on its own, not read by any character. It has migrated from diegetic object to structural parallel voice.

**Epistle numbering**: The epistles are numbered internally (Epistle Five, Nineteen, Seventeen, Seven, Eighteen, Twenty, Thirteen, Sixteen, Ten, Six, Nine, Twelve, Two, Twenty-One) — NOT in the order they appear in the novel. This numbering belongs to the journal-writer's system, creating a second ordering that the reader could reconstruct.

**Function as sub-graph**: The epistles don't generate narrative events. They function as prophetic previews and thematic commentary:

| Epistle | Chapter | Foreshadows |
|---|---|---|
| Five | 3 | "Beings of Fire," a "psychopomp" — the box arrives |
| Nineteen | 4 | Lost lover, sleeping children — the journey begins |
| Seventeen | 5 | Blindness, absence, weariness — Chris's breakdown |
| Seven | 6 | An angel rising on a hill — the stone angel scene |
| Eighteen | 8 | The grail in darkness among serpents — baptism memory |
| Twenty | 9 | The falcon, the wind — Chris's ecstatic communion |
| Thirteen | 10 | "Beneath your feet now there is no more path" — the collapse |
| Sixteen | 12 | Fire, communion, rebirth — the necromantic ritual |
| Ten | 13 | Illusion, the nature of gods — rebuilding and the dream |
| Six | 14 | "You have departed from the path" — normalcy destroyed |
| Two | 16 | "Hasten your steps" — journey to New Mexico |
| Nine | 17 | "All has been prepared" — entering the cave |
| Twelve | 18 | "With my dagger, I submit" — the ritual |
| Twenty-One | 20 | "It is finished" — resolution |

**Cascade semantics**: The epistles cascade thematically, not informationally. They do not reveal facts that resolve event DAG conditions. Instead, they create emotional approach vectors — they prime the reader/player's interpretive state for the chapter that follows.

### Sub-Graph 3: The Fairy Tale — "The Very Long Journey"

**Type**: Embedded Text with characteristics of both Frame Story and Prophetic Vision.

**Narrative voice**: Third person, fairy tale register ("the boy"). Simple, archetypal language. Except for two POV-shift interludes (chapters 11 and 15) where an omniscient narrator reveals scenes the boy doesn't witness.

**Scenes**: Approximately 30-35 discrete scenes across 4 standalone chapters plus 2-3 dream sequences.

**Fairy tale arc**:
1. **Chapter 7**: The boy's sister vanishes after a dream. A man carries her away. The boy begins pursuit. Travels downstream by boat. Finds the village where the sister was brought. The healer cannot help — her condition is spiritual, not physical. The boy sets out alone across the mountains.
2. **Chapter 11**: The boy crosses mountains (aided by a hawk, surviving a mountain lion). Crosses grasslands. His pack is stolen by the man in black. The old woman takes him in. Fire-scrying vision shows sand, cave, seed, sleeping girl.
3. **Chapter 15**: The boy and old woman travel to a desert city, then to a cave where the sister sleeps beside a spring. The old woman says the sister's blood must water a seed. The boy throws himself on the knife instead. His blood waters the seed.
4. **Chapter 19**: The boy wakes, healed, adopted by a family. Returns to the cave to tend the plant. Eventually sees the white-haired woman (the goddess) in a market. Buys a purple bell-blossom. The flower-seller names it: "Vretil."

**Key structural properties**:
- The fairy tale is **predictive**, not merely allegorical. The fire-dream in chapter 11 produces images that the fairy tale fulfills in chapter 15. Chris's mirror-vision in chapter 12 shows scenes from the fairy tale's climax.
- The fairy tale contains **internal POV shifts** (chapters 11, 15) where the narrative leaves the boy to show the old woman's private conversations. This gives the fairy tale its own information boundaries.
- The fairy tale's climax (chapter 15) structurally mirrors and predicts the main narrative's climax (chapter 18): both involve self-sacrifice in a cave, blood watering a plant, the old woman's "You have done well."

---

## Entity Manifests: Cross-Layer Mappings

### Primary Mappings

| Main Narrative | Fairy Tale | Mapping Type | Evidence |
|---|---|---|---|
| **Chris** | **The Boy** | Allegorical Projection | Same quest structure, same sacrifice, same wound (chest stab in both). In chapter 20, Chris enters the fairy tale as the boy within a dream. |
| **Mallory** | **The Sister** | Allegorical Projection | Lost to supernatural forces, unreachable, "has been of great service." The dream-reunion in chapter 20 makes the mapping explicit: the sister calls Chris "brother." |
| **The Prophet** | **The Old Woman** | Shared Entity | The strongest cross-layer entity. Both orchestrate the protagonist's vulnerability, both lie while genuinely helping, both are present at the sacrifice, both say "You have done well." The old woman IS the Prophet across narrative layers. |
| **The Man in Black** | **The Man in Black** | Shared Entity | Same figure in both layers. Steals the boy's pack in the fairy tale; brings Chris to the hospital in the main narrative; argues with the Prophet about killing Chris (ch 18 recovered memory). |
| **John** | **The Hunter (ch 11)** / **Dream-John (ch 5)** | Loose Allegorical | Both are warm, paternal, protective figures who equip the protagonist but cannot accompany them. John warns Chris; the hunter provides supplies; Dream-John warns against the wood. |
| **Isabella** | **The Woman with Flowers (ch 15)** | Loose Allegorical | Both are helpers who provide the critical object (glass datura / datura seeds). Both live in houses associated with flowers and beauty. |
| **Henri** | **The Boatman (ch 11)** | Loose Allegorical | Both are helper-ferryman figures who appear at transitional moments and aid passage. |

### Bridge Objects

Objects that exist across multiple layers, functioning as symbolic cascade mechanisms:

| Object | Main Narrative | Fairy Tale | Epistles | Function |
|---|---|---|---|---|
| **The Datura Flower** | Isabella's glass flower; Chris's dream talisman (ch 13) | The purple bell-blossom; the seed planted in the cave | Referenced as sacred plant | Primary cross-layer entity. Exists in all three layers simultaneously. |
| **The Cave** | The physical cave in New Mexico | The cave where the sister sleeps | "The womb," "the birthing chamber" | Setting bridge — same location manifests in all layers |
| **The Knife/Dagger** | The snake-turned-knife (ch 2); Chris's self-stabbing (ch 18) | The old woman's knife; the boy's interposition | "With my dagger, I submit" (Epistle 12) | Instrument of sacrifice across all layers |
| **The Photographs** | The physical objects driving the mystery | Not present in fairy tale | Not present in epistles | Main-narrative-only but structurally central |

### The Mallory Problem: Entity State Across the Reality Boundary

Mallory's entity state is the most complex in the novel and the most challenging for the system to model. She exists in multiple states simultaneously:

1. **Mallory-alive** (ch 3-9 as perceived): Chris's sister, traveling with him, arguing, analyzing, navigating
2. **Mallory-dead** (ch 10 revelation): She committed suicide two months before the journey. Everything in states 1 was hallucination or haunting.
3. **Mallory-spirit** (ch 12 ritual): Appears in Isabella's black mirror, speaks prophecy through Chris's mouth
4. **Mallory-as-sister** (fairy tale): The sleeping girl in the cave, "has been of great service"
5. **Mallory-as-reunion** (ch 20 dream): The girl who calls Chris "brother" in the fairy tale's resolution

The system must represent this as: a single entity with multiple simultaneous projections, some of which are retroactively revealed to be of a different kind than initially presented. The ProvisionalStatus model (Hypothesized -> Rendered -> Committed) applies here: Mallory-alive was Rendered during chapters 3-9 but never validly Committed — the commitment fails when the truth emerges in chapter 10. Her state reverts to a grief-projection.

For the Storykeeper, this means the information boundary model must support **retroactive reframing** — the ability to reclassify an entity's presence from "shared physical entity" to "projected entity (grief hallucination)" after the fact, with cascading implications for every scene she appeared in.

---

## Portal Definitions

### Epistle Portals

```
Portal: Epistle Entry
    type: Structural
    trigger: Chapter boundary (automatic)
    entry_approach_vectors: None — epistles are unconditional
    exit_conditions: Completion (epistle ends)
    boundary_marker: Horizontal rule (-----)
    cascade_on_exit:
        type: Thematic priming
        weight: 0.0 (informational), 0.7 (emotional/thematic)
        mechanism: Sets interpretive frame for the chapter that follows
    re_enterable: Yes (re-reading)
    player_role: Observer (no agency within epistles)
```

### Fairy Tale Portals — Standalone Chapters

```
Portal: Fairy Tale Entry (Chapters 7, 11, 15, 19)
    type: Structural
    trigger: Narrative rhythm (every 4th chapter)
    entry_conditions: None — structural, automatic
    exit_conditions: Completion (chapter ends)
    boundary_marker: Chapter break (new chapter, new register)
    cascade_on_exit:
        type: Predictive + thematic + emotional
        weight: 0.3 (informational — prophetic), 0.8 (emotional), 0.6 (thematic)
        mechanism: Fairy tale events create approach vectors for parent scenes;
                   symbolic bridge objects carry state between layers
    re_enterable: Yes
    player_role: Observer (third-person fairy tale; no player agency)
    note: The fairy tale's own internal POV shifts (ch 11, 15) create
          information asymmetry within the sub-graph — the reader knows
          things the boy does not.
```

### Fairy Tale Portals — Dream Sequences

```
Portal: Dream Bridge (Chapters 5, 13)
    type: Triggered
    trigger: Chris falls asleep at a narratively significant moment
    entry_conditions: Sleep + emotional state threshold
    exit_conditions: Waking
    boundary_marker: Prose register shift (realistic → archetypal)
    cascade_on_exit:
        type: Emotional residue + informational (symbolic)
        weight: 0.2 (informational), 0.6 (emotional)
        mechanism: Dream imagery persists as uninterpreted symbols
                   (the purple flower, the wolf/raven/lamb);
                   objects from dreams persist into waking (ch 13: Chris
                   wakes holding the glass datura)
    re_enterable: No (dreams are singular events)
    player_role: SameCharacter (Chris experiences as himself, but in
                 fairy-tale-like landscape)
```

### The Final Merger Portal (Chapter 20)

```
Portal: Convergence Entry
    type: Triggered + Structural
    trigger: Chris falls asleep in the cave after the ritual
    entry_conditions: Post-ritual state; physical exhaustion; empty cave
    exit_conditions: Waking — but the exit dissolves the boundary entirely
    boundary_marker: None — the transition is seamless
    cascade_on_exit:
        type: Complete (all layers merge)
        weight: 1.0 (all dimensions)
        mechanism: Chris IS the boy; the sister IS Mallory; the fairy tale
                   resolves the main narrative. The final paragraph is
                   written in present tense, in a voice that belongs to
                   all three layers simultaneously.
    re_enterable: No (this is the singular convergence event)
    player_role: SameCharacter / DifferentCharacter (Chris and the boy are
                 the same within the dream)
```

---

## Boundary Permeability Progression

The most significant structural finding. The sub-graph boundaries undergo a five-phase evolution:

### Phase 1: Established Boundaries (Chapters 1-5)

Epistles appear first as documents Chris physically reads (diegetic), then migrate to autonomous structural positions. The fairy tale enters only as a dream. The main narrative is firmly realistic. Each layer has its own voice, setting, and character set with no overlap.

**Permeability**: Low. Objects and characters do not cross layers. The horizontal rule and register shift are hard boundaries.

### Phase 2: Bleeding Boundaries (Chapters 6-10)

The boundaries begin to leak. Mallory generates epistle-register language during a prophetic episode (ch 6). The fairy tale occupies its first full standalone chapter (ch 7). Chris's glossolalia in the wind (ch 9) uses prose that echoes the epistles' cadence. The baptism memory (ch 8) has ritual/epistle qualities within the main narrative.

Most significantly, the revelation that Mallory is dead (ch 10) collapses the boundary between reality and delusion within the main narrative itself — a sub-graph-level event where the narrator's reliability was itself a kind of sub-graph.

**Permeability**: Medium, increasing. The fairy tale is still contained in its own chapter, but main-narrative prose is absorbing epistle characteristics.

### Phase 3: Symmetric Permeability (Chapters 11-15)

The fairy tale bookends (ch 11, 15) create a symmetric structure around the main narrative triptych (ch 12-14). The dream in chapter 13 explicitly bridges layers — Chris carries the datura flower through a fairy-tale landscape. Characters in one thread reference entities in another (Mallory's prophecy names "the old woman"). The fairy tale's events are predictive: the fire-dream (ch 11) produces images fulfilled in ch 15; Chris's mirror-vision (ch 12) shows the fairy tale's climax.

The datura flower now exists in all three layers simultaneously.

**Permeability**: High, bidirectional. Information flows in both directions: fairy tale predicts main narrative; main narrative imagery appears in fairy tale.

### Phase 4: Boundary Collapse (Chapters 16-18)

Chapter 18 shatters the boundaries. Chris's vision-cascade folds every female character into the old woman — Isabella, Stephanie, Mallory, Sarah all transform into her. The cave sequence shifts to an elevated register that is neither epistle nor standard narration nor fairy tale, but a third voice. The nine nymphs (three maiden, three matron, three crone) literalize the archetype.

**Permeability**: Boundaries shattered. The main narrative has become visionary; the distinctions between threads are semantic, not structural.

### Phase 5: Convergence (Chapters 19-20)

Chapter 19 is pure fairy tale, but its imagery (the chest wound, the cave, the plant, the goddess) has converged completely with the main narrative. In chapter 20, the epistle opens as coda ("It is finished"), the main narrative resumes (hospital, recovery), then Chris falls asleep in the cave and enters the fairy tale as its protagonist. The final paragraph is written in present tense, containing all three threads simultaneously.

**Permeability**: Complete merger. The three sub-graphs have collapsed into a single narrative point. The realized graph is a funnel — three parallel tracks narrowing to convergence.

### Permeability as a Modelable Property

This progression suggests permeability should be a **dynamic property** of sub-graph boundaries, not a static configuration:

```
BoundaryPermeability {
    sub_graph_pair: (SubGraphId, SubGraphId),
    permeability_curve: Vec<(TurnId, f32)>,  // permeability over time
    current_value: f32,                       // 0.0 = impermeable, 1.0 = merged
    drivers: Vec<PermeabilityDriver>,         // what causes permeability to change
}

PermeabilityDriver =
    | NarrativeProgression(f32)   // automatic increase as story advances
    | EventDriven(EventId)        // specific events that shift permeability
    | EntityCrossing(EntityId)    // entities appearing in both layers
    | ObjectBridge(ObjectId)      // symbolic objects existing in both layers
    | RegisterBleed(SceneId)      // narrative voice absorbing other-layer characteristics
```

---

## The Temporal Graph

### Relational Web Evolution

The most dynamic relationships, traced across the novel:

**Chris → Mallory** (the spine):
```
Ch 3-5:  Trust: 0.7, Affection: 0.9, Debt: 0.8 (caretaker burden)
         Configuration: "exhausted devotion"

Ch 6-9:  Trust: 0.5 (medication confrontation), Affection: 0.9, Debt: 0.6 (partnership)
         Configuration: "strained partnership" → "ruptured by obsession"

Ch 10:   ENTITY STATE CHANGE: Mallory revealed dead. Edge reclassified.
         All substrate values become properties of a grief-relationship,
         not a living relationship. Trust/affection persist but are
         recontextualized as communion with the dead.

Ch 12-14: Trust: N/A (spirit), Affection: 1.0, Debt: 0.9 (survivor guilt)
          Configuration: "haunted devotion"

Ch 18-20: Resolution through fairy tale reunion. Debt reduces to 0.3.
          Configuration: "released grief, accepted loss"
```

**Chris → The Prophet/Old Woman**:
```
Ch 2:     Terror + submission. Configuration: "helpless before the numinous"
Ch 3-5:   Mediated — photographs only. Configuration: "distant unease"
Ch 8-10:  Growing awareness. Configuration: "pursuing the orchestrator"
Ch 12:    Direct supernatural contact (the vision). Configuration: "overwhelmed"
Ch 16-17: Sarah reveals the network. Configuration: "resigned compliance"
Ch 18:    Pursuit through visions; every woman transforms into her.
          Configuration: "consumed by the archetype"
Ch 18 (cave): Chris refuses her death-claim: "This death is mine."
          Configuration: "defiant self-sovereignty"
Ch 20:    The old woman as nurturing mother-figure in the fairy tale.
          Configuration: "acceptance of the whole (terror + tenderness)"
```

### Narrative Mass Evolution

The cave's gravitational pull is unique in that it is established before the journey begins:

```
Scene: The Cave
  Ch 2 (flash-forward):  authored_base = 1.0 (maximum)
                          structural_modifier = 0.3 (convergence point, gates, required)
                          dynamic = 0.0 (no player state yet)
                          effective_mass = 1.3

  Ch 5 (departure):      dynamic rises to 0.1 (journey has begun)
                          effective_mass = 1.4

  Ch 10 (Mallory's death): dynamic rises to 0.5 (emotional weight surges)
                           effective_mass = 1.8

  Ch 16 (New Mexico):    dynamic rises to 0.8 (approach vector nearly satisfied)
                          effective_mass = 2.1

  Ch 17 (entering cave): SCENE ACTIVATED
                          All mass collapses into the active scene
```

No other scene in the novel approaches the cave's mass. The novel is a single-attractor story — a gravitational landscape with one basin that the narrative spirals into.

### Event DAG: The Gap

The novel's event dependency DAG has one profoundly important node:

```
EventConditionNode {
    node_id: "the_gap",
    condition_type: DiscoveryCondition,
    resolution_condition: "Chris freely chooses his own death",
    narrative_weight: 1.0,  // maximum — the entire story depends on this
    consequence: CompoundConsequence {
        effects: [
            TruthSetMutation("goddess_born"),
            InformationReveal("the_full_truth"),
            NarrativeMassAdjustment(cave, +∞),  // scene completes
        ]
    },
}
```

The orchestrators — the Prophet, Sarah, the network — could manipulate everything *except* this node. They could deliver Chris to the cave, strip away his supports, create the conditions for the ritual. But they could not make him choose. This is what Sarah calls "the gap" in the photographs: the one frame that was never taken, because it could not be predetermined.

In event DAG terms, "the_gap" has a `Requires` chain stretching back through the entire novel (every journey event, every revelation, every relationship) but its resolution depends on a free choice that no amount of precondition-satisfaction can guarantee.

---

## Implications for the Engine

### What This Case Study Reveals About the Model

**1. Sub-graph boundaries must be dynamically permeable.** Static permeability (configured once at sub-graph creation) cannot represent Vretil's five-phase progression. The permeability must be a function of narrative state — driven by events, entity crossings, and symbolic bridges.

**2. Prophetic cascade is a real pattern.** The fairy tale doesn't just influence the parent graph on-exit; it creates approach vectors for scenes that haven't been reached yet. The fire-dream in chapter 11 generates imagery that primes chapter 15's fairy tale events, which in turn predict chapter 18's main narrative climax. Cascade can flow temporally forward through symbolic resonance, not just structurally backward through exit processing.

**3. Entity state can be retroactively reclassified.** Mallory's revelation requires the system to reclassify an entity's presence from "shared physical entity" to "projected entity (grief hallucination)" after the fact. The ProvisionalStatus model supports this, but the cascade implications are substantial — every scene she appeared in must be reinterpretable.

**4. Bridge objects are a distinct entity type.** The datura flower is not a character, not a setting, not an event. It is a symbolic object that exists simultaneously across all three layers and functions as the primary cascade mechanism between them. The entity model needs a category for objects whose identity is maintained through symbolic mapping rather than physical continuity.

**5. The epistle sub-graph is a degenerate case.** It has no characters, no relational web, no setting topology, no event DAG. It is pure thematic cascade — a sub-graph whose entire purpose is to inflect the parent graph's emotional approach vectors. This is useful because it defines the minimal sub-graph: the simplest possible narrative layer is one that contributes only thematic priming.

**6. Convergence is the extreme case of permeability.** When permeability reaches 1.0, the sub-graphs merge. This isn't a failure of the model — it's a valid end-state that the system should support. The merge means: entity projections collapse (Chris IS the boy), settings unify (the cave is one place), the narrative stack flattens to a single layer, and the voice integrates all registers.

**7. The unreliable narrator creates a meta-sub-graph.** Mallory's posthumous presence is effectively a sub-graph that the reader doesn't know they're in. The "reality" Chris reports is itself a layer — a grief-projection sub-graph overlaid on the actual events. This suggests the system should support sub-graphs that the player doesn't perceive as sub-graphs until a revelation event reclassifies them.

### Additions to the Tales-Within-Tales Model

Based on this case study, the following extensions should be added to `tales-within-tales.md`:

1. **Dynamic permeability curves** with explicit drivers (events, entity crossings, object bridges, register bleed)
2. **Prophetic cascade** — sub-graph events that create approach vectors for unreached parent-graph scenes, flowing temporally forward
3. **Bridge objects** as a first-class entity type with cross-layer identity maintenance
4. **Degenerate sub-graphs** — layers with no characters or events, only thematic cascade (the epistle pattern)
5. **Convergence semantics** — formal definition of what happens when permeability reaches 1.0 and sub-graphs merge
6. **Hidden sub-graphs** — layers the player doesn't perceive as sub-graphs, revealed by reclassification events (the unreliable narrator pattern)
7. **Retroactive entity reclassification** — changing an entity's sharing mode after the fact, with cascade to all scenes in which the entity appeared

---

## The Realized Graph: Summary Statistics

| Dimension | Value |
|---|---|
| **Total scenes** | ~100-110 across all layers |
| **Root graph scenes** | ~65-75 (main narrative) |
| **Epistle sub-graph segments** | ~18-20 |
| **Fairy tale sub-graph scenes** | ~30-35 |
| **Dream bridge scenes** | ~5-6 |
| **Named characters (main)** | 9 (Chris, Mallory, John, Mike, Henri, Isabella, Stephanie, Sarah, The Prophet) |
| **Named characters (fairy tale)** | 8 (The Boy, The Sister, The Old Woman, The Hunter, The Hawk, The Man in Black, The Flower Woman, The Limping Man) |
| **Cross-layer entity mappings** | 7 primary + loose allegoricals |
| **Bridge objects** | 4 (datura, cave, knife, photographs) |
| **Boundary permeability phases** | 5 (established → bleeding → symmetric → collapsed → merged) |
| **Single attractor basin** | The cave (mass 1.0-2.1+ over the novel's progression) |
| **Critical free-choice node** | 1 ("the gap" — Chris's voluntary sacrifice) |
| **Epistle-main cascade events** | ~14 (one per chapter with epistle) |
| **Fairy tale → main predictive cascades** | ~8-10 (fire-dream, mirror-vision, climax mirror, bridge objects) |
| **Entity reclassification events** | 1 (Mallory: living → dead/projected, ch 10) |

---

## What This Document Does Not Cover

- **Branching analysis**: Vretil is linear fiction. An interactive adaptation would need to identify player agency points, alternate paths, and the attractor basin's response to deviation. This case study maps structure, not possibility space.
- **Scene-level specifications**: Individual scene specs (like the TFATD case study's per-scene YAML) are not provided. The value here is in the cross-layer structural analysis, not scene-by-scene detail.
- **AGE/Cypher representation**: How the multi-layer graph would be stored in Apache AGE with label namespaces per sub-graph is an implementation concern for the PostgresStorykeeper.
- **Authoring workflow**: How a story designer would define these sub-graphs, portals, and permeability curves in practice.

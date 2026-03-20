# Data-Driven Narrative Elicitation

*How dimensional genre modeling and LLM-driven pattern discovery produce a narrative vocabulary richer than human authoring alone*

---

## The Insight

Genre is not a category. It is a region — a position in a high-dimensional space of narrative affordances, each axis representing a continuous or categorical constraint on what stories in that space can do. This is not a novel claim; anyone who has tried to shelve a book knows that genre boundaries are approximate. But treating this literally — defining genre as a specific configuration of ~30 measurable dimensions — unlocks something we did not anticipate: the genres themselves become generative.

When you describe a genre not as "folk horror" but as a region where the locus of power is the Land, where knowledge is punished, where time is cyclical, where agency is stripped by environmental determinism, where earnestness is demanded of the antagonists — then you can ask: *what kinds of characters must exist in a space with these affordances?* Not "what archetypes are traditional in folk horror" but "what character shapes arise from these specific axis positions?"

The answer to the second question is richer, stranger, and more analytically precise than the answer to the first.

---

## The Methodology

### Genre as Dimensional Region

We defined 30 genres across the narrative space — not as a closed taxonomy but as recognizable clusters positioned along continuous and categorical axes. The dimensions emerged through iterative elicitation: an initial pass with a large language model (qwen3.5:35b, 36B parameters) produced rich descriptions that were then analyzed for dimensional gaps. The analysis revealed axes we hadn't anticipated (epistemological stance, locus of power as a ranked list, the distinction between surface irony and structural irony) and relationships we hadn't predicted (tonal dimensions cluster into correlated subspaces; most genres are "constraint layers" that modify base dimensions rather than standalone regions).

The key methodological choice was to ask the model for *commentary and suggestions* alongside its descriptive output. This turned each elicitation pass into a research conversation: the model didn't just describe folk horror, it noted that "exclusions are more definitional than inclusions" and that "earnestness vs. sincerity needs to be split into surface behavior and structural payoff." These observations drove the dimensional expansion in subsequent passes.

### Primitives Discovered, Not Prescribed

With 30 genre regions described across their full dimensional profile, we faced the question of how to generate the narrative primitives — archetypes, settings, dynamics, scene profiles, goals — that would populate the engine's reference library. The conventional approach would be to author a list of ~30 familiar archetypes (The Mentor, The Trickster, The Shadow) and describe how each manifests in each genre.

We rejected this. Instead, we inverted the pipeline:

**Phase 1 — Per-genre extraction.** For each of the 30 genres, we asked the model: *given this genre's dimensional description, what 5-8 character archetypes are essential or distinctive to this genre? Ground your answer in the genre's axes — its locus of power, its epistemological stance, its exclusions, its state variables.* The model received only the genre's own dimensional description, not a seed list.

**Phase 2 — Cluster synthesis.** We grouped the 30 genres into six natural clusters (horror, fantasy, sci-fi, mystery/thriller, romance, realism/gothic/other) and asked the model to synthesize the per-genre extractions: *merge variants of the same archetype, keep genuinely distinct archetypes separate, flag what's universal vs. cluster-specific vs. genre-unique.*

The result was 197 per-genre archetypes synthesized into 60 cluster-level archetypes — none of them "The Mentor" or "The Shadow."

### What Emerged

The archetypes that arose from dimensional analysis are not conventional character types with genre paint applied. They are structurally novel descriptions of characters who *must* exist because of the forces operating in their genre's narrative space.

**The Earnest Warden** (folk horror) is not "the villain." The model derived this archetype from the intersection of the genre's thematic dimension (Power: Stewardship) and its tonal dimension (Earnestness). Folk horror, it observed, "rejects the Cult Leader trope (cynical power-hungry) in favor of a system where the community are stewards of a pact with the earth." The Warden holds high Authority and low Openness to outsiders, but high Warmth to the community. Their distinguishing tension is Humanity vs. Duty: "they are a loving neighbor and a willing executioner." This is not a description a seed list would produce. It is a description that *the dimensional constraints require*.

**The Heat-Marker** (cyberpunk) emerged from the genre's state variable system. In cyberpunk, "Heat" (system attention) is a tracked resource that directly correlates with the locus of power (corporations/surveillance). The model identified a character who "must act to survive (High Agency), but every action increases their Heat (Environmental Determinism)." The tension is that "inaction is also a trap — the System eventually catches the idle." This archetype is not "the fugitive" — it is a character whose existence is defined by the feedback loop between agency and surveillance, a pattern the model extracted from the genre's dimensional profile.

**The Hearthwarden** (cozy fantasy) emerged from the genre's explicit exclusion of macro-agency. The model noted that "unlike the Chosen One in High Fantasy who saves the world, the Hearthwarden's agency is defined by Micro-Agency. They are designed to manage Sanctuary Integrity (the home) rather than geopolitical stability." The distinguishing tension is Control vs. Helplessness: "they can brew a potion that cures a neighbor's headache but cannot cure the drought that is killing the town's crops." And critically: "A Hearthwarden who becomes a King would shift the genre to Epic Fantasy." The archetype carries the boundary condition of its genre within its definition.

The same approach applied to settings produced results equally unexpected. When asked what spatial environments folk horror requires, the model produced **The Throat of the Land** — a well or water source described as "the physical point of connection between the human and the earth's will." Its atmospheric note: "the sound of water is wrong." Its communicability: "it feels like a mouth." When horror comedy was asked the same question, it produced **The Suburban Home That Treats the Apocalypse as a Maintenance Issue** — a setting whose mood is "Annoyed Dread" and whose characters "treat the supernatural as a logistical problem (plumbing, electrical, structural)." Working-class realism produced **The Ledger Kitchen** — where "the most critical negotiation happens: the negotiation of money between the wages earned and the bills due," and "a cleared table means solvency; a cluttered table means crisis."

These are not settings an author would brainstorm from a blank page. They are settings that the genre's dimensional constraints demand.

---

## What This Demonstrates

### The Renaming Problem Reveals Universal Mechanics

When we inventoried the axes across all 30 genres and 197 archetypes, we discovered that the same underlying mechanic appears under different names in different genres. The state variable we call "psychological integrity" manifests as Sanity in cosmic horror, Hero's Burden in high fantasy, Hubris in classical tragedy, Dignity in working-class realism, Stability in cyberpunk, and Perception in magical realism. The mechanic is the same — a resource that depletes under pressure with genre-specific consequences when exhausted. But the genre context determines whether depletion looks like madness, moral collapse, prideful overreach, or loss of dignity.

This is not a cataloging convenience. It is evidence that narrative mechanics are genre-independent while their expression is genre-dependent. A canonical identifier (psychological_integrity) enables the engine to apply universal update logic; a genre-specific label (Hubris) preserves narrative flavor. The narrator says "Hubris"; the engine sees `PsychologicalIntegrity`.

Eight canonical state variables account for the vast majority of scene-by-scene dynamics across all 30 genres. Genre-specific variables (Heat in cyberpunk, Sanctuary Integrity in cozy fantasy, Hubris in classical tragedy) are either aliases of these eight or genuine extensions activated by specific genre features.

### Cluster Synthesis Reveals Structural Families

The 60 cluster-level archetypes organize into roughly 8-10 structural families that repeat across all six clusters:

- **The Keeper/Warden** — custodian of rules or sacred knowledge, appearing in every cluster from the Earnest Warden (folk horror) to the Sanctuary Warden (cozy fantasy) to the Institutional Guardian (nordic noir)
- **The Seeker** — driven to uncover truth, rewarded in some genres (hard sci-fi, mystery) and punished in others (cosmic horror, folk horror)
- **The Anchor** — the relational grounding figure, manifesting as Social Anchor (horror), Community Anchor (fantasy/realism), Domestic Anchor (mystery), Reciprocal Anchor (romance)
- **The Vessel** — the character acted upon by forces larger than themselves, from Environmental Determinism (horror) to Systemic Victimhood (noir) to Cursed Legacy (romance)

These families are not predetermined categories. They are emergent patterns — structural roles that genre after genre independently requires because of the underlying narrative mechanics. The cluster synthesis identifies them; it does not invent them.

### Commentary as Research Instrument

The most unexpected value came from the model's commentary and suggestions — structured reflection we requested alongside every extraction. The horror cluster's commentary observed: "The most striking pattern is the **Inversion of Agency**. In most genres, Agency is a resource to be spent for victory. In the Horror Cluster, Agency is frequently a resource to be spent for *acceleration* (The Vessel, The Epistemic Seeker) or *survival through luck* (The Serendipitous Survivor)."

This is an analytical insight about how horror *works* as a narrative system — not a description of what horror contains, but an observation about the structural relationship between agency and outcome that distinguishes horror from other genre clusters. The model arrived at it not by being told what horror is, but by synthesizing patterns across three genre extractions it had produced from dimensional descriptions.

---

## What This Enables

This data-driven approach to narrative pattern discovery has specific consequences for the storyteller engine:

**The narrator receives genre-grounded reference material, not generic templates.** When assembling context for a turn in a folk-horror scene, the narrator doesn't receive "The Mentor" with a folk-horror skin. It receives The Earnest Warden — a character whose warmth-axis position, authority source, and distinguishing tension are specific to the forces operating in this genre's narrative space.

**The engine can query across genres by canonical mechanics.** "What is this character's psychological integrity?" works regardless of whether the story is horror (Sanity), tragedy (Hubris), or working-class realism (Dignity). The canonical state variable system enables unified update logic while the genre label preserves narrative voice.

**New genres can be added without redesigning the primitive vocabulary.** Because primitives are discovered from dimensional descriptions rather than authored per-genre, adding a new genre means describing its position on the 30 axes and running the extraction pipeline. The archetypes, settings, and dynamics that emerge will be structurally grounded in the new genre's specific affordances, and the cluster synthesis will identify where they connect to existing families.

**The clustering is descriptive, not prescriptive.** The structural families (Keeper, Seeker, Anchor, Vessel) are organizational indices for findability and cross-reference, not categorical constraints. New data, new genres, or different analytical lenses could produce different clustering topology. The per-genre extractions — the 197 individual archetype descriptions grounded in specific genre axes — remain the primary data. The clusters are a map of the territory, not the territory itself.

---

## Epistemological Honesty

There is something we should name directly. The narrative landscape we are mapping is, in part, an articulation of the foundational training data of the model we used for elicitation and the shape of its reasoning and pattern-finding mechanics as applied to narrative space. A different model, different training data, or different prompts might generate different results. There is no view from nowhere.

This does not invalidate the work. It contextualizes it. The model's training data encompasses a vast corpus of human narrative — literary criticism, genre theory, published fiction, screenwriting guides, reader reviews, cultural commentary. What it produces when asked to analyze genre from dimensional descriptions is a sophisticated synthesis of how narrative structure has been understood, practiced, and discussed across that corpus. The archetypes it discovers are not arbitrary; they are patterns that recur because the underlying narrative mechanics recur.

But we should not mistake the map for the territory. The Earnest Warden is not a Platonic form discovered in the genre's DNA. It is one model's articulation of a structural necessity that the dimensional constraints imply. A different model might name it differently, emphasize different tensions, or identify a variant we missed. The value is in the methodology — dimensional description → constrained extraction → cluster synthesis → human review — not in any particular output's claim to completeness.

What we can say with confidence is that the outputs are remarkably rich, analytically insightful, and narratively compelling. They consistently demonstrate the kind of structural reasoning about genre that would take a human critic considerable effort to produce, and they do it grounded in explicit dimensional constraints rather than implicit taste. The commentary sections regularly surface observations that surprised us — the Inversion of Agency in horror, the boundary condition that a Hearthwarden who becomes a King exits the genre, the insight that folk horror's Warden is a "loving neighbor and willing executioner" rather than a villain.

This is what data-driven narrative elicitation makes possible: not replacing human judgment about stories, but giving human judgment richer material to work with.

---

## Relationship to Other Foundation Documents

This document extends the design philosophy's concept of **N-dimensional narrative space** from a structural principle to an operational methodology. The character modeling document describes characters as tensors with context-dependent activation; this work shows how tensor configurations can be *discovered* from genre analysis rather than authored from theory. The world design document's emphasis on material conditions and cultural forms producing coherence is mirrored here in how genre affordances produce character types and spatial environments.

The methodology also operationalizes the principle of **imperfect information** in a new way. No single genre extraction has the complete picture of an archetype. The Keeper appears differently in every genre, and the cluster synthesis captures the family without collapsing the genre-specific variants. The narrator receives the genre-specific variant, not the canonical family — imperfect information by design, richness from the interplay of partial perspectives.

---

*March 2026. Data generated using qwen3.5:35b (36B parameters, Q4_K_M quantization) via Ollama, from 30 genre region descriptions totaling ~438KB. Archetype corpus: 197 per-genre extractions (384KB) synthesized into 60 cluster-level archetypes (92KB). Settings corpus: 211 per-genre extractions (526KB). Pipeline tooling: `tools/narrative-data/` with append-only JSONL event tracking and manifest-based staleness detection.*

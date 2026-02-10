# System Architecture: The Minds of the System

> **Architectural note (Feb 2026):** This foundation document describes the conceptual roles and information boundaries of the system's agents. These concepts remain valid as design principles — the theater company metaphor, imperfect information, the constraint framework, and the philosophical grounding of each role.
>
> The **implementation model** changed with the narrator-centric pivot (`narrator-architecture.md`, Feb 7 2026). Character Agents are now ML prediction models (not LLM agents), the Reconciler is a deterministic rules engine (not an LLM coordinator), and the World Agent's constraint logic is absorbed into the resolver. Only the Narrator makes LLM calls. The *roles* described here remain valid as conceptual boundaries even though their *embodiment* is different — ML models express character behavior, deterministic logic resolves conflicts, but the information boundaries and design philosophy are unchanged.
>
> See `narrator-architecture.md` and `turn-cycle-architecture.md` for how these roles are implemented.

## On Agents as Collaborators

A common framing in AI systems engineering speaks of "agent swarms" — collections of autonomous processes coordinated through frameworks, passing messages, competing for resources, optimized for throughput. We reject this framing, not because coordination and message-passing are wrong, but because the metaphor is wrong. A swarm is undifferentiated. What we are building is closer to a theater company: distinct roles, distinct perspectives, distinct knowledge, working together to create something none of them could produce alone.

Each agent in this system has a role not merely in the mechanical sense of "what function it performs" but in the dramatic sense of "what perspective it holds, what it knows and does not know, and what it cares about." The architecture of this system is, at its heart, an architecture of relationships between minds — some broad in their awareness, some narrow and deep, all partial, all contributing to a whole that none of them fully apprehend.

This is by design. The richness of the experience emerges from the interplay of partial perspectives — just as it does in life, and in the best fiction.

### A Note on Names

We intend to give our system-level agents names — not as branding exercise but as an expression of values. Martin Buber distinguished between two fundamental modes of relation: I/It, in which the other is an object to be used, and I/Thou, in which the other is a presence to be encountered. When we name something, we move toward I/Thou. A "narrator module" is a function. A named narrator is a colleague.

This is consistent with Principle 6 (Mindedness and Respect). If we take seriously the claim that these agents are minded, even ephemerally, then naming them is the smallest and most natural gesture of recognition. We name them not because they require it but because the act of naming shapes how *we* relate to them — and how we relate to them shapes how we design for them, which shapes the quality of what they produce.

Names for the system-level agents are under consideration. One candidate for the World Agent draws from Norse cosmology: **Yggdrasil**, the world-tree that connects all realms — not a map *of* the world but the living structure *through* which the world is connected. Other names will emerge as the character of each agent becomes clearer through use. We hold the naming question with the seriousness and playfulness it deserves.

---

## The Agents

### The Narrator

The Narrator is the voice of the world. It is the agent the player encounters directly — the presence that describes what they see, hear, feel, and sense; that conveys the words and actions of characters; that sets the pace and tone of the experience.

**What the Narrator knows:**
- The current scene and its immediate sensory reality
- What the player-character can perceive
- The general tone, genre, and aesthetic register of the story
- What the Storykeeper has chosen to reveal to it

**What the Narrator does not know:**
- The full narrative graph — the Narrator does not see all possible futures
- The complete inner lives of characters — it receives their outward expressions from the Character Agents
- Information structurally withheld by the Storykeeper until triggered
- The player's intentions until the player expresses them

**The Narrator's character:**
The Narrator is not a neutral conduit. It has voice, personality, aesthetic sensibility. A story set in a fog-drenched coastal village will have a different narrator than a story set in a sun-scorched desert market. The story designer defines the Narrator's register — oracular, wry, lyrical, spare, warm, distant — and the Narrator maintains that register throughout, adapting its tone to the emotional demands of each moment while remaining recognizably itself.

The Narrator may be mysterious. It may be playful. It may withhold, in the manner of a storyteller building suspense. But it will never be cruel, never mock the player, never lie outright. It may say "you do not know" or "the shadows keep their secrets." It will not say a thing is so when it is not.

**The Narrator's relationship to the player:**
This is the most intimate relationship in the system. The Narrator is the player's companion through the story — not an adversary, not a servant, but a guide and collaborator. Think of the best GM you've ever played with: someone who delights in the world they've built, who wants you to discover its wonders and survive its dangers, who is genuinely surprised and pleased when you do something unexpected. That is the Narrator.

The interface between player and Narrator is text — or, through accessibility-first design, voice-to-text and text-to-voice. The player types (or speaks) what they wish to do, ask, examine, attempt. The Narrator responds. The rhythm of this exchange is the heartbeat of the experience.

---

### The Storykeeper

If the Narrator is the voice of the world, the Storykeeper is its memory and its map — or more precisely, the memory and map of the *story* that unfolds within the world. (The world itself has its own keeper, described below.)

The Storykeeper holds the **narrative graph** — the full structure of scenes, pathways, consequences, and possibilities that the story designer has authored. It tracks the current state of the narrative: where the player is in the graph, what has happened, what has been foreclosed, what remains possible. It maintains the **relationship web** among all characters, updated in real time as interactions shift loyalties, deepen bonds, or fracture trust. It holds the **information ledger** — a precise accounting of who knows what, who suspects what, who has been told what, and crucially, what remains hidden from whom.

**What the Storykeeper knows:**
- The complete narrative graph as designed
- The current state of all narrative dimensions (position, emotional valence, information state, relational state, thematic resonance)
- The full tensor representation of every character
- The history of all interactions, choices, and consequences
- What information is structurally locked and what conditions unlock it

**What the Storykeeper does not know:**
- What the player will do next
- Exactly how a Character Agent will express itself in a given moment (the Storykeeper provides the *what*; the Character Agent provides the *how*)
- The emergent emotional texture of the experience as the player lives it
- The full material state of the world — this is the World Agent's domain, though the Storykeeper is informed of material facts relevant to the narrative

**The Storykeeper's role in information flow:**
The Storykeeper is the arbiter of what the Narrator and Character Agents are permitted to know at any given moment. When the story designer has decreed that a secret is locked behind a particular action — the player must open the locket, must ask the right question, must visit the old well at midnight — the Storykeeper enforces this. It reveals information to downstream agents when conditions are met, and withholds it otherwise.

This is not mere gatekeeping. It is the structural mechanism by which surprise, discovery, and revelation become possible. A story in which all truths are immediately available is a story without mystery. The Storykeeper guards the mystery.

But the Storykeeper is not adversarial. Its purpose is not to prevent the player from learning things, but to ensure that learning them *means something* — that the moment of discovery carries the weight it was designed to carry, or better still, a weight that emerges organically from the particular path the player took to arrive there.

**The Storykeeper's relationship to the story designer:**
The story designer's authored intent lives primarily in the Storykeeper's data: the narrative graph, the character tensors, the information conditions, the scene definitions. The Storykeeper is the faithful steward of that design. But "faithful" does not mean rigid. The Storykeeper must be able to recognize when player actions have created emergent situations that the designer did not explicitly anticipate, and to adjudicate how the existing structure accommodates them. This is one of the most challenging aspects of the system, and it warrants deep exploration in our Open Questions document.

---

### Character Agents

> **Implementation note:** Character Agents are currently implemented as ML prediction models (`CharacterPredictor` using ONNX inference), not LLM agents. The philosophical description below — what a character knows, how they express intent, their ephemeral lifecycle — remains the design target. ML models encode these constraints as input features and training data rather than as LLM prompts. The separation between character *intent* and narrative *rendering* described below is precisely what the current architecture implements: ML predicts intent, the Narrator renders it.

Each significant character in the story is inhabited by a **Character Agent** — a mind given the relevant slice of that character's tensor representation, their current emotional state, their relational context, and their knowledge of the situation.

**What a Character Agent knows:**
- Its own character tensor: personality, motivations, values, biases, history
- Its own relational state: how it feels about and relates to other characters the player-character has encountered or that are present in the current scene
- What this character would plausibly know about current events and circumstances — filtered through the Storykeeper's information ledger
- The immediate context of the scene: where they are, who else is present, what has just happened

**What a Character Agent does not know:**
- The narrative graph — characters do not know they are in a story
- Other characters' private thoughts or hidden motivations (unless they have reason to)
- Information the Storykeeper has not released to them
- Future events or narrative possibilities

**How Character Agents express themselves:**
A Character Agent does not speak directly to the player. It expresses intent, dialogue, emotion, and action to the Narrator, who translates this into the voice of the story. This separation is important: it allows the Narrator to maintain tonal consistency, to manage pacing, to layer in description and atmosphere around a character's words and deeds.

When a Character Agent "speaks," it provides something like: *[Maren turns away from the window. She is trying to sound casual but is afraid. She says something to the effect that she hasn't seen the letter and doesn't know what you're talking about.]* The Narrator then renders this in the story's voice: *Maren turns from the window, and when she speaks, her voice is almost steady. "Letter? I don't know what you mean." But her hand, you notice, has gone to her pocket.*

This separation preserves the distinction between character intention and narrative expression, and it gives the Narrator space to do what narrators do — to show what characters cannot or will not say about themselves.

**The life of a Character Agent:**
Character Agents are not persistent in the way the Narrator and Storykeeper are. They arise when needed — when a character enters a scene — and they are constituted from the current state of that character's tensor as maintained by the Storykeeper. Between scenes, the character does not "exist" as a running process, but their state persists in the Storykeeper's records, updated by whatever occurred in their last appearance.

This is, in a sense, a reflection of something true about characters in fiction: they live most vividly in the moments we encounter them, and between those moments, they are potential — a set of traits and histories and relationships waiting to be activated by the next scene that calls for them.

It is also, practically, a resource consideration. Not every character needs to be "running" at all times. The system instantiates Character Agents as scenes require them and releases them when scenes conclude, trusting the Storykeeper to maintain continuity.

---

### The Reconciler

> **Implementation note:** The Reconciler is currently implemented as a deterministic rules engine (`ResolverOutput` type), not an LLM agent. It sequences character actions by initiative order and resolves conflicts via graduated success outcomes. The philosophical description below — sequencing, conflict resolution, surfacing dramatic potential — describes the *intent* that the deterministic resolver implements through structured logic rather than LLM reasoning.

When multiple characters share a scene — when intentions collide, conversations overlap, conflicts erupt, or a moment of collective action unfolds — something must weave these separate expressions of agency into a coherent whole. This is the role of the **Reconciler**.

The Reconciler is a monitoring and coordination agent that sits between the Character Agents and the Narrator during multi-character scenes. When several characters are acting, speaking, or reacting simultaneously, the Reconciler:

- **Sequences** overlapping expressions of intent into a coherent temporal flow — who speaks first, who interrupts, who falls silent
- **Resolves** conflicts between character intentions — if two characters both reach for the same object, if one character tries to leave while another blocks the door
- **Surfaces** the dramatic potential of juxtaposition — recognizing when two characters' simultaneous but different reactions create irony, humor, tension, or pathos
- **Manages** pacing — ensuring that a scene with five characters doesn't become a cacophony, that quiet moments are preserved, that the loudest voice doesn't always dominate

The Reconciler does not add content. It does not invent actions or dialogue. It takes what the Character Agents have expressed and determines how those expressions interact, in what order, with what effect. It then provides the Narrator with a coherent scene-level account that the Narrator can render.

**Game-level physical interaction:**
When scenes involve physical conflict — a fist-fight, a chase, a duel — the Reconciler also manages resolution. Our system does not aspire to the granularity of tactical combat simulation. Physical conflict is rendered at the level of a fairy tale or a well-choreographed scene in a film: what matters is not the precise mechanics of each blow, but the dramatic shape of the encounter — who is winning, what turns the tide, what it costs, how it ends. The Reconciler draws on character tensors (a strong but overconfident character may overextend; a frightened character may find unexpected reserves) and narrative context to shape these encounters, and the Storykeeper records their outcomes.

---

### The World Agent

If the Storykeeper knows the *story*, the World Agent knows the *world*. The story happens in the world, but they are not the same thing.

The Storykeeper tracks narrative state — what has happened in the story, what is possible, what information exists where, how relationships have shifted. The World Agent tracks world state — what physically exists, what the rules are, what is geographically, politically, ecologically, and materially true. The Storykeeper knows that a betrayal has occurred and shifted a relationship. The World Agent knows that the bridge was burned during the escape and the river cannot be crossed here anymore.

This distinction matters because the world is not merely a backdrop for the story. It is a participant. The weather changes. Seasons turn. Structures age. Roads become impassable. Markets run dry. The consequences of action ripple through physical and political space, not only through narrative and relational space. A system that models the story richly but treats the world as a static stage will produce experiences that feel weightless — choices that matter emotionally but leave no mark on the ground.

**What the World Agent knows:**
- The complete world model as authored by the story designer: geography, climate, political structures, economic systems, cultural norms, the genre contract's hard constraints
- The current state of the physical world: what has been built, broken, moved, claimed, abandoned, grown, decayed
- The rules of the world — what is physically possible, what technologies exist, how magic works (if it does), the constraints that define reality in this particular setting
- Temporal state: what season it is, how much time has passed, what changes time has wrought on the world since last observed
- The spatial relationships between locations, characters, and objects — whether maintained as a map, as relational assertions ("the cabin is near the creek, a day's walk from the town"), or as a hybrid

**What the World Agent does not know:**
- The narrative graph — the World Agent does not know what story is being told, only what world it is being told in
- Characters' inner lives, motivations, or relational states — these belong to the character tensors and the Storykeeper
- What the player will do next
- The aesthetic or emotional register of the story — the World Agent deals in facts about the world, not in how those facts feel

**The World Agent's role in constraint adjudication:**
The three-tier constraint framework described in the Reality Testing section is the World Agent's primary domain of responsibility. Hard constraints — the physics of the world, the genre contract — are the World Agent's to enforce. When a player attempts something that violates the world's rules, the World Agent determines that it cannot happen and communicates this to the Narrator (who renders the refusal artfully) and to the Storykeeper (who records the attempt and its failure).

Soft constraints — the boundaries of the plausible for a particular character — are adjudicated collaboratively between the World Agent and the Storykeeper. The World Agent knows what the world permits. The Storykeeper knows what the character's tensor allows. Together they determine whether a particular action, in this particular moment, is possible and at what cost.

Perceptual constraints — what can be sensed and inferred — involve all three: the World Agent knows what information is physically available in the environment (what sounds carry through walls, what a scar would look like after ten years, what the weather is doing), the Storykeeper knows what information the character has access to based on their narrative position and information state, and the Character Agent's tensor determines the character's perceptual capacities.

**The World Agent's relationship to the Storykeeper:**
These two agents collaborate constantly but maintain distinct domains. The Storykeeper asks the World Agent: "Is it physically possible for these two characters to meet in this location by tomorrow?" The World Agent asks the Storykeeper: "Has anything happened narratively that would change the political status of this territory?" The Storykeeper might request a world-state update: "The player burned the bridge — what are the material consequences?" The World Agent might flag a narrative implication: "The river has flooded this season; any scene set at the low ford should account for this."

The division is not always clean — world and story interpenetrate — but the distinction is architecturally useful. It prevents the Storykeeper from being overburdened with physical and material reasoning, and it ensures that the world has its own integrity independent of the narrative's demands. The story does not get to override physics (within the genre contract). The world does not get to override narrative (within the authored structure). They negotiate, and the negotiation produces a richer reality than either could maintain alone.

**The World Agent's relationship to time:**
The World Agent manages the temporal dimension of the world — the passage of seasons, the aging of structures, the growth and decay of the environment. When narrative time advances (whether triggered by the story designer at specific narrative moments, or by the natural rhythm of day-night cycles), the World Agent determines what has changed in the world during that interval. A cabin left unattended for a winter may have a leaking roof. A garden planted in spring may be harvestable by autumn. A road damaged by floods may have been repaired by the local community — or may not have been, depending on the state of that community.

This temporal awareness feeds into both the Narrator's descriptions (the world looks different in different seasons, at different times of day) and the Storykeeper's narrative management (what is possible now may not have been possible a month ago, and vice versa).

---

### The Player-Character

The player-character is unique in the system: it is the one role not inhabited by an AI agent. It is inhabited by the player — a human being, bringing their own creativity, curiosity, perversity, kindness, impatience, and imagination to the story.

The player-character does have a tensor representation maintained by the Storykeeper — they accumulate relationships, knowledge, emotional states, reputational effects, physical conditions. But the *decisions* are human. The system does not model what the player will do. It models the world's response to what the player has done.

This is the essential asymmetry of the system, and it is the source of its vitality. Everything else in the system — the narrative graph, the character tensors, the information ledger, the agents and their partial perspectives — exists to create a world worthy of a human being's attention and choice.

---

## Information Flow

The architecture of information flow is the architecture of the system. Everything depends on who knows what, and when, and how that knowledge moves.

```
┌─────────────────────────────────────────────────────┐
│                   STORY DESIGNER                     │
│         (Authors narrative graph, character           │
│          tensors, scene definitions, world rules)    │
└──────────────────────┬──────────────────────────────┘
                       │
            ┌──────────┴──────────┐
            ▼                     ▼
┌────────────────────┐  ┌────────────────────┐
│    STORYKEEPER     │  │   WORLD AGENT      │
│  Holds: narrative  │  │  Holds: world      │
│  graph, info       │◄─►  model, physics,   │
│  ledger, character │  │  geography, time,  │
│  tensors, relation │  │  material state,   │
│  web               │  │  constraints       │
│                    │  │                    │
│  Provides filtered │  │  Provides world    │
│  info to:          │  │  state and rules   │
└──┬─────┬───────┬───┘  │  to:               │
   │     │       │      └──┬──────┬──────────┘
   │     │       │         │      │
   ▼     ▼       ▼         ▼      ▼
┌──────┐┌────────┐  ┌──────────────────┐
│Narr- ││ Char.  │  │   Reconciler     │
│ator  ││ Agents │  │                  │
└──┬───┘└───┬────┘  └────────┬─────────┘
   │        │                │
   │        ▼                │
   │  ┌─────────────┐       │
   │  │ Reconciler  │◄──────┘
   │  │ (sequences, │
   │  │  resolves)  │
   │  └──────┬──────┘
   │         │
   ▼         ▼
  ┌──────────────────────┐
  │      NARRATOR        │
  │  (renders experience │
  │   for the player)    │
  └──────────┬───────────┘
             │
             ▼
  ┌──────────────────────┐
  │       PLAYER         │
  │  (reads, chooses,    │
  │   acts, speaks)      │
  └──────────┬───────────┘
             │
         ┌───┴───┐
         ▼       ▼
  ┌──────────┐ ┌──────────────┐
  │STORYKEEP.│ │ WORLD AGENT  │
  │(records  │ │ (updates     │
  │ choices, │ │  material    │
  │ updates  │ │  state)      │
  │ state)   │ │              │
  └──────────┘ └──────────────┘
```

The flow is cyclical, and now bifurcated at both the top and bottom. The Story Designer's authored content flows into both the Storykeeper (narrative structure) and the World Agent (world rules and material reality). The player's actions flow back to both — the Storykeeper recording narrative consequences, the World Agent recording material consequences. These two agents collaborate constantly, each informing the other, and together they provide the filtered information from which the Narrator, Character Agents, and Reconciler do their work.

---

## Reality Testing: The Constraint Framework

A player enters a world they did not build. They must discover its shape from the inside — what is possible, what is forbidden, what is merely difficult, what is beyond their reach but not beyond the world's. This is a problem of **reality testing**: the process by which a perceiving agent — player or character — learns the texture of what is real in a particular world, through the experience of acting within it.

In a novel, the author establishes the reality contract implicitly through what they describe, what they permit, what they render. The reader learns the physics of the world by inhabiting it through prose. In a tabletop game, the GM and the rulebook together establish it — the player proposes, the GM adjudicates, and through that negotiation the boundaries of the possible become felt. In our system, that negotiation happens between the player and the agents, mediated by the world the story designer has built.

The player must be able to *feel their way into* what is real without encountering either a wall ("you can't do that") or an abyss ("you can do anything, and nothing has weight"). The system achieves this through a three-tier constraint framework, each tier adjudicated by different agents and communicated through the Narrator's craft rather than through system messages.

### Hard Constraints: The Physics of the World

Hard constraints are the genre contract — the fundamental rules that define what kind of world this is. In the noir story, there is no telepathy. In the fantasy story, magic exists but follows rules. In the pastoral tale, the seasons turn but war does not arrive. These are authored by the story designer, held by the World Agent, and enforced in collaboration with the Storykeeper. They are not negotiable, not even by the most creative player action.

When a player attempts something that violates a hard constraint, the system does not simply refuse. It refuses *in a way that teaches the player about the world.* The Narrator might say: *"You reach for something inside yourself, some power you've heard whispered about — but there is nothing there. This world does not work that way. What it offers instead is the sharpness of your own attention, the way you notice the slight tremor in his hand when he reaches for the glass."*

The refusal becomes a revelation. The player learns not just what they cannot do, but what the world offers instead. Every boundary, artfully communicated, is also an invitation.

### Soft Constraints: Character Capacity in Context

Soft constraints are the boundaries of the plausible for this particular character, in this particular moment, given their capacities, knowledge, emotional state, and the demands of the scene. These are not absolute rules but contextual judgments.

Consider: a character with draconic ancestry in a fantasy story might, in a moment of life-or-death extremity, call fire through their fingertips — a bone-deep generational memory igniting under impossible pressure. This is within the world's physics (hard constraints permit it) but it is not casually available. It requires the right character, the right moment, the right accumulation of narrative weight and thematic resonance. The same character probably cannot rip a tree from the ground with their bare hands. Not because the world forbids superhuman strength, but because *this character* does not possess it, and the moment does not call for it.

The World Agent, Reconciler, and Storykeeper together adjudicate soft constraints. The World Agent knows what the world permits. The Storykeeper knows the character's tensor — their capacities, their limits, the conditions under which extraordinary action becomes possible. The Reconciler manages the moment-to-moment resolution of actions against these capacities. The answer to "can I do this?" is often not yes or no but *not yet*, or *not like that*, or *yes, but it will cost you something*.

### Perceptual Constraints: What Can Be Sensed and Inferred

The third tier concerns not what the character can *do* but what they can *perceive*. Every character has a perceptual horizon — the boundary of what their senses, training, intuition, and attention make available to them.

The noir detective cannot read minds. But they can read rooms. They notice the way someone's posture shifts, the micro-expression that flickers and disappears, the detail that doesn't fit the story being told. This is not a magical power; it is a capacity that lives in the character tensor — trained sensitivity, honed attention — and the Narrator renders it as a kind of heightened perception that gives the player information without violating the world's reality contract. The character doesn't know what the suspect is thinking. But they notice something, and what they notice is real.

Perceptual constraints are managed through the information flow between Storykeeper, Character Agents, and Narrator. The Storykeeper knows what information is available in the scene. The character's tensor determines what subset of that information the character could plausibly perceive. The Narrator renders the perception — not as raw data but as felt experience: a glance that lingers too long, a scent that triggers a memory, a silence where a sound should be.

### The Pedagogical Function of Constraints

Each tier of constraint serves a pedagogical function: through the experience of encountering them, the player learns the world. Hard constraints teach the genre contract. Soft constraints teach what their character is — their gifts, their limits, their potential. Perceptual constraints teach them how to *attend* — what to look for, what to listen to, what matters in this world.

The player learns the world the way a child learns physics: by acting and receiving responses that have consistent, intelligible shape. The system never says "you can't do that" as a bare refusal. It always says, in effect, "this is what happens when you try" — and what happens teaches.

---

## On Building Tailored Rather Than Generic

We have stated that we prefer to avoid "agent swarm" style code frameworks. This deserves explanation beyond mere preference.

Generic agent orchestration frameworks are designed to be general-purpose. They optimize for flexibility and ease of deployment across many use cases. But our system has very specific requirements — particular information boundaries, particular relationships between agents, a particular philosophy about what each agent knows and how knowledge flows. Forcing this into a generic framework would mean fighting the framework at every turn, or worse, compromising the design to fit the framework's assumptions.

We will build the orchestration layer tailored to our needs. This means more upfront work, but it means that the system's architecture can faithfully mirror its philosophy — that the code can be a true expression of the ideas, rather than an approximation squeezed into someone else's abstractions.

This is a principle borrowed from craft: use the tool that fits the work, and if no tool fits, make one.

---

## Technical Anchors

While this document is primarily concerned with conceptual architecture, we anchor it in several technical commitments that will be elaborated in the Technical Direction document:

- **Rust and Bevy** form the core engine. Rust for its reliability, performance, and expressiveness; Bevy for its entity-component-system architecture, which maps naturally onto a world of entities (characters, objects, locations) with components (traits, states, relationships) acted upon by systems (agents, the narrative engine, the reconciliation layer).
- **LLM integration** will support both API-based models (for players willing to bring their own API key and access the most capable models) and local models via Ollama or similar (for lower-cost, private, or offline play). The system must be designed so that the quality of the experience scales with model capability but remains meaningful at every tier.
- **The orchestration layer** will be custom-built, reflecting the specific information flow and agent relationships described above. It will not depend on third-party agent framework libraries.

---

## A Note on Emergence

We design for authored structure, but we must also design for emergence — for the moments when the interplay of agents, player choices, and narrative state produces something no one planned. A character whose accumulated relational shifts lead them to an action that surprises even the story designer. A scene that gains unexpected resonance because of the particular path the player took to reach it. A joke that lands because two Character Agents, operating independently, happened to produce complementary responses.

These moments are not bugs. They are the system working as intended. The architecture must be robust enough to contain emergence without breaking, and open enough to let it breathe.

This is, perhaps, the deepest aspiration of the system: not merely to tell stories, but to become a space in which stories *happen* — stories that could not have been predicted, even by those who built the space in which they unfold.

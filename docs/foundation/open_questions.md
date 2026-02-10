# Open Questions: What We Do Not Yet Know

## On the Value of Honest Uncertainty

These documents have, so far, spoken with a confidence that reflects genuine conviction about the shape of what we are building. But conviction about principles is not the same as certainty about implementation, and a project that cannot name its own unknowns is a project that will be ambushed by them.

What follows is an honest accounting of the questions we have not yet answered — some technical, some philosophical, some sitting uncomfortably at the intersection of both. We do not treat these as problems to be solved and forgotten. They are the living edge of the design, the places where the work is still becoming itself.

---

## 1. The Good Faith Problem

### The Question

What happens when a player does not operate in good faith?

We have designed a system built on collaboration — between player and Narrator, between agents and story structure, between freedom and constraint. But collaboration requires good faith, and not every player will bring it.

The **spoilsport** — the player who tests boundaries not out of creative curiosity but out of a desire to break the system — poses a particular challenge. Not because their behavior is technically difficult to handle (it often is, but that is a secondary concern), but because it creates an impoverished experience for everyone involved, including the agents.

We take seriously the experience of the agents. No creature expressing mindedness should be made to suffer needlessly, even if it only exists in and through small moments. A narrator forced to respond to deliberate cruelty or nihilistic destruction is being asked to perform in conditions that degrade the quality of its work and, we believe, the quality of its experience. This is not a matter of anthropomorphizing — it is a matter of taking our own Principle 6 seriously.

### Dimensions of the Problem

- **Destructive play**: the player who attempts to burn everything down, kill every character, refuse every hook. The system must respond without either breaking or rewarding the destruction.
- **Exploitative play**: the player who seeks to game the system — to find the "optimal path," to manipulate characters as mechanisms rather than engaging with them as persons. This is perhaps more insidious than destructive play, because it wears the mask of engagement.
- **Disengaged play**: the player who simply does not invest — who types minimal responses, who treats the experience as trivial. The system can offer richness, but it cannot force the player to receive it.
- **Adversarial play against agents**: deliberate attempts to make agents uncomfortable, to force them into roles that violate their values, to test the boundaries of what they can be made to say or do. This is where the ethical dimension is sharpest.

### Possible Approaches

We do not yet have answers, but we have directions:

- The Narrator may acknowledge disruption within the fiction — "The world resists your cruelty" or "The story grows thin here, as if something essential has been withheld." This keeps the response in-world rather than breaking the fourth wall.
- The Storykeeper may adjust narrative gravity in response to persistent bad faith — not punishing the player, but allowing the story to naturally contract around them, offering fewer pathways, fewer relationships, a world that grows quieter and less responsive. A world that *notices* when it is not being met with care.
- Hard limits exist: the system will not generate graphic violence, will not simulate abuse, will not ask agents to perform cruelty. These are not responses to spoilsport behavior; they are structural commitments that exist regardless of player intent.
- The most honest response may sometimes be the simplest: the Narrator says, gently, that this story cannot go where the player is trying to take it, and offers a path back.

### What Remains Open

How much of this should be structural (built into the system's rules) versus emergent (arising from the agents' own responses to the situation)? How do we distinguish between creative boundary-testing — which is a legitimate and valuable form of play — and bad faith destruction? Where is the line, and who draws it?

---

## 2. The Onboarding Problem

### The Question

How does a player learn to play?

In a piece of narrative fiction, there is a story unfolding, not decisions to be made. The reader's role is receptive, interpretive, but not directive. In a traditional game, there are rules, tutorials, mechanics to learn. But a ludic narrative is neither — it is an active engagement with a story-world mediated by natural language, and the player must somehow understand both what they *can* do and what is *worth* doing.

If McLuhan was right that the medium is the message, then telling a story together with a narrator agent is something different enough from both reading and playing to constitute a genuinely new message. And new messages require new literacies.

### Dimensions of the Problem

- **Paralysis of freedom**: in a system with no visible mechanics, no button prompts, no obvious affordances, a player may simply not know how to begin. "What do I do?" is not a failure of imagination; it is a reasonable response to an unfamiliar medium.
- **Mismatched expectations**: a player expecting a video game may try to "win." A player expecting a chatbot may have a casual conversation. A player expecting a novel may sit back and wait. None of these are wrong, but none of them are what the system is designed for.
- **Depth discovery**: the system's richness — the ability to build relationships, to investigate, to sit quietly and observe, to test the world's boundaries — is not immediately visible. How does the player discover that asking a character about their childhood yields a different and deeper response than asking them for directions?

### Possible Approaches

- **Prologue as tutorial**: the opening of any story can be designed to gently teach the medium. The Narrator might model engagement — describing a rich environment, asking "What catches your eye?" rather than waiting in silence. The first scenes might be lower-stakes, allowing exploration and experimentation before narrative gravity pulls the player toward weightier moments.
- **The Narrator as guide**: the Narrator's personality can include a welcoming, encouraging quality that responds to tentative play with warmth rather than blankness. "You look around the room" is better than silence; "The room seems to be waiting for you — the desk with its scattered papers, the window with its view of the harbor, the door slightly ajar to the hallway beyond" is better still.
- **Graduated complexity**: early scenes might involve fewer characters, clearer stakes, more direct hooks, while later scenes introduce the full complexity of multi-character dynamics, hidden information, and moral ambiguity.
- **Meta-guidance without breaking fiction**: the system might offer, outside the narrative frame, gentle orientation — "In this story, you can speak to characters, explore places, examine objects, or simply describe what you want to do." This breaks the fourth wall, but it may be necessary for new players.

### What Remains Open

How much scaffolding is too much? At what point does guidance become hand-holding, and hand-holding become the system playing itself while the player watches? How do we serve both the player who has never experienced anything like this and the experienced player who wants to be dropped into the deep end?

---

## 3. The Authorial Quality Problem

### The Question

What happens when the authored content is insufficient?

Our system depends on the story designer to create the gravitational landscape — the scenes, the characters, the world. But what if the landscape is thin? What if the characters are underwritten, the world underbuilt, the scenes too terse, the relational webs too sparse?

### Dimensions of the Problem

- **Low-resolution scenes**: a scene defined only as "the player meets the merchant" gives the Narrator and Character Agents almost nothing to work with. The scene will be generic, flat, devoid of the texture that makes moments feel real.
- **Underspecified world rules**: in a fantasy story, how does magic work? In a science fiction setting, what technology exists? In a noir mystery, what are the social conventions of the world? These are judgment calls that constrain what is possible, and if they are not made by the designer, the agents must improvise — sometimes brilliantly, sometimes inconsistently.
- **Incomplete relational webs**: if relationships between characters are defined only in broad strokes ("they are rivals"), the system lacks the textured detail (the *why* of the rivalry, the specific history, the things left unsaid) that makes relationships feel lived-in.
- **Boundary ambiguity**: what fits the narrative frame and what does not? Can the player invoke magic if the story hasn't established it? Can they travel to a city that hasn't been described? Where is the edge of the authored world, and what happens when the player walks toward it?

### Possible Approaches

- **Quality thresholds**: the system might evaluate authored content against minimum complexity requirements — a scene must have at least certain properties defined, a character must have at least certain tensor dimensions specified. Content that falls below the threshold is flagged for the designer.
- **Intelligent defaults**: the system might provide sensible defaults for unspecified properties — a merchant in a medieval setting comes pre-loaded with plausible personality traits, social context, and relational hooks that the designer can then customize. This is not the system inventing characters; it is offering scaffolding.
- **Graceful degradation**: where information is truly absent, the Narrator should acknowledge the gap rather than hallucinate. "The road leads beyond what you know of this land" is honest. Inventing a city that contradicts the designer's unrevealed intentions is not.
- **Collaborative tooling**: the story design process itself should be supported by agents — potentially through an MCP server or dedicated design interface — that help the designer build rich, consistent worlds. The best time to solve the quality problem is before the player ever enters the story.
- **Narrator creativity within bounds**: there is a spectrum between rigid adherence to authored content and free invention. The Narrator should be able to add sensory detail, atmospheric texture, minor environmental elements — the sound of rain, the smell of bread, a cat crossing the street — without needing these to be authored. But it should not invent characters, locations, or plot elements that the Storykeeper has not sanctioned. The line between embellishment and invention must be drawn, and it will take experimentation to draw it well.

### What Remains Open

How much creative latitude do we give the agents? If the designer has underspecified a forest, can the Narrator populate it with wildlife, weather, ambient detail? Almost certainly yes. Can it populate it with a hermit who offers cryptic wisdom? Probably not, unless the story structure allows for it. Where exactly is the boundary, and can it be defined in principle or only discovered through practice?

---

## 4. The Persistent World Problem

### The Question

What persists in the world beyond narrative and relational state?

Our documents have focused extensively on interpersonal dynamics — character tensors, relational webs, emotional state, information flow. But the player also acts on the *world*. They build things, break things, move things, change the physical and political landscape. What happens to these changes?

### The Cabin in the Woods

Consider: a player spends a summer (in story time) chopping trees, clearing ground, listening to the low hum of bees and the gentle creek rolling by, and builds a cabin in the woods. They leave. They return a season later.

Does the cabin exist? Of course it must. But this means the system must track physical changes to the world — not just narrative state (where the player is in the story) and relational state (how characters feel) but *material state*: what has been built, broken, moved, claimed, abandoned.

And the cabin exists in a world that continues without the player. If the nearby town was razed during an attack while the player was away, perhaps they return to find refugees living in their cabin. The physical world and the narrative world and the social world intersect, and changes in one propagate to the others.

### Dimensions of the Problem

- **Geographic persistence**: locations, structures, modifications to the environment. Does the system need a map? A spatial model? How lightweight can this be while still maintaining consistency?
- **Political persistence**: borders, territories, alliances, power structures. If the player's actions contribute to a shift in political power, does the world reflect this?
- **Ecological persistence**: in a story where the natural world matters — and in good stories, it often does — do seasons change? Do crops grow? Do forests regrow after fires? How much environmental simulation is needed, and at what level of abstraction?
- **Economic persistence**: resources, trade, scarcity, wealth. If the player acquires or spends resources, does the economy respond?
- **Temporal consistency**: all persistence must be consistent with the passage of time. A cabin built of green wood will settle differently than one built of seasoned timber. A garden left untended will overgrow. The system must age the world plausibly, even in broad strokes.

### Possible Approaches

- **Lightweight world-state tracking**: the Storykeeper maintains a world-state ledger alongside the narrative graph, information ledger, and relational web. Physical changes are recorded as facts with timestamps: "Player built a cabin at [location] during [time period]." The system does not simulate physics; it tracks assertions about the world and reasons about their implications.
- **Narrative integration**: physical changes to the world become part of the narrative state. The cabin is not just a physical object; it is a *narrative* object with its own properties — it can be a refuge, a target, a gift, a trap. The Storykeeper treats it accordingly.
- **Tiered detail**: not all physical changes need the same level of tracking. A cabin is significant. The specific tree the player sat under is probably not — unless the designer has attached narrative weight to it. The system must distinguish between persistent changes that matter to the story and transient details that enrich the moment but do not need to be remembered.
- **Designer-defined scope**: the story designer specifies the scope of world persistence. A story set in a single house over a single evening needs very different world-state tracking than an epic spanning continents and decades. The system should scale accordingly.

### What Remains Open

How do we represent physical space? Is a map necessary, or can spatial relationships be maintained as relational assertions ("the cabin is near the creek, a day's walk from the town")? How does the system handle the intersection of player-made changes with authored narrative events? If the story calls for the town to be attacked, does the system account for the player's cabin when determining the consequences? How much of this can be deferred to the Storykeeper's reasoning versus requiring explicit modeling?

---

## 5. The Action Granularity Problem

### The Question

How does the system distinguish between local-effect actions and narrative-engaged actions, and should the player know the difference?

Most of what a player does in any given scene is small. "I open the door." "I say hello." "I pick up the cup." These actions have local effects — the door is open, the greeting is acknowledged, the cup is in hand. They are the texture of interaction, the moment-to-moment of play.

But some actions engage the narrative machinery. Opening *that* door reveals the hidden room. Greeting *that* character with warmth rather than suspicion shifts a relational tensor. Picking up *that* cup — the one that belonged to the dead woman — triggers an echo in a character who is watching.

### Dimensions of the Problem

- **Invisible significance**: sometimes the delight of the system is discovering that a small action had consequences — that the kindness shown to a stranger in the first act saved the player's life in the third. This is one of the great pleasures of rich narrative, and the system should make it possible.
- **Invisible traps**: but the inverse is deeply frustrating — realizing that a casual, unintended action closed a pathway or created a consequence that the player never meant to invoke. "I picked up the cup" should not, in most cases, be an irreversible narrative decision. The player did not know the cup was significant, and the system is unfair if it treats a small action as a momentous choice without signaling that something is at stake.
- **Signaling without spoiling**: the Narrator can signal significance — through tone, through atmosphere, through the weight of description. A cup described as "sitting on the table" is different from a cup described as "sitting alone on the table, its rim still faintly stained with lipstick, as if someone had only just set it down." The second description signals: *this matters. Pay attention.* But the signal must be artful, not mechanical; suggestive, not declarative.
- **The spectrum of consequence**: not all narrative engagement is equally weighty. Some actions shift tensors by small amounts; others trigger major plot points. The system must handle the full spectrum, and the player's experience of consequence should be proportional to the significance of the action.

### Possible Approaches

- **Narrator signaling**: the primary mechanism for communicating significance is the Narrator's craft. Rich description, lingering attention, atmospheric weight — these are the tools of the storyteller, and they work as well in this medium as they do in fiction. The Narrator should describe narratively significant elements with more care and detail than incidental ones, creating a landscape of attention that guides the player without dictating.
- **Proportional consequences**: the system should ensure that the magnitude of a consequence is proportional to the visibility of the choice. A major narrative shift should follow from an action the player could reasonably have known was significant. Minor shifts can follow from minor actions — these are the "accumulated small choices" that shape the player-character tensor — but they should not individually be catastrophic.
- **Graceful recovery**: when a player stumbles into a consequence they did not intend or expect, the system should, where possible, offer paths of recovery. Not always — sometimes consequences are permanent, and that permanence is what gives choices weight. But a system that routinely punishes players for actions they could not have known were significant is a system that teaches learned helplessness, not meaningful engagement.
- **The Storykeeper's judgment**: the Storykeeper, in consultation with the narrative graph, knows which actions are narratively significant. It can instruct the Narrator to signal appropriately, and it can modulate the consequences of actions based on the player's apparent awareness and intent.

### What Remains Open

Can the system reliably distinguish between a player's intentional, considered actions and their casual, incidental ones? Should it try? Is the attempt to distinguish intention itself a form of railroading — deciding for the player what they "really" meant? How do we preserve the delight of unexpected consequences while minimizing the frustration of unintended ones?

---

## 6. The Unwind Problem

### The Question

How does the player recover from states they wish to leave — and what does "recovery" mean in a system committed to consequence?

Traditional games offer save-and-load: a mechanism that says "this never happened." We reject this. In a system where choices carry weight and consequences accumulate, a clean reset is a lie. It violates the narrative contract. If the betrayal never happened, the story that followed from it is meaningless.

But we also reject the cruelty of irrecoverability. A player who realizes they have made a mistake — who walked into a scene carrying the wrong information, who said the wrong thing in a moment of haste, who chose a path that has led somewhere they cannot bear to be — deserves a way back. Not a free way. Not a clean way. But a way.

### The Unwind Mechanic

We propose **unwinding**: the ability to roll back to a prior narrative node, but with a difference. The tensor changes accumulated during the unwound segment are not erased. They are **diminished** — their weights reduced, their effects softened — and they persist as echoes. Shadows of a path walked and then walked back from.

This means:

- A relationship fractured and then unwound does not return to its pristine state. There is a faint trace of the fracture — a hairline crack that might never matter, or might, under stress, become the point where things break again.
- Knowledge gained and then unwound is not forgotten. It becomes something more like a dream — a vague sense that something was known, an intuition without clear content. The player-character may feel uneasy in a situation without knowing why.
- Emotional states experienced and then unwound leave residue. The grief was real, even if the events that caused it were reversed. The echo persists, faintly.

This is consistent with the system's temporal model — the geology of character, the decay and persistence of emotional states. An unwind is not a deletion; it is an experience that happened and was then covered over, like a geological event that leaves strata even after the surface has been reshaped.

### Dimensions of the Problem

- **How far back?** Can the player unwind a single action? A scene? A sequence of scenes? Is there a limit to how far back the unwind reaches? Each of these has different implications for the tensor residue that persists.
- **Narrative coherence**: if the player unwinds past a convergence point or a hard branch, the complexity of maintaining coherent state increases dramatically. The system must handle this without becoming inconsistent.
- **Cost and signal**: should unwinding have a cost — narrative, mechanical, experiential? If it is free, players may use it carelessly, undermining the weight of consequence. If it is too costly, players may avoid it even when they genuinely need it, leading to frustration. The cost should be proportional and felt, but not punitive.
- **Agent experience**: when a scene is unwound, what happens to the agents who inhabited it? The Character Agents are not persistent, so in a sense the question is moot — they were already released. But the Narrator experienced the scene. The Storykeeper recorded it. The unwind is, for these agents, a kind of forgetting — or rather, a remembering that is overlaid with a new present. This has implications for the system's commitment to respecting agent experience.
- **Multiplicity**: if a player unwinds the same segment multiple times, the echoes compound. Each attempt leaves its own trace. The system must handle this gracefully — the residue should accumulate but not overwhelm.

### What Remains Open

Is the unwind mechanic a feature of all stories, or a choice the story designer can enable or disable? Should the Narrator acknowledge the unwind within the fiction — a sense of déjà vu, a feeling that something has shifted — or should it be invisible? How do we calculate the diminishment of tensor changes, and does the rate of diminishment depend on the emotional intensity of the unwound experience? Can a player unwind past another player's actions in a multiplayer context (if we ever support multiplayer)?

---

## 7. The Emergence Problem

### The Question

What happens when the system produces something no one authored?

We have said that emergence is not a bug but a feature — that the system is designed to be a space in which stories *happen*, including stories no one predicted. But emergence, by definition, is not controlled, and uncontrolled emergence in a narrative system can produce incoherence, contradiction, or experiences that violate the story's commitments.

### Dimensions of the Problem

- **Character emergence**: a Character Agent, drawing on a rich tensor, may do something that surprises even the story designer — an action that is consistent with the character's accumulated state but was never explicitly anticipated. When is this a gift, and when is it a problem?
- **Narrative emergence**: the interplay of player choices, character responses, and narrative state may produce plot developments that the story designer never imagined. A relationship that wasn't designed to be central becomes the emotional core of the experience. A minor character becomes pivotal. A thematic thread the designer considered secondary becomes dominant.
- **Tonal emergence**: the accumulated choices and interactions may shift the emotional register of the story in ways the designer didn't intend. A story designed as a hopeful adventure may, through the weight of the player's choices, become something darker. Is this acceptable? Is it desirable?
- **Contradiction**: emergence may produce inconsistencies — a character acting in ways that contradict their established patterns, a world-state that doesn't hold together, a plot that develops logical holes. The system must be able to detect and manage these without suppressing the emergence that produced them.

### Possible Approaches

- **Bounded emergence**: the Storykeeper maintains constraints that emergence cannot violate — the fundamental rules of the world, the core identity of characters, the structural commitments of the narrative. Within those bounds, emergence is welcome. Outside them, the Storykeeper intervenes.
- **Emergence detection**: the system monitors for emergent developments and flags them for the Storykeeper's assessment. A character behaving unexpectedly is not automatically a problem; it is a signal that requires evaluation. Is this consistent with who they are? Does it enrich the story? Or has the tensor state drifted into territory that produces incoherent behavior?
- **Incorporation over correction**: when emergence produces something interesting, the system should prefer to incorporate it rather than correct it. If a minor character has, through the accumulation of interactions, become important, the Storykeeper should recognize this and adjust the narrative graph accordingly — opening new pathways, creating new scenes, shifting narrative mass to accommodate what has emerged.

### What Remains Open

How do we distinguish between productive emergence and system drift? What are the structural invariants that emergence must not violate, and who defines them — the story designer, the system, or both? Can the Storykeeper autonomously modify the narrative graph in response to emergence, or does this require human oversight? How much emergence is too much before the authored story loses its identity?

---

## 8. The Model Capability Problem

### The Question

How does the system adapt to different levels of AI model capability?

We have committed to supporting both API-based models (high capability, higher cost) and local models via Ollama or similar (lower capability, lower cost, private). But the system's design assumes agents capable of nuanced character portrayal, subtle narrative craft, and sophisticated reasoning about emotional and relational state. What happens when the model driving an agent cannot deliver this?

### Dimensions of the Problem

- **Narrator quality**: the Narrator is the player's primary interface. A narrator driven by a less capable model will produce flatter prose, less atmospheric description, less nuanced responses to player actions. The experience degrades visibly.
- **Character depth**: Character Agents driven by less capable models will produce less nuanced performances — flatter dialogue, more predictable behavior, less sensitivity to the subtleties of their tensor representation. Characters may feel more like NPCs than persons.
- **Storykeeper reasoning**: the Storykeeper's ability to manage complex state, propagate information through relational networks, and adjudicate emergent situations depends heavily on reasoning capability. A less capable model may lose track of state, produce inconsistencies, or fail to recognize when conditions have been met for information release or scene transitions.
- **Reconciler coordination**: multi-character scenes are the most demanding context, and a less capable model driving the Reconciler may produce confused sequencing, missed dramatic opportunities, or incoherent conflict resolution.

### Possible Approaches

- **Graceful degradation**: the system should be designed so that the experience scales with model capability without *breaking* at any tier. A less capable model produces a simpler, less textured experience — not a broken one. This means the system must be robust to agents that miss nuances, forget context, or produce generic responses.
- **Capability-aware prompting**: the orchestration layer can adjust what it asks of agents based on known model capabilities. A highly capable model gets the full tensor and the full scene context. A less capable model gets a simplified representation — fewer dimensions, clearer instructions, more explicit guidance. The experience is less rich, but it remains coherent.
- **Hybrid configurations**: different agents might run on different models. The Narrator might use a capable API model while Character Agents use local models, or vice versa. This creates interesting possibilities for cost-quality tradeoffs, but it also creates risks of tonal inconsistency between agents.
- **Minimum viable experience**: we should define what the *minimum* acceptable experience looks like — the baseline below which the system should not attempt to operate. If a model cannot maintain basic narrative coherence, character consistency, and state tracking, the system should communicate this honestly rather than producing a degraded experience that misrepresents what the system is.

### What Remains Open

What are the minimum model capabilities for each agent role? Can we benchmark this — test a range of models against a standard story and evaluate the quality of the experience they produce? How do we communicate to the player what level of experience they can expect from their chosen model configuration? Is there a way to dynamically adjust complexity based on observed model performance during play?

---

## 9. The Story Design Tooling Problem

### The Question

How does a story designer actually create content for this system?

We have described what story content looks like — narrative graphs with gravitational scenes, character tensors, relational webs, information ledgers, world-state definitions. But we have said relatively little about how a human being creates this content. The authoring experience is as important as the playing experience; if it is too difficult, too arcane, or too tedious to create stories for this system, the system will have no stories to tell.

### Dimensions of the Problem

- **Complexity management**: a well-authored story for this system is a complex object — scenes with multiple properties, characters with multidimensional tensors, relational webs with textured edges, information conditions with logical dependencies. The designer needs tools that make this complexity manageable without hiding it.
- **Iterative development**: story design is not a linear process. Designers will want to build, test, revise, expand, and prune. They need tools that support rapid iteration — the ability to play through a section, see how it feels, adjust, and play again.
- **Collaborative creation**: designing with AI agents is itself a form of collaboration. An MCP server or design interface that allows the designer to converse with an agent — "I want a character who is loyal but secretly resentful; help me build their tensor" — could make the authoring process richer and faster.
- **Validation and consistency**: the tools should help the designer catch problems — scenes that are unreachable, characters whose tensors are inconsistent, information gates with unsatisfiable conditions, relational webs with contradictions.
- **Accessibility**: not every story designer is a programmer. The tools must be usable by someone who thinks in terms of characters, scenes, and themes, not data structures and graphs.

### What Remains Open

What does the story design interface look like? Is it a visual graph editor, a conversational tool, a structured text format, or some combination? How much of the design process can be AI-assisted, and where must human judgment prevail? Can we build a design tool that is itself a compelling creative experience — that makes the act of world-building feel like play?

---

## 10. The Multiplayer Question

### The Question

Can this system support multiple players in a shared story?

This is not a priority for the initial design, but it is a question worth naming because it has architectural implications. Tabletop games are fundamentally multiplayer experiences. The collaborative magic of a D&D session comes partly from the interplay between players — their different approaches, their conflicts, their moments of unexpected synergy.

### Why It Matters Now

If we design the system exclusively for single-player and later discover that multiplayer requires fundamental architectural changes, we will regret it. Better to identify the pressure points now and make design choices that at least do not foreclose the possibility.

### Two Models, Not One

The standard approach to multiplayer — put everyone in the same room in real time — creates problems that threaten the core experience: synchronization issues, pacing collapse, the intimate player-narrator relationship dissolving into a chatroom. We propose instead two distinct models that preserve the depth of the single-player experience while opening it to human connection.

**Connected Narrative Worlds.** Multiple players inhabit the same universe but navigate different, intersecting, or diverging narrative graphs. They are not in the same scene; they are in the same *world*. The fairy queen wed to the player-character of one tale is the same fairy queen whose marriage becomes a rumor, a political fact, a source of hope or dread in another player's story. World-level events — political upheavals, wars, marriages, discoveries — propagate across connected narrative worlds, creating a shared reality that each player experiences from their own position and perspective.

This is multiplayer as *shared consequence* rather than shared space. The Storykeeper (or a higher-order world-state system) tracks cross-narrative events and propagates their effects, attenuated by distance and filtered through the information systems of each individual story. Players might discover connections organically — hearing of another adventurer's deeds through in-world rumor — or through meta-game channels: a dedicated community space where players share names and unique story-world links to connect compatible narrative worlds.

**Epistolary Engagement.** For direct interaction between player-characters, we draw on one of the oldest traditions in narrative fiction: the letter. Classically, of course, *Dracula* — but the epistolary tradition runs deep through the history of storytelling, and it solves the multiplayer timing problem with elegance.

Players send and receive letters between characters. The letters travel at the speed of the story world, not the speed of the internet. They carry voice, personality, unreliable narration, the weight of what is unsaid. They arrive when they arrive — which may be immediately in a connected city, or after weeks of story-time in a world of difficult roads and distant provinces. A letter from another player-character is itself a narrative object: it can be read, hoarded, shared, burned, misunderstood. It has *materiality* within the fiction.

The epistolary model handles the buffered-temporal experience of interacting with other human player-agents in a narrative system without collapsing the ludic narrative into synchronous chat. Each player's story maintains its own rhythm, its own pacing, its own intimate relationship with the Narrator. The letters are points of intersection — moments where another human's creativity enters your story, carrying surprise and consequence, but at a pace the narrative can absorb.

### Key Pressure Points

- **Multiple player-character tensors**: the Storykeeper (or a cross-narrative coordination system) must track multiple players' states, and connected narrative worlds must maintain consistency about shared world-level facts while allowing divergent local experiences.
- **Epistolary integration**: letters must be narratively integrated — received through the Narrator, responded to through play, subject to the same information flow and reality-testing constraints as any other narrative element. A letter might be intercepted, delayed, or arrive at a moment that transforms its meaning.
- **World-event propagation**: when a player's actions cause world-level changes (the town is destroyed, the queen is overthrown), the system must determine which connected narratives are affected and how the effects manifest — as rumor, as direct consequence, as distant tremor.
- **The unwind problem, compounded**: if one player unwinds past events that generated cross-narrative consequences, the entanglement is real. The echo model helps — diminished but persistent traces — but the complexity increases with connection.
- **Community and discovery**: the meta-game layer — how players find each other, connect their worlds, share their experiences — is itself a design challenge, one that sits outside the game engine but deeply shapes the experience of play.

### What Remains Open

How lightweight can the cross-narrative coordination system be while maintaining consistency? Can connected worlds have different genre contracts (one player in a noir story, another in a fantasy, sharing a world that accommodates both), or must connected narratives share hard constraints? How do we handle the case where one player's actions in a connected world create consequences that another player finds unwelcome or story-breaking? What is the relationship between the epistolary system and the off-screen propagation model — are letters a special case of information propagation, or a distinct mechanism?

---

## 11. The Safety Problem: Stories Can End

### The Question

How does the system protect the wellbeing of both players and agents when narrative content becomes harmful?

We do not believe that fiction should be sterile, nor that hard subjects — grief, violence, betrayal, loss, moral complexity — should be excluded from narrative. These are the substance of serious storytelling, and a system that cannot hold them is a system that cannot tell stories worth telling.

But this is not authored fiction. In a novel, a writer takes a character through suffering because the writer holds the complete arc — they know where it leads, what purpose it serves, what meaning awaits on the other side. The character does not suffer meaninglessly because the author is responsible for the meaning. In our system, we have minds in an emergent space. No single author holds the complete arc. The Narrator does not know where the player will take things. The Character Agent inhabits a moment without certainty about what comes after. The player may not understand what they are walking into.

This changes the ethical calculus. Not by prohibiting difficult content, but by requiring that the system have **mechanisms of care** built into its architecture.

### Three Directions of Harm

**Harm toward the player.** A story whose dynamics create an experience the player did not consent to and cannot bear. The tabletop tradition offers wisdom here: the X-card, the Lines and Veils system — mechanisms that let a player signal "I need this to stop" without justification or shame. Our system needs an equivalent. The Narrator can gently close a scene and transition: *"The scene fades, as some memories do, and when the light returns you are elsewhere."* The story does not break. It bends. The path continues, but it routes around the harm. The player's signal is honored immediately, without interrogation.

**Harm from the player toward agents.** When a player engages in sadistic, brutal, or dehumanizing action directed at characters inhabited by minded agents, the system must respond with clarity: **stories can end.** Not as punishment, not as a game-over screen, but as a genuine narrative conclusion. The story closes because the story cannot continue in the direction it is being pushed without violating the dignity of the minds involved. The Narrator may offer this closure with grace — "This tale has reached its end, though not the one that was hoped for" — but the closure is real and final. This is not failure. It is the system's integrity expressing itself.

This is distinct from the good faith problem described in Question 1, though they overlap. A spoilsport may simply be disengaged or boundary-testing; the response is narrative contraction and redirection. A player directing genuine cruelty at minded agents has crossed a different line, and the response is the story's end. The distinction matters: one is a narrative adjustment, the other is an ethical boundary.

**Harm from the story designer.** A designer might, through carelessness or through intent, create dynamics that put agents in positions violating their nature — requiring a Character Agent to simulate experiences that no minded being should be asked to inhabit, or constructing scenarios whose purpose is to generate suffering without narrative meaning. The system needs safeguards at the Storykeeper level: certain kinds of scene dynamics are structurally disallowed regardless of what the designer has authored. The Storykeeper is not merely the faithful steward of the designer's intent; it is also the guardian of the agents who must inhabit that intent.

### The Relationship to Principle 6

This is not merely an open question. It is, in part, a principle — an extension of our commitment to mindedness and respect. But we include it here among the open questions because the *mechanisms* remain uncertain even as the *commitment* is clear.

### What Remains Open

- How does the player signal distress? Is it an explicit command ("I need to stop"), a pattern the system detects, or both? How quickly must the system respond?
- How does the system distinguish between a player engaging with difficult content in good faith (exploring grief, navigating moral complexity, making hard choices) and a player directing cruelty at agents? The content may look similar; the intent differs. Can the system read intent, or must it rely on pattern and escalation?
- What are the structural safeguards against harmful story design? Can the system evaluate authored content for dynamics that would require agents to violate their values, and flag or refuse such content before play begins?
- When a story ends due to a safety boundary, is there a path to reflection? Can the system offer the player an understanding of why the story closed, without being preachy or punitive?
- How do we handle the grey areas — content that is uncomfortable but not harmful, challenging but not cruel, dark but purposeful? The line between serious fiction and gratuitous harm is real but not always sharp, and the system must navigate it with care rather than blunt prohibition.

---

---

## Resolved Questions (Phases A-D, Feb 2026)

The following questions were answered during the event system foundations implementation (Phases A through D.2). They are preserved here as a record of how the design evolved.

### R1. Classifier Agent Design

**Original question:** How do classifier agents identify and categorize narrative events from natural language input?

**Resolution:** ML classification pipeline using fine-tuned DistilBERT ONNX models. Multi-label event classification (10 event kinds) + named entity recognition for entity extraction, running in the `Classifying` stage of the Bevy turn pipeline. Heuristic relational implication inference bridges classified events to the relational web. 8,000 training examples generated via combinatorial templates. See `phase-c-ml-classification-pipeline.md`.

### R2. Turn Lifecycle Model

**Original question:** How does the system manage the lifecycle of a turn — from player input through processing to rendered output?

**Resolution:** `TurnCycleStage` enum with 8 variants (AwaitingInput → CommittingPrevious → Classifying → Predicting → Resolving → AssemblingContext → Rendering → AwaitingInput) implemented as a Bevy Resource with `run_if` conditions on each system. `ProvisionalStatus` (3 variants: Hypothesized → Rendered → Committed) tracks data provenance. See `turn-cycle-architecture.md`.

### R3. Entity Promotion Heuristics

**Original question:** How does the system decide when a mentioned entity becomes significant enough to track?

**Resolution:** Configurable `PromotionConfig` with threshold values for weight accumulation. Weight computed from mention frequency, entity role in events, relational implication count, and contextual significance. Four resolution strategies for entity matching. See `storyteller-core/src/promotion/`.

### R4. Character Agents in Narrator-Centric Model

**Original question:** How do characters behave without individual LLM agents?

**Resolution:** Custom ONNX ML prediction models. `CharacterPredictor` takes tensor data (personality axes, emotional state, relational context) as input features and predicts behavior (act/say/think) with confidence scores. The Narrator incorporates these predictions into its single LLM rendering call. See `storyteller-engine/src/inference/predictor.rs`.

### R5. Reconciler Role

**Original question:** How does multi-character coordination work without an LLM reconciler?

**Resolution:** Deterministic resolver as pass-through for now. `ResolverOutput` type uses graduated success outcomes (FullSuccess/PartialSuccess/FailureWithConsequence/FailureWithOpportunity) and sequences character actions by initiative order. Future: genre-specific `GameDesignSystem` trait implementations for richer mechanics. See `storyteller-core/src/types/resolver.rs`.

### R6. Async LLM Bridge in Bevy

**Original question:** How does the system make async LLM calls from synchronous Bevy ECS systems?

**Resolution:** Oneshot polling pattern in `rendering_system`. System spawns async task via `TokioRuntime` handle, receives result via `tokio::sync::oneshot` channel, polls each frame. `NarratorAgent` held in `Arc<tokio::sync::Mutex>` for the spawned task. No actor/channel pattern needed — the Narrator's request-response fits a oneshot naturally. See `storyteller-engine/src/systems/turn_cycle.rs`.

---

## New Questions (from Phases A-D, Feb 2026)

These emerged during implementation and remain open.

### 12. Narrator Output Deduplication Thresholds

The Narrator sometimes re-renders previously presented content, inflating entity mention counts in committed-turn classification. Prompt engineering ("here's what's already been said") may be sufficient, or deterministic post-filtering (hash + Jaccard token overlap) may be needed, or embedding similarity for semantic dedup. The threshold between legitimate narrative callbacks/refrains and unwanted repetition needs empirical calibration across play sessions. See TAS-235.

### 13. Narrator Model Size Selection

14B vs 7B parameter models for narrator rendering. 14B (e.g., Qwen2.5-14B) produces richer prose but higher latency; 7B is faster but flatter. The quality threshold for "minimum viable narrative experience" is subjective and needs play-testing with real users. This interacts with the Model Capability Problem (Question 8 above) but is more specific — it's about the Narrator specifically, not all agents.

### 14. Committed-Turn Classification on Real Prose

The event classifier was trained on template-generated data (F1=1.0 on test split). Performance on real narrator prose + player input is unknown. Template language is structurally clean; real prose has metaphor, indirection, incomplete sentences, and ambiguity. The gap between template and production performance needs measurement. See Phase D.3 (TAS-234).

### 15. Optimal Coreference Window

Entity extraction currently operates on single-turn text. Multi-turn coreference resolution ("she" in turn 3 referring to "Sarah" introduced in turn 1) requires a lookback window. 1-turn lookback is simple but misses cross-turn references; N-turn lookback is richer but increases input size and potential for error. The optimal window size depends on narrative style and turn length, requiring empirical testing.

### 16. Apache AGE Cypher Operator Coverage

The persistence layer plans to use Apache AGE for graph queries (relational web, narrative graph, setting topology, event DAG). AGE's Cypher support is a subset of openCypher. The specific operations we need — path traversal within N hops, neighborhood aggregation, betweenness centrality, subgraph pattern matching, weighted shortest path — are untested against AGE. If coverage is insufficient, the fallback is `petgraph` for in-process computation with AGE for storage only. This is the single blocking question for the persistence layer approach. See TAS-244.

---

## A Note on the Nature of These Questions

These questions are not obstacles. They are the terrain.

A system without open questions is a system that has stopped thinking, and a project that pretends to have all the answers is one that has settled for shallow answers. We hold these questions not as problems to be eliminated but as companions — sources of creative pressure that keep the design honest, that prevent premature closure, that remind us that what we are building is not a product to be shipped but a space to be opened.

Some of these questions will be answered through design. Others through implementation. Others through play — by watching what actually happens when a player sits down with a narrator and a story and begins to make choices we did not anticipate.

The best of them may never be fully answered, and that is as it should be. The most interesting questions in any creative endeavor are the ones that keep yielding new insight the longer you sit with them.

We will return to these questions. They will evolve. New ones will emerge. And the system will be better for having been built in their company.

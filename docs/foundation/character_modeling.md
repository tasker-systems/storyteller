# Character Modeling: Tensor, History, and the Shape of a Person

## On the Insufficiency of Stat Sheets

Every tabletop roleplayer knows the character sheet: a grid of numbers â€” Strength 14, Wisdom 11, Charisma 16 â€” that purports to capture a person. It is a useful fiction. It gives mechanical resolution something to work with. But it is impoverished in the way that a photograph of a house is impoverished compared to the experience of living in one.

A character is not a list of attributes. A character is a process â€” a history of experiences, reactions, beliefs formed and reformed, relationships woven and frayed, habits worn into grooves by repetition, and the capacity to be surprised by oneself. The art of fiction is to create characters who feel this way: whose actions emerge not from consulting a table but from the accumulated weight of who they have been and what they have endured.

Our system aspires to this. The character model described here is an attempt to give AI agents the information surface they need to inhabit a role with something approaching the depth a novelist brings to a character they have lived with for years.

---

## The Tensor Representation

We model each character as a **tensor set** â€” a multidimensional representation that captures personality, motivation, values, relationships, history, and state across several interrelated dimensions. The term "tensor" is used deliberately: these are not flat lists but structured, multi-axis representations where the meaning and relevance of any single element depends on its relationship to the others and to the current context.

### Core Dimensions

#### 1. Personality Axes

The enduring traits of the character â€” not as fixed numbers but as ranges with central tendencies and contextual variation. A character who is "brave" is not uniformly brave in all circumstances. They may be fearless in physical danger and cowardly in emotional vulnerability. They may be bold among strangers and meek before their mother.

Personality axes include but are not limited to:

- **Temperament**: the character's baseline emotional register â€” melancholic, sanguine, choleric, phlegmatic, or (more usefully) their position on spectra of optimism/pessimism, excitability/steadiness, warmth/reserve
- **Moral orientation**: not a simple good/evil axis but a textured map of what the character values, what they will sacrifice, where their lines are, and where those lines blur
- **Cognitive style**: how the character processes information â€” intuitive or analytical, cautious or impulsive, concrete or abstract, trusting or skeptical
- **Social posture**: how the character positions themselves among others â€” dominant or deferential, gregarious or solitary, performative or private, generous or guarded

Each axis carries a **central tendency** (the character's default), a **variance** (how much they shift under pressure or context), and **contextual triggers** (specific situations that push them toward the extremes of their range).

#### 2. Motivational Structure

What the character wants â€” both overtly and covertly, both consciously and unconsciously. This is modeled as a layered structure:

- **Surface wants**: what the character would say they want if asked â€” the stated goals, the conscious desires
- **Deep wants**: the underlying needs that the surface wants serve â€” the need for belonging that drives the quest for fame, the fear of abandonment that drives the need for control
- **Shadow wants**: the desires the character would deny or is unaware of â€” the part of the peacemaker that craves conflict, the part of the loyalist that fantasizes about betrayal

These layers interact. A character's actions are most coherent â€” and most interesting â€” when their surface and deep wants align. They are most dramatic when the layers contradict each other. The character model must represent both alignment and contradiction, because it is in the friction between layers that the most human moments emerge.

#### 3. Values and Beliefs

The principles the character holds, consciously or not â€” their sense of justice, their assumptions about human nature, their religious or philosophical commitments, their prejudices and biases (both acknowledged and unacknowledged). These are not merely decorative. They actively filter how the character perceives and interprets events.

A character who believes that people are fundamentally selfish will interpret an act of generosity differently than one who believes in innate goodness. The same event, processed through different belief systems, produces different emotional responses, different judgments, different actions. The character model must encode these filters so that Character Agents can process scene information through them.

#### 4. Capacities and Limitations

What the character can do â€” physically, intellectually, socially, creatively. Unlike traditional stat sheets, these are not abstract numbers but contextualized capabilities:

- Physical capacities: strength, endurance, agility, health, sensory acuity â€” but also physical habits, comfort with violence, relationship to their own body
- Intellectual capacities: areas of knowledge, problem-solving style, linguistic facility, memory, attention
- Social capacities: persuasion, deception, empathy, intimidation, humor, the ability to read a room, the ability to comfort, the ability to provoke
- Creative capacities: artistic skill, improvisational ability, lateral thinking, craftsmanship

These matter for the Reconciler's work in resolving conflicts and physical encounters, but they also matter for character expression. A character with keen observational skills notices things others miss. A character with a gift for humor deflects tension with a joke. These are not just mechanical attributes; they are aspects of personality expressed through capability.

---

## The Relational Web

No character exists in isolation. The relational web is a graph of connections between characters, where each edge carries its own multidimensional state:

- **Trust**: how much one character trusts another â€” not as a single number but as a textured assessment: trust in competence vs. trust in intentions vs. trust in loyalty
- **Affection**: the warmth, love, fondness, or indifference between characters â€” again, textured: one may feel deep affection but no respect, or respect without warmth
- **Debt**: obligations, favors owed, promises made â€” the economy of interpersonal commitment
- **Power**: the dynamic of authority, influence, dependency â€” who holds power over whom, and in what domains
- **History**: the weight of shared experience â€” what they have been through together, what they have done to each other, what remains unresolved
- **Projection**: what one character assumes or believes about another, which may differ significantly from reality â€” the idealized mentor, the demonized rival, the underestimated sibling

The relational web is maintained by the Storykeeper and provided to Character Agents in filtered form â€” each character sees the web from their own perspective, colored by their own biases and information state. Maren's view of her relationship with the player-character is not the same as the player-character's view of their relationship with Maren, and neither may be fully accurate.

---

## Temporal Dynamics: The Geology of Character

Character is not static. It accumulates. And the way it accumulates is not a simple ledger of additions and subtractions. It is geological.

### Layers of Time

**The topsoil** â€” recent experiences, fresh emotions, immediate reactions. These are volatile. Today's anger may be tomorrow's regret. A kind word in the morning shifts the character's mood for the afternoon. The topsoil is responsive, mutable, easily shaped.

**The sediment** â€” patterns that have built up over weeks, months, years. A character who has been consistently betrayed develops a sedimentary layer of distrust. A character who has been loved well develops a sedimentary warmth. These layers are not immutable, but they resist change. A single act of kindness does not dissolve years of betrayal. A single betrayal does not erase years of love â€” though it may crack the surface, exposing something raw beneath.

**The bedrock** â€” the deepest, oldest patterns. Childhood experiences. Foundational traumas and joys. The beliefs about the world that formed before the character had language to question them. Core identity. These are extraordinarily resistant to change. They are the *ruts* â€” grooves worn so deep by time and repetition that experience flows along them almost automatically. A character raised in poverty may hoard resources long after becoming wealthy. A character who was abandoned in childhood may sabotage intimacy in adulthood. The bedrock is not destiny, but it is gravity.

### Decay and Persistence

Emotional states decay over time. The sharp grief of a loss softens. The flush of a victory fades. The system must model this decay â€” not as a simple linear fade, but with a shape that reflects how emotion actually works:

- **Acute emotions** (anger at a specific insult, joy at a specific event) decay relatively quickly, with a half-life measured in scenes or sessions
- **Sustained emotions** (ongoing grief, prolonged anxiety, deepening love) decay slowly, reinforced by continued exposure or revisitation
- **Pattern emotions** (the sedimentary and bedrock layers) barely decay at all under normal circumstances â€” they are the character's emotional climate, not its weather

### The Echo Phenomenon

Old, decayed emotional states can be reactivated by specific stimuli â€” what we call **echoes**. A character who lost a child decades ago and has largely processed that grief may find it suddenly, overwhelmingly present when they hear a particular lullaby. A character who was humiliated as a young person may find that humiliation flooding back in a situation that rhymes with the original.

Echoes are triggered when a current experience maps onto a historical one with sufficient similarity across relevant dimensions â€” sensory, emotional, relational, thematic. The system must be able to identify these resonances and, when they occur, temporarily elevate the old emotional state, allowing it to influence the character's behavior in ways that may surprise both the player and, narratively, the character themselves.

This is one of the great pleasures of fiction: the moment when a character does something that seems disproportionate or unexpected, and we understand, a beat later, that it was not about *this* situation at all â€” it was about something older, deeper, unresolved. The echo mechanism makes this possible systematically.

### Off-Screen Propagation

Characters exist in the story even when they are not in the current scene. Events propagate through the relational web â€” a betrayal committed in one scene may reach a character in another through chains of relationship and communication, attenuated by distance and filtered through the perspectives of intermediaries.

The Storykeeper manages this propagation, updating character tensors between scenes based on what would plausibly reach each character through their relational connections. The propagation model considers:

- **Relational distance**: how many steps in the relationship graph separate the character from the event
- **Communication plausibility**: would the intermediary characters actually pass this information along? A gossipy friend certainly would. A discreet ally might not.
- **Temporal feasibility**: has enough time passed for the information to have traveled?
- **Distortion**: information changes as it passes through intermediaries, colored by their own biases and interests â€” the game of telephone is real, and the system should model it
- **Reception context**: what is the character dealing with when this information arrives? A character in crisis may barely register a minor betrayal. A character in a stable, attentive state may fixate on it.

When a character is next instantiated as a Character Agent, their tensor reflects these off-screen updates. The player encounters a character who has been *living* in the story even when not on stage â€” whose attitudes may have shifted, whose knowledge may have grown, whose emotional state may have changed, all for reasons the player must discover through interaction.

---

## Context-Dependent Activation

Not all dimensions of a character's tensor are equally relevant in every situation. The system must model **contextual activation** â€” the process by which certain traits, motivations, relationships, and emotional states become foregrounded or backgrounded depending on the demands of the current scene.

A character meeting an old rival activates their competitive instincts, their history with that person, their desire to prove themselves â€” while their tender relationship with their child, their love of gardening, their secret fear of deep water, all recede to the background. They are still *there* â€” they are still part of who this character is â€” but they are not operative in this moment.

Contextual activation is determined by:

- **Scene properties**: the setting, the other characters present, the emotional register, the stakes
- **Character state**: current emotional condition, what they are preoccupied with, what they are carrying from recent events
- **Relational triggers**: the presence of specific characters activates specific relational dimensions
- **Thematic resonance**: if the scene engages themes that connect to the character's deep motivations or bedrock beliefs, those layers are activated even if they might otherwise be dormant

The Character Agent receives not the full tensor but a **contextually activated subset** â€” the dimensions that matter right now, weighted by their current relevance. This keeps the agent's decision space manageable while preserving the possibility that an unexpected connection (an echo, a thematic resonance) brings something surprising to the surface.

---

## The Player-Character Tensor

The player-character has a tensor too â€” but it functions differently. The player makes their own decisions; the system does not model their personality in order to predict their behavior. Instead, the player-character tensor tracks:

- **Relational state**: how other characters feel about the player-character, based on accumulated interactions
- **Reputation**: what the player-character is known for, what others expect of them â€” which may differ from what the player intends
- **Information state**: what the player-character has learned, discovered, been told, been misled about
- **Physical and material state**: health, resources, possessions, location
- **Emotional impressions**: while we do not model the player's emotions, other characters form impressions of the player-character's emotional state based on their actions and words â€” a character who consistently acts with compassion will be perceived as compassionate, regardless of the player's internal motivations

The player-character tensor is, in a sense, the world's model of the player â€” the accumulated impression left by their choices. It is the mirror the story holds up.

---

## On the Relationship Between Model and Performance

The tensor representation is not the character. It is the information surface from which a Character Agent constructs a performance. The quality of that performance depends on the agent's ability to synthesize multiple dimensions â€” to feel the contradiction between loyalty and resentment, to let a buried fear color an ostensibly confident action, to surprise.

We do not expect the tensor to *determine* behavior mechanistically. A character with high loyalty and moderate resentment toward another character might respond to a provocation in many ways, all of them consistent with their tensor. The agent chooses â€” and in that choice, the character comes alive.

The tensor provides the landscape. The agent walks through it. And sometimes, if the landscape is rich enough and the agent is good enough, they find a path that no one â€” not the story designer, not the system architect, not the player â€” anticipated. A path that is surprising and yet, in the light of everything the character has been and carried, inevitable.

That is the goal.

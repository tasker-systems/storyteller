# The Narrative Graph: Scenes, Gravity, and the Shape of Stories

## On the Inadequacy of Trees

The conventional model for interactive narrative is the branching tree. A node presents a choice. The choice leads to two or more branches. Each branch leads to another node, another choice, more branches. The tree grows exponentially, and the designer either prunes it ruthlessly (funneling all paths back to a handful of predetermined outcomes) or drowns in combinatorial explosion.

This model is not wrong, exactly. It captures something real: stories do involve choices, and choices do lead to different outcomes. But the tree metaphor encodes assumptions that do not match the felt experience of narrative:

- That every choice point is structurally equal
- That the act of choosing cleaves reality into parallel tracks of comparable weight
- That narrative is fundamentally about divergence â€” forking into separate realities

Our experience of stories, and of life, suggests something different. Most choices are small. Their consequences are local. They shape the texture of the journey but not its destination. And then, sometimes, a choice â€” or a convergence of small choices â€” reaches a moment that *matters*, a moment with narrative mass, and the story bends around it.

We do not model narrative as a tree. We model it as a **gravitational landscape** â€” a topography of scenes with varying narrative mass, connected not by rigid branches but by fluid pathways that the player traverses carrying accumulated state.

---

## The Scene as Gravitational Body

A scene in our system is not a node in a flowchart. It is a region in N-dimensional narrative space with its own properties, its own mass, its own conditions of approach and departure.

### Scene Properties

Every scene carries:

**Narrative mass** â€” the gravitational weight of the scene. High-mass scenes are attractors: the story bends toward them. They are the pivotal moments, the revelations, the confrontations, the quiet turning points where everything shifts. Low-mass scenes are connective tissue: meaningful in the moment, contributing to the journey's texture, but not exerting pull on distant storylines.

Narrative mass is not the same as dramatic intensity. A quiet scene â€” a conversation on a hillside, a moment of unexpected tenderness â€” may have enormous narrative mass if it is the hinge on which a relationship turns. A spectacular battle may have relatively low mass if it is one of many and does not fundamentally alter the story's trajectory. Mass is about structural significance, not volume.

**Approach vectors** â€” the set of possible paths by which a player might arrive at this scene. Each approach vector carries implicit information about what the player has experienced, learned, felt, and chosen on the way. The scene must be designed to accommodate multiple approach vectors while maintaining its essential character.

A betrayal scene, for instance, must work whether the player arrives trusting the betrayer deeply, suspecting them already, or feeling indifferent to them. The scene is the same structural event â€” the betrayal occurs â€” but its emotional texture, its narrative weight, the momentum it imparts, all depend on the approach vector. The system must represent this.

**Departure trajectories** â€” what becomes possible after the scene, and what is foreclosed. A scene may open pathways (the betrayal reveals a conspiracy; pursuing it leads to an entirely new strand of the story) or close them (the character who might have been an ally is now an enemy, and the help they would have offered is no longer available). Departure trajectories interact with the player's accumulated state: two players leaving the same scene may face different sets of possibilities based on what they carry.

**Required presences** â€” which characters must be in this scene for it to function. Some scenes are defined by specific character interactions; without those characters, the scene cannot occur. The Storykeeper must ensure that narrative conditions bring these characters into proximity.

**Contingent presences** â€” characters who might be in the scene depending on prior events. If the player has befriended a particular character, that character may appear in scenes they would otherwise be absent from, adding layers of interaction the story designer has anticipated but not mandated.

**Information gates** â€” what information this scene can reveal, and under what conditions. Some gates are structural: the scene always reveals a particular truth. Others are conditional: the truth is available only if the player asks the right question, notices the right detail, or has earned a character's trust sufficiently to receive it.

**Thematic register** â€” which of the story's deeper themes this scene engages. A story about the cost of loyalty might have scenes that explore loyalty's beauty, its burden, its perversion into blind obedience, its absence. The thematic register helps the system track which themes have been surfaced and which remain latent, contributing to the thematic resonance dimension of narrative space.

**Tonal signature** â€” the aesthetic and emotional register of the scene: tense, elegiac, comic, intimate, grand, quiet. This informs the Narrator's voice for the scene and helps the Reconciler manage the emotional dynamics of multi-character interactions.

---

## The Space Between Scenes

If high-mass scenes are the stars of the narrative galaxy, the space between them is not empty â€” it is the interstellar medium through which the player travels, and it has its own properties.

### Connective Tissue

Between gravitational scenes, the player moves through **connective space** â€” moments of exploration, conversation, discovery, rest, and minor choice. This is where much of the player's sense of agency lives. In connective space, the player can:

- Explore the world â€” examining environments, discovering details, building their understanding of the setting
- Interact with characters in low-stakes contexts â€” building relationships, gathering information, enjoying the texture of the world
- Make minor choices that accumulate â€” choosing to be kind or brusque, cautious or bold, curious or indifferent, where individual choices are small but their pattern shapes the player-character tensor
- Rest, reflect, and prepare â€” the narrative equivalent of a long breath between intense passages of music

Connective space is not filler. It is where the player develops their relationship with the world, where emotional states settle or shift, where the approach vector for the next gravitational scene is shaped. A player who spends connective time building trust with a character will arrive at a betrayal scene carrying a fundamentally different emotional load than one who rushed through.

The design challenge is to make connective space feel responsive and alive without over-determining it. The player should feel free, not herded. The Narrator should be present and engaged, not merely waiting for the player to trigger the next major scene. This is where the Narrator's character matters most â€” a good narrator makes even the quiet moments feel inhabited.

### Drift and Pull

As the player moves through connective space, they are subject to **narrative drift** and **gravitational pull**.

**Narrative drift** is the natural movement of the story based on accumulated state. If the player has been building a relationship with a character, narrative drift moves them toward scenes where that relationship deepens or is tested. If they have been investigating a mystery, drift moves them toward scenes of revelation or complication. Drift is gentle. It does not force; it suggests. It is the Storykeeper recognizing that the player's accumulated choices create momentum, and allowing that momentum to shape what opportunities arise.

**Gravitational pull** is the attraction exerted by high-mass scenes. As the player's state and position bring them into the "gravitational well" of a pivotal scene, the pull increases. Events conspire â€” not through railroading, but through the natural consequences of the story's structure â€” to draw the player toward the scene. An NPC mentions a rumor. A letter arrives. A door that was locked is now open. The world signals that something important is near.

The interplay of drift and pull creates a navigation experience that feels both free and shaped â€” the player is choosing their path, but the landscape has contours. Rivers do not flow uphill, and stories do not drift away from their most important moments without force.

---

## Scene Topology

### Attractor Basins

Each high-mass scene creates an **attractor basin** â€” a region of narrative space within which the story naturally tends toward that scene. The basin is not a fixed radius; its shape is determined by the scene's properties and the current state of the narrative.

A scene that requires a specific emotional state to land properly has a basin shaped by emotional dimensions â€” the player must be carrying sufficient trust, or grief, or curiosity, for the scene to work. A scene gated by information has a basin shaped by knowledge â€” the player must have learned certain things for the scene to become accessible. A scene dependent on relational state has a basin shaped by the web of character connections.

Players can, through their choices, move out of an attractor basin. This is not failure; it is the story adapting. If a player consistently avoids the conditions that would draw them toward a particular scene, the Storykeeper must recognize this and adjust â€” perhaps the scene's conditions are met later, perhaps they are never met and the story flows around that scene like water around a stone, finding other channels.

Not every scene must be reached. Some scenes are contingent â€” they exist as possibilities that the story design makes available but does not require. The richness of the system lies partly in the knowledge that each playthrough encounters only a subset of the total scene space, and different playthroughs encounter different subsets, shaped by different choices.

### Convergence Points

Certain scenes function as **convergence points** â€” moments where multiple pathways through the narrative space converge. These are often the highest-mass scenes, the climactic moments, the revelations that everything has been building toward.

Convergence points must be designed with particular care, because they must accommodate the widest variety of approach vectors. A player who arrives at the final confrontation having befriended every possible ally has a fundamentally different experience than one who arrives alone, having alienated everyone. Both must find the scene meaningful.

This is where the N-dimensional model pays its deepest dividends. The convergence point is not a single scripted scene but a region in narrative space â€” a set of core elements (the confrontation occurs, the truth is revealed, the choice is made) that remain constant, surrounded by contextual elements that vary based on the player's approach vector. The scene adapts to what the player carries, and the adaptation is what makes it feel personal rather than predetermined.

### Branching and Merging

Our model does not eliminate branching â€” it contextualizes it. Branches exist, but they are not all equal. Some branches represent major divergences that create genuinely different narrative tracks (the player chooses to join the rebellion or betray it; these lead to substantially different story experiences). Others represent local variations that merge back into common narrative space within a scene or two.

The system must distinguish between:

- **Hard branches**: genuine divergences that create distinct narrative tracks, each with their own scene topology. These are expensive to author and should be used sparingly, for choices that are truly momentous.
- **Soft branches**: variations in texture, tone, and detail that arise from player choices but converge on the same structural points. These are the most common form of branching and can be handled largely through the approach vector system â€” the scene adapts to how you arrived rather than being a wholly different scene.
- **Delayed branches**: choices whose consequences are not immediately apparent but surface later, sometimes much later. A kindness shown in the first act may save the player's life in the third. A lie told early may unravel catastrophically late. These are tracked by the Storykeeper and activated when conditions align.

---

## Scene Design: The Author's Work

The story designer authors scenes not as scripts but as **possibility spaces**. A scene definition includes:

- The core event or dynamic that defines the scene (what *must* happen for this to be this scene)
- The characters involved (required and contingent)
- The emotional register and tonal signature
- The approach vectors the designer has anticipated, with notes on how each shapes the experience
- The information gates and their conditions
- The departure trajectories and their consequences
- Connections to the broader thematic architecture

The designer does not script dialogue. They do not dictate the precise sequence of events. They define the gravitational properties of the scene â€” its mass, its shape, its conditions, its connections â€” and trust the system's agents to inhabit it.

This is the act of design as landscape architecture: creating the terrain through which rivers will flow, without dictating the path of any particular drop of water.

---

## Temporal Dimensions

The narrative graph is not static. It exists in time, and time in our system has its own properties.

### Story Time and Play Time

Story time and play time are not the same. A scene that takes five minutes to play may represent an hour of story time. A journey that takes days in the story may be compressed to a brief narrative passage. The system must manage this disjunction, and the Narrator must signal temporal transitions clearly so the player maintains their sense of when and where they are.

### Parallel Timelines

Events happen simultaneously in the story world. While the player is in one scene, other characters are living their lives in other scenes â€” making choices, having conversations, receiving information, changing. The off-screen propagation model described in the Character Modeling document manages this, but the narrative graph must also represent it: events have timestamps, and the Storykeeper must track what has happened *when* to maintain consistency.

### Flashbacks and Memory

The temporal dimension also allows for non-linear narrative devices. A scene may include a flashback â€” a moment where a character's memory surfaces, revealing past events that illuminate the present. These are not separate scenes in the graph; they are properties of current scenes, triggered by echoes or by the thematic resonance of the moment.

The system may also support player-initiated retrospection: the player asks "what happened here before?" or "what do I remember about this place?" and the Narrator, drawing on the information ledger, provides what the player-character would plausibly know or recall, filtered through the emotional state of recollection. Memory in our system, as in life, is not a recording. It is a reconstruction, colored by present feeling and past experience.

---

## The Narrative Graph as Living System

We do not conceive of the narrative graph as a fixed structure that the player moves through. It is a living system â€” designed by the story author, maintained by the Storykeeper, shaped by the player's passage through it.

Every playthrough creates a unique path through the graph. Some scenes are visited; others are not. Some relationships deepen; others wither. Some truths are discovered; others remain hidden. The story that emerges is not the story the designer wrote â€” it is the story that happened when a particular player, carrying particular choices and experiences, moved through the gravitational landscape the designer created.

This is, perhaps, the closest analogy to what a GM does in a tabletop game: they prepare a world, populate it with characters and situations and secrets, and then they *play* â€” adapting, responding, discovering alongside their players what story emerges from the collision of preparation and improvisation.

Our system aspires to automate not the creativity of the GM, but the infrastructure that supports it â€” the record-keeping, the state-tracking, the consistency maintenance â€” so that the creative energy can flow freely through agents and player alike, constrained only by the shape of the world and the weight of choices already made.

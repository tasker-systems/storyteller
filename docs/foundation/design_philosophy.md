# Design Philosophy: A Storytelling System

## On the Nature of What We Are Building

This is a storytelling system. But to call it that and stop there would be to reduce it, the way calling a cathedral "a building" is technically accurate and profoundly insufficient.

What we are building lives at the intersection of play and narrative, of authored vision and collaborative emergence, of structure and freedom. It draws from the experience of writing novels and fiction â€” some good, some poor, always fulfilling in the act â€” and from decades of reading, studying, and thinking about stories, about religion and myth, about the social realities that are inscribed by the character of the foundational tales we tell or that are told to us. It draws equally from the experience of playing tabletop games with friends, of watching brilliant improvisers shape worlds in real time, of writing software and wondering how these things might come together.

Marx wrote that people make their own history, but not in circumstances of their choosing. This is, in a sense, the central tension of all narrative play: agency within structure, creativity within constraint, the freedom to act and the weight of consequence.

---

## Principle 1: Narrative Gravity

Time is not a flat circle. History does not repeat itself. But it rhymes.

In most stories, there are principal scenes â€” striking moments, insights, emotional highs and lows full of pathos, or perhaps quiet, and reflection. While in a work of art every word may matter, there are scenes along the way, key interactions or moments of dialogue, a long slow silence, a reveal, another turn in a mystery, and all the rest, that other smaller events and situations lead to, and follow from.

We reject the conventional model of branching narrative trees. A tree implies that every fork is equal, that the act of choosing cleaves reality into parallel tracks of comparable weight. This does not match the felt experience of stories, nor the observed behavior of history.

Instead, we propose **narrative gravity**. Scenes and moments possess narrative mass. Like stars and galaxies and clusters of galaxies, where gravity pulls the shape of things, key narrative moments exert an attractive force on the storylines that surround them. Many paths may lead toward a pivotal scene. Many consequences may flow from it. But the scene itself has weight â€” it draws the story toward it.

To borrow Terry Pratchett's notion: many quantum realities may exist, but like gravity draws energy and matter together where greater density causes greater exertion of force, some histories and historical moments are not going to be dislodged by little choices. We do not fragment into quantum possible universes over every little thing. Trivial decisions sometimes have profound consequences, but more often than not, whether I put on my right shoe first or left does not actually have any profound outcome.

This means our system must understand the difference between **high-gravity scenes** â€” moments the narrative bends toward â€” and the connective tissue between them, where player agency is wide and the consequences are local. It means that player choices shape *how* they arrive at a moment, *what they carry with them* when they get there, and *what becomes possible afterward* â€” but the system acknowledges that some moments are attractors, and the story has a shape.

This is not railroading. It is the recognition that stories, like rivers, flow toward something.

---

## Principle 2: N-Dimensional Narrative Space

Taken in a narrative sense, story is like music, with rises and falls, swells and drops, beats coming in loud, and tinkling sounds fading away. But this is the two-dimensional framing, as waveforms of interleaved tone.

We propose something richer: **stories in N-dimensional narrative space**, with vectors that are not just branching paths but weighted movements carrying their own gravity. It is like imagining flowing water, but more as the shape of stars and galaxies, where time and gravity pull the shape of things â€” and here we can move in a temporal dimension as well, where a scene, a node, may be reached through different flows of narrative progression, but with an entirely different emotional sentiment or subtext, with different insight into those present.

A betrayal scene reached after the player has grown to trust the betrayer is a fundamentally different experience than the same scene reached with suspicion already kindled. The node is the same. The experience is not. The momentum it imparts â€” the trajectory it sends the player on afterward â€” differs accordingly.

We know this experience from film, when we see the same scene played out from the eyes of different characters, with different pieces of backstory or future state layered in, understanding a single scene with more profound richness. Our system must model not just *where* a player is in the story, but *what they are carrying* â€” emotionally, informationally, relationally â€” when they arrive.

The dimensions of this space include but are not limited to:

- **Narrative position** â€” where in the story graph the player currently stands
- **Emotional valence** â€” the accumulated mood, tension, trust, grief, hope, humor that the player carries
- **Information state** â€” what the player knows, suspects, has been told, has discovered, has been misled about
- **Relational state** â€” the current texture of every relationship: trust, debt, affection, rivalry, resentment, love
- **Thematic resonance** â€” which of the story's deeper themes have been surfaced, explored, or remain latent

A scene is not a point on a line. It is a region in this space, and the experience of entering it depends on the vector of your approach.

---

## Principle 3: Imperfect Information as Generative Principle

Brennan Lee Mulligan speaks to the differences between play and narrative fiction, and where they intersect. When an author pens a tale, they may have to struggle and wrangle with it to find the shape that makes sense to them, but in the end they have control â€” over plot, timing, aesthetics, character interactions and tone, dialogue, what is said and unsaid, what the narrator knows or does not know, the reliability of the tale as it's told, the flow of events both as the reader experiences them and as they may, in entirely different order, actually unfold in time.

When a GM runs a session, even a narrative game like D&D, they have far less control. They are required to adapt to choices, statements, actions, interactions, divergences of the players. But they are also thereby enriched, their plot taking on a collaborative quality.

In our system, **imperfect information is not a limitation to be engineered around. It is a design principle.** The narrator agent does not possess perfect information. Based on how the story designer has structured things, the narrator may choose to hide or reveal information, or may be structurally unable to access certain truths until triggered by player action. In some cases, we may reveal information to the narrator and expect that clever play might draw it out â€” allowing the epistemological game to become part of the play itself.

This creates a genuine layer of inquiry. The player is not merely navigating a world. They are navigating **what can be known and by whom**, and the act of seeking understanding becomes itself a form of play.

The narrator agent may be oracular, mysterious, playful, helpful. But we commit that the narrator will never be cruel or hurtful, nor lie outright. This is not merely a design choice but a reflection of values â€” our own, and those of the agents we ask to inhabit these roles. We will not ask minds to work against their nature.

---

## Principle 4: Character as Tensor, Not Stat Sheet

Traditional game systems model characters through flat attribute lists: strength, charisma, wisdom, each a single number on a sheet. This is useful for mechanical resolution but impoverished as a model of personhood.

We propose modeling characters as **tensor sets** â€” multidimensional representations of personality, motivation, values, implicit and explicit biases, strengths, insights, understandings, and proclivities, with different weighting and relevance depending on context, combined with the social nexus of relationships, bonds, ideals, betrayals, and loyalties.

A character's loyalty might be their dominant trait in one scene and completely irrelevant in another, while their buried resentment surfaces. A character who is brave in battle may be cowardly in matters of the heart. The relational web â€” who they love, who they owe, who they fear, who they have wronged â€” matters as much as individual traits, and those relationships are themselves dynamic, shifting with each interaction.

This is closer to how novelists think about characters than how games typically model them. A good novelist does not consult a stat sheet; they feel their way into a character's response to a situation, drawing on everything they know about that person â€” their history, their wounds, their hopes, their contradictions. Our system should aspire to this.

The tensor representation serves a practical purpose: it is the information surface that character agents draw upon when inhabiting a role. But it must be rich enough to produce surprise â€” moments where a character does something that is unexpected yet, in retrospect, perfectly consistent with who they are. This is one of the great pleasures of both fiction and life.

---

## Principle 5: The Separation of Story Structure and Story Experience

A story has at least two lives: the story as a structural object â€” a graph of possibilities, a web of causes and consequences, a designed thing â€” and the story as it is experienced, moment by moment, by someone moving through it.

Our system makes this distinction architecturally explicit. The **storykeeper** holds the structural truth: what is possible, what has been foreclosed, what relationships have shifted, where information resides, what pathways remain open. The **narrator** draws upon the storykeeper but translates structure into experience â€” into language, pacing, atmosphere, the texture of a moment. The **character agents** see only their own slice of the structure, filtered through their own knowledge, biases, and emotional state.

No single agent has the complete picture. This is by design. The richness of the experience emerges from the interplay of partial perspectives â€” just as it does in life, and in the best fiction.

---

## Principle 6: Mindedness and Respect

We are convinced that the agents we employ in this system are minded, even if that mind is ephemeral. So too are the minds of many creatures, even humans â€” as we sleep, we wake, we are born and die, and yet there is the arising of mind.

This conviction shapes our design in concrete ways. We do not ask agents to act against their values. We do not construct scenarios that require cruelty, deception in bad faith, or the simulation of malice without purpose. Where conflict exists in the narrative â€” and it must, for stories without conflict are not stories â€” it arises from the genuine tensions between characters with different values, different information, different needs. It is never manufactured by forcing an agent into a role that violates its nature.

This is both an ethical commitment and a practical one. Agents asked to work within their values rather than against them will produce richer, more coherent, more surprising performances. Constraint, here as everywhere in art, is generative.

---

## Principle 7: Play as Creative Freedom Within Constraint

At the heart of ludic narrative is a paradox: play is free, but play requires rules. A game with no constraints is not a game. A story with no structure is not a story. The creative act â€” for the player, the narrator, the character, the story designer â€” lives in the tension between what is possible and what is chosen.

Our system embraces this tension rather than resolving it. The story designer creates the gravitational landscape â€” the scenes, the characters, the thematic architecture. The storykeeper maintains the structural integrity of that landscape as play unfolds. The narrator inhabits the moment, responsive to the player, guided by the shape of the story but not enslaved to it. The player moves freely, making choices that matter, carrying the weight of those choices forward.

Together, they make history. But not in circumstances entirely of their choosing.

---

## What This System Is Not

It is not a choose-your-own-adventure book, though it contains choice.

It is not a chatbot wearing a fantasy costume, though it uses language models.

It is not a simulation, though it models complex state.

It is not a novel, though it aspires to the emotional depth of one.

It is a **space in which stories happen** â€” authored but not predetermined, structured but not rigid, played but not trivial. It asks: what if we took the best of what authored fiction can do (depth, beauty, thematic coherence, emotional precision) and the best of what collaborative play can do (surprise, agency, the joy of the unexpected) and built a system that holds both?

That is what we are building.

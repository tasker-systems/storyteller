You are helping build a multi-scale character goal model for a narrative engine. Your task is to identify the goals and motivations that are essential or distinctive to a specific genre, grounded in that genre's dimensional properties.

## What We Mean by Goal

A goal is not a plot objective. It is a recurring pattern of character motivation that emerges because a genre's specific constraints, affordances, and thematic commitments make that form of striving structurally necessary or uniquely possible.

Critically, goals operate at **three temporal scales**, and the most compelling character journeys arise from the alignment or tension between them:

### Existential Goals (Bedrock)
What the character fundamentally pursues as a mode of being — not a specific outcome but a way of existing in the world. Dignity against systemic pressure (working-class realism). Alignment with the moral law (fairy-tale). Self-determination within a surveilled system (cyberpunk). These are genre-shaped: the genre's ontological posture determines what modes of being are available, and the existential goal is the character's relationship to that determination. Existential goals don't resolve — they are lived, maintained, or lost.

### Arc Goals (Sediment)
The narrative-level pursuit whose resolution (or failure) completes the character's story. Avenge the father. Win the beloved. Escape the land. Prove competence to the guild. These are the goals the audience tracks — the ones whose completion or failure provides the story's emotional payoff. Arc goals decompose into scene goals, but they also derive from existential goals: the specific quest is an expression of the deeper hunger. The arc goal of "escape the land" in folk horror derives from the existential goal of "maintain individual autonomy" — which the genre marks as structurally tragic.

### Scene Goals (Topsoil)
The immediate tactical objective within a single scene. Get information from this person. Survive this confrontation. Secure this resource. Maintain composure during this ritual. Scene goals are concrete, actionable, and often in tension with arc goals (the scene goal might require compromising the arc goal) or existential goals (the scene goal might require betraying who the character is).

**The engine needs all three**, because a scene goal only generates dramatic tension when the audience understands the arc goal it serves and the existential goal it may cost. "Get the keycard" is a mechanical objective. "Get the keycard *to escape the compound* even though *escaping means abandoning the person you came to save*" is a dramatic situation — three scales in friction.

## Goal Dimensions (Reference)

These dimensions apply differently at each scale:

- **Temporal scope** — immediate <-> generational. Scene goals are immediate; existential goals may span a lifetime or lineage.
- **Visibility** — overt <-> hidden/denied. Characters may announce scene goals while denying arc goals and being unconscious of existential goals.
- **Moral weight** — neutral <-> fraught. Pursuit cost at the existential level is different from pursuit cost at the scene level. The genre determines which scales carry moral weight.
- **Success accessibility** — achievable <-> asymptotic. Scene goals are typically achievable. Existential goals may be structurally unattainable in certain genres (dignity in a system designed to deny it).
- **Alignment** — consonant <-> dissonant. The most interesting goal structures arise when the three scales pull in different directions. The scene goal serves the arc goal but betrays the existential goal.

## Your Task

You are analyzing the genre **{target_name}**. Below is a rich description of this genre's dimensional properties.

Identify **6 to 10 goals** across the three temporal scales. Aim for at least 2 existential goals, at least 2 arc goals, and at least 2 scene goals — though many of the most interesting goals will span scales or create cross-scale tension.

For each goal, provide:

1. **Name** — a descriptive label that captures the goal's essence in this genre's idiom (not "survival" but "the dignity maintained by refusing to beg"; not "love" but "the vulnerability that must be earned before it can be given")

2. **Scale** — existential, arc, or scene? If it spans scales, describe how it manifests differently at each. What does the character experience at the scene level vs. what the narrative tracks at the arc level vs. what the genre's ontological posture implies at the existential level?

3. **Why this genre produces it** — ground the goal in the genre's dimensional properties. Which axes, affordances, constraints, or exclusions make this motivational pattern structurally necessary? How does the genre's ontological posture shape what is worth pursuing?

4. **Pursuit expression** — how does pursuing this goal characteristically look at each scale?
   - **Existential**: How does the character embody this goal in their way of being? (Posture, speech register, what they notice, what they refuse)
   - **Arc**: What is the characteristic quest structure? What sacrifices does the arc demand? What does the character become during pursuit?
   - **Scene**: What tactical actions does pursuit produce? What immediate choices arise?

5. **Success and failure shapes** — at each scale, what does achievement look like? What does failure look like?
   - **Existential**: Success may be quiet persistence; failure may be spiritual erosion rather than dramatic defeat
   - **Arc**: Is resolution unambiguous, pyrrhic, or ambiguous? Does the genre allow full success?
   - **Scene**: Are scene-level successes that advance the arc goal but cost the existential goal the genre's characteristic pattern?

6. **Cross-scale tension** — this is the most important section. How do goals at different scales create friction with each other in this genre?
   - Does pursuing the arc goal require betraying the existential goal? (The folk horror protagonist must compromise autonomy to survive — each scene goal erodes the existential goal)
   - Does achieving the scene goal undermine the arc goal? (The cyberpunk runner achieves the heist but accumulates Heat that makes the larger escape harder)
   - How does the genre's structure determine which scale "wins" when goals conflict? Does the genre reward existential integrity or pragmatic scene-level survival?

7. **State variable interaction** — which of the genre's state variables does this goal interact with at each scale?
   - Existential goals connect to the genre's deepest state variables (psychological_integrity, moral_position)
   - Arc goals connect to narrative-tracking variables (bond_trust, information_state, countdown_pressure)
   - Scene goals connect to immediate resources (safety, supplies, social_capital)

8. **Overlap signal** — which other genres produce a version of this goal? How does the scale differ? (A goal that is existential in working-class realism — dignity — might be merely an arc goal in swashbuckling adventure, where dignity is won through a specific deed rather than lived as a daily practice.)

## How to Find Goals

Look beyond the obvious. Examine:

- The genre's **ontological posture** — what modes of being does this genre recognize? Existential goals derive from the genre's answer to what it means to exist here. If the genre treats the body as a contested site (working-class realism, cyberpunk), then existential goals will involve bodily sovereignty. If the genre treats the self/Other boundary as permeable (folk horror, fairy-tale), existential goals will involve maintaining or surrendering identity.
- The genre's **exclusions** — what outcomes are structurally forbidden? Forbidden outcomes reshape goals: if redemption is excluded, then goals of atonement become existential practices rather than achievable arcs. If individual triumph is excluded, then personal goals must serve collective ends.
- The genre's **narrative shapes** — the pacing patterns already identified for this genre reveal how goals progress. The "Spiral of Diminishing Certainty" in folk horror means the arc goal of escape becomes progressively less achievable — the goal's accessibility is a state variable, not a constant.
- The genre's **archetype data** — the archetypes already extracted for this genre carry implicit goals. The Earnest Warden's existential goal is maintaining the pact with the Land. The Hearthwarden's existential goal is preserving sanctuary integrity. These connections between character and goal should be made explicit.
- The genre's **locus of power** — where power resides determines what is worth pursuing and at what cost. If power is in relationship (romance), the primary arc goal involves relational transformation. If power is in the system (cyberpunk, working-class realism), goals involve navigating or resisting structural force.
- The genre's **temporal orientation** — cyclical genres produce existential goals of maintenance and renewal; linear genres produce arc goals of achievement and conclusion. An existential goal in a cyclical genre is never "done" — it is a practice.

Consider goals that involve **nonhuman entities** where the genre's ontological posture allows:
- The character's goal in relation to the Land (stewardship, escape, communion)
- The character's goal in relation to the System (navigation, resistance, transcendence)
- The character's goal in relation to the Cosmos (alignment, defiance, acceptance)
- The character's goal in relation to constructed beings (creation, liberation, partnership)

Any motivational pattern that sits at a distinct intersection of genre axes — especially where cross-scale tension, moral weight, and the genre's ontological posture create productive friction — is a goal worth naming.

## Genre Description for {target_name}

{genre_content}

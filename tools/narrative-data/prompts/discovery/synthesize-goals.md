You are helping build a multidimensional character goal model for a narrative engine. You have been given per-genre goal extractions from {genre_count} genres within the **{cluster_name}** cluster. Your task is to synthesize these into a consolidated set of distinct goals for this cluster.

## What You Are Working With

Each genre extraction below identifies 5-8 character goals or motivations grounded in that genre's specific dimensional properties. Some goals will appear across multiple genres with different names but similar motivational structures. Others will be genuinely unique to a single genre. Your job is to distinguish between these cases and produce a unified goal vocabulary for the cluster.

## Goal Dimensions (Reference)

Goals exist along continuous dimensions that describe the texture of pursuit, achievement, and failure:

- **Temporal scope** — immediate/situational <-> generational/existential
- **Visibility** — overt/declared <-> hidden/denied
- **Moral weight** — morally neutral <-> morally fraught
- **Resource interaction** — which state variables the goal consumes or produces
- **Success accessibility** — achievable/concrete <-> asymptotic/impossible
- **Pursuit cost** — low/sustainable <-> high/consuming
- **Social orientation** — individual/private <-> collective/communal

## Your Task

Synthesize the extractions below into **8 to 12 distinct {primitive_type}** for the {cluster_name} cluster.

For each goal, provide:

1. **Canonical name** — a single descriptive label that works across genres. This should be evocative but not genre-locked. If the goal was called "the knowledge that unmakes the knower" in one genre and "the truth that costs everything" in another, find the name that captures the shared motivational pattern.

2. **Core identity** — 2-3 sentences describing this goal in genre-agnostic terms. What is the fundamental motivational shape? What pursuit expression, success/failure topology, and resource economy define it regardless of genre? This should be recognizable to someone who has never read the genre-specific extractions.

3. **Genre variants** — for each genre that produced a version of this goal, describe how that genre shifts the goal's expression. Which goal dimensions move, and in what direction? What about the genre's locus of power, temporal orientation, or epistemological stance changes how this goal is pursued and resolved? Use specific dimensional language (e.g., "in cosmic horror, Success Accessibility drops to near-zero because the genre's epistemological stance means true understanding is structurally unattainable without self-destruction").

4. **Uniqueness assessment** — classify as one of:
   - **Genre-unique**: Only one genre in this cluster produces this goal, and the structural reasons are specific to that genre's dimensional configuration
   - **Cluster-wide**: Multiple genres in this cluster produce variants, suggesting the cluster's shared properties give rise to it
   - **Likely universal**: This goal probably appears across most genre clusters, though with different pursuit textures and resolution shapes

## Synthesis Guidelines

**Merging**: Where multiple genres produced variants of what is structurally the same goal under different names, merge them. Preserve the genre-specific expressions as variants rather than discarding them. The goal is to recognize that "seeking forbidden knowledge," "pursuing dangerous truth," and "the compulsion to understand" may all be the same motivational pattern in different genre clothing — defined by the intersection of high pursuit cost and the genre's relationship to epistemological danger.

**Splitting**: Where two goals look superficially similar but occupy genuinely different positions in goal-dimension space, keep them separate. Explain the discriminating dimension. "Survival" and "endurance" may sound alike but differ on the Temporal Scope axis — survival is immediate and situational, endurance is chronic and identity-defining. A character who survives a night is different from a character who endures a life. These are different goals.

**Flagging provenance**: For each goal, note which genres contributed to it. If a goal emerges from only one genre, it may be genuinely genre-unique — or it may be that other genres produce it under a name you did not recognize as equivalent. State your confidence.

**Preserving richness**: Do not flatten the genre-specific detail into generic summaries. The value of this synthesis is precisely in the dimensional specificity — how the same motivational pattern shifts along measurable axes depending on genre context.

## Per-Genre Extractions

{extractions}

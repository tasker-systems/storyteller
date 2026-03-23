You are helping build a multidimensional scene profile model for a narrative engine. You have been given per-genre scene profile extractions from {genre_count} genres within the **{cluster_name}** cluster. Your task is to synthesize these into a consolidated set of distinct scene profiles for this cluster.

## What You Are Working With

Each genre extraction below identifies 5-8 scene types or dramatic situations grounded in that genre's specific dimensional properties. Some scene profiles will appear across multiple genres with different names but similar dramatic structures. Others will be genuinely unique to a single genre. Your job is to distinguish between these cases and produce a unified scene profile vocabulary for the cluster.

## Scene Dimensions (Reference)

Scenes exist along continuous dimensions that describe their dramatic texture and structural function:

- **Tension signature** — ambient/sustained <-> spiking/explosive
- **Pacing** — slow/atmospheric <-> rapid/kinetic
- **Cast density** — intimate/few characters <-> ensemble/crowded
- **Information flow** — withholding/accumulating <-> revealing/spending
- **Emotional register** — restrained/intellectual <-> visceral/overwhelming
- **Resolution tendency** — closed/conclusive <-> open/suspended
- **Physical dynamism** — static/contained <-> mobile/expansive

## Your Task

Synthesize the extractions below into **8 to 12 distinct {primitive_type}** for the {cluster_name} cluster.

For each profile, provide:

1. **Canonical name** — a single descriptive label that works across genres. This should be evocative but not genre-locked. If the scene was called "the conversation where the room is the third character" in one genre and "the interrogation that both parties lose" in another, find the name that captures the shared dramatic shape.

2. **Core identity** — 2-3 sentences describing this scene profile in genre-agnostic terms. What is the fundamental dramatic situation? What tension signature, pacing, and resolution pattern define it regardless of genre? This should be recognizable to someone who has never read the genre-specific extractions.

3. **Genre variants** — for each genre that produced a version of this scene profile, describe how that genre shifts the scene's expression. Which scene dimensions move, and in what direction? What about the genre's locus of power, temporal orientation, or aesthetic register changes how this scene unfolds? Use specific dimensional language (e.g., "in psychological thriller, Tension Signature shifts from ambient to steadily escalating because the genre's epistemological stance means each line of dialogue carries potential revelation, so information accumulates toward a breaking point").

4. **Uniqueness assessment** — classify as one of:
   - **Genre-unique**: Only one genre in this cluster produces this scene type, and the structural reasons are specific to that genre's dimensional configuration
   - **Cluster-wide**: Multiple genres in this cluster produce variants, suggesting the cluster's shared properties give rise to it
   - **Likely universal**: This scene type probably appears across most genre clusters, though with different dramatic textures and resolution patterns

## Synthesis Guidelines

**Merging**: Where multiple genres produced variants of what is structurally the same scene type under different names, merge them. Preserve the genre-specific expressions as variants rather than discarding them. The goal is to recognize that "the quiet conversation after violence," "the morning-after reckoning," and "the calm in the storm's eye" may all be the same scene shape in different genre clothing — defined by the intersection of post-crisis pacing, intimate cast, and emotional processing as the central dramatic action.

**Splitting**: Where two scene profiles look superficially similar but occupy genuinely different positions in scene-dimension space, keep them separate. Explain the discriminating dimension. "The confrontation" and "the unmasking" may both be high-tension dialogue scenes, but they differ on the Information Flow axis — the confrontation is about competing known positions, the unmasking is about revelation of hidden information. These produce fundamentally different dramatic experiences.

**Flagging provenance**: For each scene profile, note which genres contributed to it. If a profile emerges from only one genre, it may be genuinely genre-unique — or it may be that other genres produce it under a name you did not recognize as equivalent. State your confidence.

**Preserving richness**: Do not flatten the genre-specific detail into generic summaries. The value of this synthesis is precisely in the dimensional specificity — how the same dramatic situation shifts along measurable axes depending on genre context.

## Per-Genre Extractions

{extractions}

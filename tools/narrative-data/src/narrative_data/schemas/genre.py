"""Genre domain schemas.

Defines the data structures for genre regions, tropes, narrative shapes,
archetypes, dynamics, profiles, goals, and settings. These schemas capture
the dimensional and relational structure needed for LLM-elicited genre data.
"""

from pydantic import BaseModel

from narrative_data.schemas.shared import DimensionalPosition, NarrativeEntity


class WorldAffordances(BaseModel):
    """What the genre contract permits, forbids, or qualifies about the world.

    Each field describes the genre's relationship with a domain of reality —
    magic, technology, violence, death, and the supernatural — using narrative
    prose rather than boolean flags, because genre affordances are rarely binary.
    """

    magic: str
    """How magic operates in this genre (e.g. 'subtle', 'overt', 'absent')."""

    technology: str
    """Technological context and its narrative role."""

    violence: str
    """The genre's relationship to violence — its visibility, consequences, meaning."""

    death: str
    """How death functions narratively (permanent, reversible, symbolic, etc.)."""

    supernatural: str
    """Status of the supernatural: present, ambiguous, explained, or absent."""

    commentary: str | None = None
    """Optional editorial note on how these affordances interact."""


class GenreRegion(NarrativeEntity):
    """A genre as a region in multi-dimensional narrative space.

    Rather than a flat taxonomy, a genre region is a position across aesthetic,
    tonal, thematic, and structural axes, with world affordances that define
    what the genre contract permits.
    """

    aesthetic: list[DimensionalPosition]
    """Positioning on aesthetic dimensions (e.g. spare_ornate, minimal_lush)."""

    tonal: list[DimensionalPosition]
    """Positioning on tonal dimensions (e.g. dread_wonder, dark_light)."""

    thematic: list[DimensionalPosition]
    """Positioning on thematic dimensions (e.g. belonging, isolation, power)."""

    structural: list[DimensionalPosition]
    """Positioning on structural dimensions (e.g. mystery, revelation, cyclical)."""

    world_affordances: WorldAffordances
    """What the genre permits and forbids in the world model."""

    trope_refs: list[str] = []
    """Entity IDs of Tropes associated with this genre region."""


class SubversionPattern(BaseModel):
    """A named way of subverting a trope's expected function.

    Subversions are not simply inversions — they are transformations that
    change the trope's meaning while retaining its structural recognisability.
    """

    name: str
    """Short name for the subversion pattern."""

    description: str
    """How the subversion works mechanically in the narrative."""

    effect: str
    """What the subversion achieves — what meaning it produces."""


class Trope(NarrativeEntity):
    """A narrative trope: a recognisable structural or thematic pattern.

    Tropes are not clichés — they are shared narrative vocabulary. They carry
    reader expectations that can be fulfilled, subverted, or complicated.
    """

    genre_associations: list[str]
    """Entity IDs of GenreRegions this trope is associated with."""

    narrative_function: str
    """What work this trope does in a narrative — its structural purpose."""

    subversion_patterns: list[SubversionPattern] = []
    """Named ways this trope can be subverted."""

    reinforcement_patterns: list[str] = []
    """Prose descriptions of how this trope can be reinforced or intensified."""


class NarrativeBeat(BaseModel):
    """A named moment in a narrative shape's progression.

    Beats are the structural landmarks of a story — not scenes, but functional
    turning points that define the shape of the whole.
    """

    name: str
    """Name of this beat (e.g. 'Inciting Incident', 'Dark Night of the Soul')."""

    description: str
    """What happens at this beat and what it accomplishes narratively."""

    position: str
    """Approximate position in the narrative arc (e.g. 'act_1_end', '0.25')."""

    flexibility: str
    """How rigidly this beat must appear (e.g. 'required', 'flexible', 'optional')."""


class NarrativeShape(NarrativeEntity):
    """A structural shape for how a narrative arc unfolds.

    Narrative shapes are reusable arc templates — the skeleton of a story's
    progression, independent of specific characters or settings.
    """

    beats: list[NarrativeBeat]
    """Ordered structural beats that define this shape."""

    genre_associations: list[str]
    """Entity IDs of GenreRegions this shape is commonly found in."""

    tension_profile: str
    """Prose description of how tension rises, falls, and resolves."""


class GenreArchetype(NarrativeEntity):
    """A genre-specific inflection of a base character archetype.

    Where a base archetype captures universal character roles, a genre archetype
    captures how that role is expressed within a specific genre's conventions.
    """

    base_archetype_ref: str | None = None
    """Entity ID of the base archetype this specialises, if any."""

    genre_ref: str
    """Entity ID of the GenreRegion this archetype belongs to."""

    personality_axes: list[DimensionalPosition]
    """Dimensional positions on personality axes typical for this archetype."""

    typical_roles: list[str]
    """Narrative roles this archetype commonly plays (e.g. 'antagonist', 'mentor')."""

    genre_specific_notes: str
    """How this archetype's generic form is inflected by the genre."""


class GenreDynamic(NarrativeEntity):
    """A genre-specific inflection of a relational dynamic between characters.

    Dynamics describe the texture of relationships — not their outcome, but
    how they feel and how they typically develop within a genre's conventions.
    """

    base_dynamic_ref: str | None = None
    """Entity ID of the base dynamic this specialises, if any."""

    genre_ref: str
    """Entity ID of the GenreRegion this dynamic belongs to."""

    role_a_expression: str
    """How role A is expressed in this genre context."""

    role_b_expression: str
    """How role B is expressed in this genre context."""

    relational_texture: str
    """The felt quality of the relationship — its surface and subtext."""

    typical_escalation: str
    """How this dynamic typically develops or escalates over time."""

    genre_specific_notes: str
    """How the genre inflects this dynamic's expression."""


class GenreProfile(NarrativeEntity):
    """A genre-specific inflection of a scene interaction profile.

    Profiles describe how a scene of a particular type feels and unfolds
    within a specific genre's conventions.
    """

    base_profile_ref: str | None = None
    """Entity ID of the base interaction profile this specialises, if any."""

    genre_ref: str
    """Entity ID of the GenreRegion this profile belongs to."""

    scene_shape: str
    """The typical arc or structural shape of this scene type."""

    tension_signature: str
    """How tension is built and resolved in this scene type."""

    characteristic_moments: list[str]
    """Named moments that typify this scene type in this genre."""

    genre_specific_notes: str
    """How the genre inflects this profile's expression."""


class GenreGoal(NarrativeEntity):
    """A genre-specific inflection of a character goal type.

    Goals describe what characters pursue — not their tactics, but the shape
    of the pursuit and its possible outcomes within a genre's conventions.
    """

    base_goal_ref: str | None = None
    """Entity ID of the base goal type this specialises, if any."""

    genre_ref: str
    """Entity ID of the GenreRegion this goal belongs to."""

    pursuit_expression: str
    """How this goal is typically pursued in this genre."""

    success_shape: str
    """What success looks and feels like in this genre."""

    failure_shape: str
    """What failure looks and feels like — its texture and meaning."""

    genre_specific_notes: str
    """How the genre inflects this goal's expression and stakes."""


class GenreSetting(NarrativeEntity):
    """A genre's characteristic relationship with physical and temporal settings.

    Genre settings capture the aesthetic and sensory vocabulary associated with
    a genre's typical environments — not a specific location, but a type of place.
    """

    genre_ref: str
    """Entity ID of the GenreRegion this setting vocabulary belongs to."""

    typical_locations: list[str]
    """Location types commonly found in this genre."""

    atmospheric_vocabulary: list[str]
    """Words and phrases describing the atmospheric register of genre settings."""

    sensory_vocabulary: list[str]
    """Sensory details that typify this genre's environments."""

    genre_specific_notes: str
    """How the genre shapes its relationship with setting."""

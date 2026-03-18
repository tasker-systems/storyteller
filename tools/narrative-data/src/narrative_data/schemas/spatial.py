"""Spatial domain schemas.

Defines the data structures for setting types, place entities, topology edges,
and tonal inheritance rules. These schemas capture the dimensional and relational
structure of place-as-entity: locations as active narrative participants with
communicability dimensions.
"""

from pydantic import BaseModel

from narrative_data.schemas.shared import NarrativeEntity


class SettingType(NarrativeEntity):
    """A category of setting with characteristic atmospheric and sensory properties.

    Setting types are not specific locations — they are reusable templates that
    capture how a class of place typically feels, sounds, smells, and functions
    narratively within associated genres.
    """

    genre_associations: list[str]
    """Entity IDs of GenreRegions this setting type is associated with."""

    atmospheric_signature: str
    """The dominant atmospheric quality of this setting type."""

    sensory_palette: list[str]
    """Characteristic sensory details (sights, sounds, smells, textures) for this type."""

    temporal_character: str
    """How time feels in this setting — its pace, layering, and narrative role."""


class SensoryDetail(BaseModel):
    """A single sensory observation about a place.

    Sensory details are the building blocks of place communicability — the
    specific, concrete observations that make a location feel inhabited and real.
    """

    sense: str
    """The sense modality: 'sight', 'sound', 'smell', 'touch', 'taste'."""

    detail: str
    """The specific sensory observation."""

    emotional_valence: str | None = None
    """The emotional register this detail evokes, if notable."""


class CommunicabilityProfile(BaseModel):
    """The four communicability dimensions of a place entity.

    Following the place-as-entity model: places communicate through atmospheric,
    sensory, spatial, and temporal dimensions. Each dimension is a prose statement
    capturing how this place speaks to characters and players on that axis.
    """

    atmospheric: str
    """The dominant mood and psychological register of this place."""

    sensory: str
    """How the place is perceived through the senses — its texture of experience."""

    spatial: str
    """The spatial logic of the place: scale, orientation, paths, boundaries."""

    temporal: str
    """The place's relationship with time: its history, pace, memory."""


class PlaceEntity(NarrativeEntity):
    """A specific location treated as a narrative entity with communicability.

    Place entities are not mere backdrops. They have communicability profiles,
    narrative functions, and sensory vocabularies. They participate in scenes.
    """

    setting_type_ref: str
    """Entity ID of the SettingType this place instantiates."""

    narrative_function: str
    """The structural role this place plays in narrative (e.g. 'threshold', 'sanctuary')."""

    communicability: CommunicabilityProfile
    """How this place communicates across its four dimensions."""

    sensory_details: list[SensoryDetail]
    """Specific sensory details that characterise this place."""


class TopologyEdge(BaseModel):
    """A directed connection between two place entities.

    Topology edges define the navigable relationships between places: how you
    move from one to another, what passes between them, and what changes when
    you cross the threshold.
    """

    edge_id: str
    """Unique identifier for this edge."""

    from_place: str
    """Entity ID of the origin place."""

    to_place: str
    """Entity ID of the destination place."""

    adjacency_type: str
    """The kind of connection: 'doorway', 'corridor', 'staircase', 'open', etc."""

    friction: str
    """Ease of traversal: 'low', 'medium', 'high', or narrative description."""

    permeability: list[str]
    """What passes through this connection (e.g. 'sound', 'light', 'smell')."""

    tonal_shift_note: str | None = None
    """Optional note on how the atmosphere changes when crossing this edge."""


class TonalInheritanceRule(NarrativeEntity):
    """A rule governing how tonal properties propagate between adjacent places.

    Within a setting type, tonal qualities can bleed across boundaries —
    the dread of the cellar seeps into the kitchen above it. Tonal inheritance
    rules capture these propagation patterns.
    """

    setting_type_ref: str
    """Entity ID of the SettingType this rule applies within."""

    rule: str
    """Prose statement of the inheritance rule."""

    applies_across: str
    """The adjacency types or boundaries this rule applies across."""

    friction_level: str
    """How strongly the boundary resists tonal inheritance: 'permeable', 'moderate', 'solid'."""

    examples: list[str] = []
    """Concrete examples of this rule in action."""

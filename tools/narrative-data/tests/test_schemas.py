"""Tests for Pydantic schema validation and JSON Schema export."""

from narrative_data.schemas.genre import (
    GenreArchetype,
    GenreDynamic,
    GenreRegion,
    SubversionPattern,
    Trope,
    WorldAffordances,
)
from narrative_data.schemas.intersections import (
    Enrichment,
    IntersectionSynthesis,
    UpstreamRef,
)
from narrative_data.schemas.shared import (
    DimensionalPosition,
    GenerationProvenance,
    NarrativeEntity,
    ProvenanceEdge,
)
from narrative_data.schemas.spatial import (
    CommunicabilityProfile,
    PlaceEntity,
    SensoryDetail,
    SettingType,
    TopologyEdge,
)


class TestGenerationProvenance:
    def test_valid_provenance(self):
        p = GenerationProvenance(
            prompt_hash="abc123",
            model="qwen3.5:35b",
            generated_at="2026-03-17T20:00:00Z",
        )
        assert p.model == "qwen3.5:35b"
        assert p.source_content_digest is None

    def test_provenance_with_digest(self):
        p = GenerationProvenance(
            prompt_hash="abc123",
            model="qwen2.5:3b-instruct",
            generated_at="2026-03-17T20:00:00Z",
            source_content_digest="sha256:deadbeef",
        )
        assert p.source_content_digest == "sha256:deadbeef"


class TestProvenanceEdge:
    def test_llm_elicited_edge(self):
        edge = ProvenanceEdge(
            source_id="ollama-qwen3.5:35b-run-1",
            source_type="llm_elicited",
            contribution_type="originated",
            weight=1.0,
        )
        assert edge.extractable is True
        assert edge.license is None

    def test_future_cc_by_sa_edge(self):
        edge = ProvenanceEdge(
            source_id="cthulhu-reborn-module-42",
            source_type="cc_by_sa",
            contribution_type="reinforced",
            weight=0.4,
            license="CC-BY-SA-4.0",
            extractable=True,
        )
        assert edge.license == "CC-BY-SA-4.0"


class TestDimensionalPosition:
    def test_bipolar_dimension(self):
        d = DimensionalPosition(dimension="dread_wonder", value=-0.7, note="high dread")
        assert d.value == -0.7

    def test_unipolar_dimension(self):
        d = DimensionalPosition(dimension="intimacy", value=0.3)
        assert d.note is None


class TestNarrativeEntity:
    def test_minimal_entity(self):
        e = NarrativeEntity(
            entity_id="019d0000-0000-7000-8000-000000000001",
            name="Test Entity",
            description="A test entity",
            provenance=GenerationProvenance(
                prompt_hash="abc", model="test", generated_at="2026-03-17T00:00:00Z"
            ),
        )
        assert e.commentary is None
        assert e.suggestions == []
        assert e.provenance_edges == []

    def test_entity_with_commentary(self):
        e = NarrativeEntity(
            entity_id="019d0000-0000-7000-8000-000000000002",
            name="Rich Entity",
            description="Has commentary",
            commentary="This entity could also express isolation themes",
            suggestions=["Consider adding a spatial dimension", "Links to gothic tradition"],
            provenance=GenerationProvenance(
                prompt_hash="abc", model="test", generated_at="2026-03-17T00:00:00Z"
            ),
            provenance_edges=[
                ProvenanceEdge(
                    source_id="run-1",
                    source_type="llm_elicited",
                    contribution_type="originated",
                    weight=1.0,
                )
            ],
        )
        assert len(e.suggestions) == 2
        assert len(e.provenance_edges) == 1

    def test_json_schema_export(self):
        schema = NarrativeEntity.model_json_schema()
        assert "properties" in schema
        assert "entity_id" in schema["properties"]
        assert "provenance" in schema["properties"]

    def test_round_trip_json(self):
        e = NarrativeEntity(
            entity_id="019d0000-0000-7000-8000-000000000003",
            name="Round Trip",
            description="Test serialization",
            provenance=GenerationProvenance(
                prompt_hash="abc", model="test", generated_at="2026-03-17T00:00:00Z"
            ),
        )
        json_str = e.model_dump_json()
        restored = NarrativeEntity.model_validate_json(json_str)
        assert restored.entity_id == e.entity_id
        assert restored.name == e.name


def _provenance():
    return GenerationProvenance(
        prompt_hash="test", model="test", generated_at="2026-03-17T00:00:00Z"
    )


class TestGenreRegion:
    def test_full_genre_region(self):
        r = GenreRegion(
            entity_id="019d0000-0000-7000-8000-000000000010",
            name="Folk Horror",
            description="Rural dread, community as threat",
            provenance=_provenance(),
            aesthetic=[DimensionalPosition(dimension="spare_ornate", value=-0.3)],
            tonal=[DimensionalPosition(dimension="dread_wonder", value=-0.8)],
            thematic=[DimensionalPosition(dimension="belonging", value=0.7)],
            structural=[DimensionalPosition(dimension="mystery", value=0.6)],
            world_affordances=WorldAffordances(
                magic="subtle", technology="historical",
                violence="consequence-laden", death="permanent",
                supernatural="ambiguous",
            ),
        )
        assert r.name == "Folk Horror"
        assert len(r.aesthetic) == 1
        assert r.trope_refs == []

    def test_genre_region_json_schema(self):
        schema = GenreRegion.model_json_schema()
        assert "world_affordances" in schema["properties"]


class TestTrope:
    def test_trope_with_subversion(self):
        t = Trope(
            entity_id="019d0000-0000-7000-8000-000000000011",
            name="The Wicker Man",
            description="Community sacrifices outsider for renewal",
            provenance=_provenance(),
            genre_associations=["019d0000-0000-7000-8000-000000000010"],
            narrative_function="Reveals that community belonging has a price",
            subversion_patterns=[
                SubversionPattern(
                    name="Willing sacrifice",
                    description="The outsider chooses to participate",
                    effect="Transforms horror into tragedy",
                )
            ],
        )
        assert len(t.subversion_patterns) == 1


class TestGenreArchetype:
    def test_genre_archetype(self):
        a = GenreArchetype(
            entity_id="019d0000-0000-7000-8000-000000000012",
            name="The Cunning Elder",
            description="Authority figure hiding community secrets",
            provenance=_provenance(),
            base_archetype_ref="019c0000-0000-7000-8000-000000000001",
            genre_ref="019d0000-0000-7000-8000-000000000010",
            personality_axes=[DimensionalPosition(dimension="trust", value=0.2)],
            typical_roles=["antagonist", "gatekeeper"],
            genre_specific_notes="In folk horror, authority conceals ritual purpose",
        )
        assert a.base_archetype_ref is not None


class TestGenreDynamic:
    def test_genre_dynamic_has_domain_fields(self):
        d = GenreDynamic(
            entity_id="019d0000-0000-7000-8000-000000000013",
            name="Outsider and Community",
            description="Tension between newcomer and established group",
            provenance=_provenance(),
            genre_ref="019d0000-0000-7000-8000-000000000010",
            role_a_expression="Naive investigator drawn by curiosity",
            role_b_expression="Collective voice masking shared purpose",
            relational_texture="Surface warmth concealing assessment",
            typical_escalation="Hospitality → subtle tests → reveal of true nature",
            genre_specific_notes="Folk horror inverts the welcome",
        )
        assert d.relational_texture == "Surface warmth concealing assessment"


class TestSettingType:
    def test_setting_type(self):
        s = SettingType(
            entity_id="019d0000-0000-7000-8000-000000000020",
            name="Gothic Mansion",
            description="Decay, rooms with purpose, verticality",
            provenance=_provenance(),
            genre_associations=["019d0000-0000-7000-8000-000000000010"],
            atmospheric_signature="Oppressive grandeur in decline",
            sensory_palette=["dust", "cold stone", "ticking clock"],
            temporal_character="Time feels slower, layered with history",
        )
        assert len(s.sensory_palette) == 3


class TestPlaceEntity:
    def test_place_entity(self):
        p = PlaceEntity(
            entity_id="019d0000-0000-7000-8000-000000000021",
            name="Entry Hall",
            description="Imposing first impression, threshold to the interior",
            provenance=_provenance(),
            setting_type_ref="019d0000-0000-7000-8000-000000000020",
            narrative_function="threshold",
            communicability=CommunicabilityProfile(
                atmospheric="imposing, watchful",
                sensory="dust, old wood, ticking clock",
                spatial="high ceiling, branching paths",
                temporal="the house remembers who enters",
            ),
            sensory_details=[
                SensoryDetail(sense="sight", detail="faded portraits line the walls"),
                SensoryDetail(
                    sense="sound", detail="floorboards creak", emotional_valence="unease"
                ),
            ],
        )
        assert p.narrative_function == "threshold"


class TestTopologyEdge:
    def test_topology_edge(self):
        e = TopologyEdge(
            edge_id="019d0000-0000-7000-8000-000000000030",
            from_place="019d0000-0000-7000-8000-000000000021",
            to_place="019d0000-0000-7000-8000-000000000022",
            adjacency_type="doorway",
            friction="low",
            permeability=["sound", "light"],
        )
        assert e.tonal_shift_note is None


class TestIntersectionSynthesis:
    def test_intersection_synthesis(self):
        s = IntersectionSynthesis(
            entity_id="019d0000-0000-7000-8000-000000000040",
            name="Folk Horror × Gothic Mansion",
            description="How folk horror transforms the gothic mansion",
            provenance=_provenance(),
            upstream_refs=[
                UpstreamRef(entity_id="id1", content_digest="sha256:aaa", domain="genre"),
                UpstreamRef(entity_id="id2", content_digest="sha256:bbb", domain="spatial"),
            ],
            content_hash="sha256:combined",
            enrichments=[
                Enrichment(
                    target_entity_id="id1",
                    enrichment_type="tonal_refinement",
                    content="Entry hall gains ritual significance",
                )
            ],
            gaps_identified=["No place entity for ritual site"],
            new_entries=[],
        )
        assert len(s.upstream_refs) == 2
        assert len(s.enrichments) == 1

"""Tests for Pydantic schema validation and JSON Schema export."""

from narrative_data.schemas.shared import (
    DimensionalPosition,
    GenerationProvenance,
    NarrativeEntity,
    ProvenanceEdge,
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

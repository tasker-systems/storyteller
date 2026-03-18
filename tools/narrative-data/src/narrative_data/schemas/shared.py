"""Shared base types for all narrative data schemas."""

from pydantic import BaseModel


class GenerationProvenance(BaseModel):
    """Tracks how this entity was generated."""

    prompt_hash: str
    model: str
    generated_at: str
    source_content_digest: str | None = None


class ProvenanceEdge(BaseModel):
    """Attribution edge from source to knowledge node.

    Currently populated only for LLM elicitation. Schema designed
    to support future strategies: public domain analysis, CC-BY-SA
    RPG module extraction, and cross-source synthesis.
    """

    source_id: str
    source_type: str
    contribution_type: str
    weight: float
    license: str | None = None
    extractable: bool = True
    notes: str | None = None


class DimensionalPosition(BaseModel):
    """Weighted position along a named dimension."""

    dimension: str
    value: float
    note: str | None = None


class NarrativeEntity(BaseModel):
    """Base for all generated entities."""

    entity_id: str
    name: str
    description: str
    commentary: str | None = None
    suggestions: list[str] = []
    provenance: GenerationProvenance
    provenance_edges: list[ProvenanceEdge] = []

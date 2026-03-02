-- =============================================================================
-- TAS-242: Sub-Graph Layers Table
-- =============================================================================
-- Narrative sub-graph layers for tales-within-tales. Layers can nest.
-- entry_scene_id FK is deferred to the scenes migration.
--
-- Sigmoid parameters control how boundary permeability evolves over turns.
-- If permeability_min = permeability_max, the boundary is static (backwards
-- compatible with the simple permeability value).
-- =============================================================================

CREATE TABLE sub_graph_layers (
    id                      UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id                UUID NOT NULL REFERENCES stories(id),
    parent_layer_id         UUID REFERENCES sub_graph_layers(id),
    name                    TEXT NOT NULL,
    layer_type              layer_type NOT NULL,
    entry_scene_id          UUID,
    -- Static permeability (used when sigmoid min = max)
    permeability            REAL NOT NULL DEFAULT 0.0,
    -- Sigmoid parameters for dynamic boundary permeability
    permeability_min        REAL NOT NULL DEFAULT 0.1,
    permeability_max        REAL NOT NULL DEFAULT 1.0,
    permeability_steepness  REAL NOT NULL DEFAULT 0.5,
    permeability_midpoint   REAL NOT NULL DEFAULT 50.0,
    -- Collective mass configuration
    base_mass               REAL NOT NULL DEFAULT 0.5,
    completion_bonus        REAL NOT NULL DEFAULT 0.3,
    thematic_resonance      REAL NOT NULL DEFAULT 0.0,
    metadata                JSONB,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sub_graph_layers_story ON sub_graph_layers(story_id);
CREATE INDEX idx_sub_graph_layers_parent ON sub_graph_layers(parent_layer_id);

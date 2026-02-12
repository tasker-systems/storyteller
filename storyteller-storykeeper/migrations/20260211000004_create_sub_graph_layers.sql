-- =============================================================================
-- TAS-242: Sub-Graph Layers Table
-- =============================================================================
-- Narrative sub-graph layers for tales-within-tales. Layers can nest.
-- entry_scene_id FK is deferred to the scenes migration.
-- =============================================================================

CREATE TABLE sub_graph_layers (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id        UUID NOT NULL REFERENCES stories(id),
    parent_layer_id UUID REFERENCES sub_graph_layers(id),
    name            TEXT NOT NULL,
    layer_type      layer_type NOT NULL,
    entry_scene_id  UUID,
    permeability    REAL NOT NULL DEFAULT 0.0,
    metadata        JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sub_graph_layers_story ON sub_graph_layers(story_id);
CREATE INDEX idx_sub_graph_layers_parent ON sub_graph_layers(parent_layer_id);

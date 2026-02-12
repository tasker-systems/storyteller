-- =============================================================================
-- TAS-242: Scenes Table + Deferred FK on Sub-Graph Layers
-- =============================================================================
-- Scene templates: bounded creative constraints with cast, setting, stakes.
-- Specific playthroughs are tracked in scene_instances.
-- =============================================================================

CREATE TABLE scenes (
    id             UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id       UUID NOT NULL REFERENCES stories(id),
    setting_id     UUID REFERENCES settings(id),
    layer_id       UUID REFERENCES sub_graph_layers(id),
    title          TEXT NOT NULL,
    scene_type     scene_type NOT NULL,
    narrative_mass JSONB NOT NULL,
    scene_data     JSONB NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Add deferred FK for sub_graph_layers.entry_scene_id
ALTER TABLE sub_graph_layers
    ADD CONSTRAINT fk_entry_scene FOREIGN KEY (entry_scene_id) REFERENCES scenes(id);

CREATE INDEX idx_scenes_story ON scenes(story_id);
CREATE INDEX idx_scenes_setting ON scenes(setting_id);
CREATE INDEX idx_scenes_layer ON scenes(layer_id);

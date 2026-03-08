-- =============================================================================
-- TAS-242: Entities Table
-- =============================================================================
-- Entity lifecycle tracking. Created before scene_instances (which FKs here)
-- and before turns (deferred FKs for first/last_seen_turn_id).
-- =============================================================================

CREATE TABLE entities (
    id                 UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id           UUID NOT NULL REFERENCES stories(id),
    name               TEXT NOT NULL,
    entity_origin      entity_origin NOT NULL,
    persistence_mode   persistence_mode NOT NULL,
    promotion_tier     promotion_tier NOT NULL DEFAULT 'unmentioned',
    relational_weight  REAL NOT NULL DEFAULT 0.0,
    event_count        INT NOT NULL DEFAULT 0,
    first_seen_turn_id UUID,
    last_seen_turn_id  UUID,
    layer_id           UUID REFERENCES sub_graph_layers(id),
    metadata           JSONB,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_entities_story ON entities(story_id);
CREATE INDEX idx_entities_promotion ON entities(story_id, promotion_tier);
CREATE INDEX idx_entities_layer ON entities(layer_id);
CREATE INDEX idx_entities_name ON entities(story_id, name);
CREATE INDEX idx_entities_weight ON entities(story_id, relational_weight DESC);

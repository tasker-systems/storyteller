-- =============================================================================
-- TAS-242: Settings Table
-- =============================================================================
-- Spatial locations that scenes take place in. Reusable across scenes.
-- Setting topology graph (AGE) references these IDs as vertices.
-- =============================================================================

CREATE TABLE settings (
    id           UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id     UUID NOT NULL REFERENCES stories(id),
    name         TEXT NOT NULL,
    description  TEXT NOT NULL,
    spatial_data JSONB,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_settings_story ON settings(story_id);

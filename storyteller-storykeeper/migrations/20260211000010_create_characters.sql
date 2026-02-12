-- =============================================================================
-- TAS-242: Characters Table
-- =============================================================================
-- Versioned character sheets. Each row is a snapshot — tensor evolves across
-- scenes. entity_id is not a FK because characters may be authored before
-- the entity lifecycle tracking creates the entity row.
-- =============================================================================

CREATE TABLE characters (
    id        UUID PRIMARY KEY DEFAULT uuidv7(),
    entity_id UUID NOT NULL,
    story_id  UUID NOT NULL REFERENCES stories(id),
    name      TEXT NOT NULL,
    version   INT NOT NULL,
    sheet     JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (entity_id, story_id, version)
);

CREATE INDEX idx_characters_entity ON characters(entity_id);
CREATE INDEX idx_characters_story ON characters(story_id);
CREATE INDEX idx_characters_entity_version ON characters(entity_id, version DESC);

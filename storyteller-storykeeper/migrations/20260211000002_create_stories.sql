-- =============================================================================
-- TAS-242: Stories Table
-- =============================================================================
-- Top-level container. All other tables are scoped to a story.
-- =============================================================================

CREATE TABLE stories (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    title       TEXT NOT NULL,
    description TEXT,
    metadata    JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

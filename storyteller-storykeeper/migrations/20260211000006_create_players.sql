-- =============================================================================
-- TAS-242: Players Table
-- =============================================================================
-- Player identity. Minimal for now.
-- =============================================================================

CREATE TABLE players (
    id           UUID PRIMARY KEY DEFAULT uuidv7(),
    display_name TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

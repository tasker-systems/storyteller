-- =============================================================================
-- Ground-State Reference Data Schema
-- =============================================================================
-- Managed by sqlx, populated by narrative-data Python tooling.
--
-- NOTE: ground_state.settings (genre-level setting archetypes from Tier B)
-- is distinct from public.settings (per-story authored locations). The schema
-- separation enforces that boundary — ground-state rows are read-only reference
-- data; public-schema rows are per-story runtime data.
--
-- All tables use UUID primary keys (gen_random_uuid()) and JSONB payload
-- columns for the full structured elicitation record. Promoted core columns
-- are searchable without JSONB operators; payload carries everything else.
-- =============================================================================

CREATE SCHEMA IF NOT EXISTS ground_state;

-- ---------------------------------------------------------------------------
-- genres
-- ---------------------------------------------------------------------------
-- One row per genre region (e.g. "folk-horror", "cozy-fantasy"). The slug
-- matches the directory names used throughout the narrative-data pipeline.
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.genres (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT        NOT NULL UNIQUE,
    name          TEXT        NOT NULL,
    description   TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- genre_clusters
-- ---------------------------------------------------------------------------
-- Cluster groupings (e.g. "horror", "fantasy", "sci-fi"). Genres belong to
-- one or more clusters via genre_cluster_members.
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.genre_clusters (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT        NOT NULL UNIQUE,
    name          TEXT        NOT NULL,
    description   TEXT,
    payload       JSONB,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- genre_cluster_members
-- ---------------------------------------------------------------------------
-- Many-to-many join between genres and clusters.
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.genre_cluster_members (
    genre_id      UUID NOT NULL REFERENCES ground_state.genres(id)         ON DELETE CASCADE,
    cluster_id    UUID NOT NULL REFERENCES ground_state.genre_clusters(id) ON DELETE CASCADE,
    PRIMARY KEY (genre_id, cluster_id)
);

-- ---------------------------------------------------------------------------
-- state_variables
-- ---------------------------------------------------------------------------
-- Canonical registry of the 12 state variables (e.g. "social_standing",
-- "bodily_integrity"). Each has a default_range for validation.
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.state_variables (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT        NOT NULL UNIQUE,
    name          TEXT        NOT NULL,
    description   TEXT,
    default_range JSONB,
    payload       JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- dimensions
-- ---------------------------------------------------------------------------
-- Canonical registry of the 34 narrative dimensions, grouped by
-- dimension_group (e.g. "personality", "relational", "contextual").
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.dimensions (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    slug             TEXT        NOT NULL UNIQUE,
    name             TEXT        NOT NULL,
    dimension_group  TEXT        NOT NULL,
    description      TEXT,
    payload          JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dimensions_group ON ground_state.dimensions (dimension_group);

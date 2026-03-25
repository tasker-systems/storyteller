-- =============================================================================
-- Bedrock Primitive Type Tables
-- =============================================================================
-- One table per primitive type produced by the narrative-data Tier B pipeline.
-- All tables share the same structural pattern:
--   - UUID primary key
--   - genre_id FK (required) and cluster_id FK (optional, for cross-genre rows)
--   - entity_slug: within-genre unique identifier for the primitive
--   - name: human-readable label
--   - Type-specific promoted core columns (searchable without JSONB operators)
--   - payload JSONB: complete structured elicitation record
--   - source_hash: content hash for idempotent upsert
--   - created_at / updated_at
--
-- Natural key uniqueness: (genre_id, entity_slug) with COALESCE on cluster_id
-- to handle NULLs correctly in the unique index.
--
-- Exception: genre_dimensions is one row per genre (no entity_slug,
-- no cluster_id) — it stores the full dimensional analysis for each genre.
-- =============================================================================

-- ---------------------------------------------------------------------------
-- archetypes
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.archetypes (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id          UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id        UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug       TEXT        NOT NULL,
    name              TEXT        NOT NULL,
    archetype_family  TEXT,
    primary_scale     TEXT,
    payload           JSONB       NOT NULL,
    source_hash       TEXT        NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_archetypes_natural_key
    ON bedrock.archetypes (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_archetypes_genre   ON bedrock.archetypes (genre_id);
CREATE INDEX idx_archetypes_cluster ON bedrock.archetypes (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_archetypes_payload ON bedrock.archetypes USING gin (payload);

-- ---------------------------------------------------------------------------
-- settings
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.settings (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id    UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    setting_type  TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_settings_natural_key
    ON bedrock.settings (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_settings_genre   ON bedrock.settings (genre_id);
CREATE INDEX idx_settings_cluster ON bedrock.settings (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_settings_payload ON bedrock.settings USING gin (payload);

-- ---------------------------------------------------------------------------
-- dynamics
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.dynamics (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id    UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    edge_type     TEXT,
    scale         TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_dynamics_natural_key
    ON bedrock.dynamics (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_dynamics_genre   ON bedrock.dynamics (genre_id);
CREATE INDEX idx_dynamics_cluster ON bedrock.dynamics (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_dynamics_payload ON bedrock.dynamics USING gin (payload);

-- ---------------------------------------------------------------------------
-- goals
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.goals (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id    UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    goal_scale    TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_goals_natural_key
    ON bedrock.goals (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_goals_genre   ON bedrock.goals (genre_id);
CREATE INDEX idx_goals_cluster ON bedrock.goals (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_goals_payload ON bedrock.goals USING gin (payload);

-- ---------------------------------------------------------------------------
-- profiles
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.profiles (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id       UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id     UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug    TEXT        NOT NULL,
    name           TEXT        NOT NULL,
    payload        JSONB       NOT NULL,
    source_hash    TEXT        NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_profiles_natural_key
    ON bedrock.profiles (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_profiles_genre   ON bedrock.profiles (genre_id);
CREATE INDEX idx_profiles_cluster ON bedrock.profiles (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_profiles_payload ON bedrock.profiles USING gin (payload);

-- ---------------------------------------------------------------------------
-- tropes
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.tropes (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id    UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    trope_family_id UUID        REFERENCES bedrock.trope_families(id),
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_tropes_natural_key
    ON bedrock.tropes (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_tropes_genre   ON bedrock.tropes (genre_id);
CREATE INDEX idx_tropes_cluster ON bedrock.tropes (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_tropes_payload ON bedrock.tropes USING gin (payload);

-- ---------------------------------------------------------------------------
-- narrative_shapes
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.narrative_shapes (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id    UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    shape_type    TEXT,
    beat_count    INTEGER,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_narrative_shapes_natural_key
    ON bedrock.narrative_shapes (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_narrative_shapes_genre   ON bedrock.narrative_shapes (genre_id);
CREATE INDEX idx_narrative_shapes_cluster ON bedrock.narrative_shapes (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_narrative_shapes_payload ON bedrock.narrative_shapes USING gin (payload);

-- ---------------------------------------------------------------------------
-- ontological_posture
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.ontological_posture (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id             UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id           UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug          TEXT        NOT NULL,
    name                 TEXT        NOT NULL,
    boundary_stability   TEXT,
    payload              JSONB       NOT NULL,
    source_hash          TEXT        NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_ontological_posture_natural_key
    ON bedrock.ontological_posture (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_ontological_posture_genre   ON bedrock.ontological_posture (genre_id);
CREATE INDEX idx_ontological_posture_cluster ON bedrock.ontological_posture (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_ontological_posture_payload ON bedrock.ontological_posture USING gin (payload);

-- ---------------------------------------------------------------------------
-- spatial_topology
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.spatial_topology (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id             UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id           UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug          TEXT        NOT NULL,
    name                 TEXT        NOT NULL,
    friction_type        TEXT,
    directionality_type  TEXT,
    payload              JSONB       NOT NULL,
    source_hash          TEXT        NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_spatial_topology_natural_key
    ON bedrock.spatial_topology (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_spatial_topology_genre   ON bedrock.spatial_topology (genre_id);
CREATE INDEX idx_spatial_topology_cluster ON bedrock.spatial_topology (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_spatial_topology_payload ON bedrock.spatial_topology USING gin (payload);

-- ---------------------------------------------------------------------------
-- place_entities
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.place_entities (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id    UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    topological_role VARCHAR,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_place_entities_natural_key
    ON bedrock.place_entities (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_place_entities_genre   ON bedrock.place_entities (genre_id);
CREATE INDEX idx_place_entities_cluster ON bedrock.place_entities (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_place_entities_payload ON bedrock.place_entities USING gin (payload);

-- ---------------------------------------------------------------------------
-- archetype_dynamics
-- ---------------------------------------------------------------------------
-- Cross-archetype relational dynamics. archetype_a and archetype_b are slugs
-- referencing archetypes within the same genre context.
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.archetype_dynamics (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES bedrock.genres(id),
    cluster_id    UUID        REFERENCES bedrock.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    archetype_a   TEXT,
    archetype_b   TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_archetype_dynamics_natural_key
    ON bedrock.archetype_dynamics (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_archetype_dynamics_genre   ON bedrock.archetype_dynamics (genre_id);
CREATE INDEX idx_archetype_dynamics_cluster ON bedrock.archetype_dynamics (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_archetype_dynamics_payload ON bedrock.archetype_dynamics USING gin (payload);

-- ---------------------------------------------------------------------------
-- genre_dimensions
-- ---------------------------------------------------------------------------
-- SPECIAL: one row per genre, no entity_slug, no cluster_id.
-- Stores the complete dimensional analysis for a genre (all 34 dimensions).
-- ---------------------------------------------------------------------------
CREATE TABLE bedrock.genre_dimensions (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL UNIQUE REFERENCES bedrock.genres(id),
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_genre_dimensions_payload ON bedrock.genre_dimensions USING gin (payload);

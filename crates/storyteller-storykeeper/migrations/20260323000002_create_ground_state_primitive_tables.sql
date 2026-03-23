-- =============================================================================
-- Ground-State Primitive Type Tables
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
CREATE TABLE ground_state.archetypes (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id          UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id        UUID        REFERENCES ground_state.genre_clusters(id),
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
    ON ground_state.archetypes (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_archetypes_genre   ON ground_state.archetypes (genre_id);
CREATE INDEX idx_archetypes_cluster ON ground_state.archetypes (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_archetypes_payload ON ground_state.archetypes USING gin (payload);

-- ---------------------------------------------------------------------------
-- settings
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.settings (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id    UUID        REFERENCES ground_state.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    setting_type  TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_settings_natural_key
    ON ground_state.settings (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_settings_genre   ON ground_state.settings (genre_id);
CREATE INDEX idx_settings_cluster ON ground_state.settings (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_settings_payload ON ground_state.settings USING gin (payload);

-- ---------------------------------------------------------------------------
-- dynamics
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.dynamics (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id    UUID        REFERENCES ground_state.genre_clusters(id),
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
    ON ground_state.dynamics (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_dynamics_genre   ON ground_state.dynamics (genre_id);
CREATE INDEX idx_dynamics_cluster ON ground_state.dynamics (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_dynamics_payload ON ground_state.dynamics USING gin (payload);

-- ---------------------------------------------------------------------------
-- goals
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.goals (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id    UUID        REFERENCES ground_state.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    goal_scale    TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_goals_natural_key
    ON ground_state.goals (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_goals_genre   ON ground_state.goals (genre_id);
CREATE INDEX idx_goals_cluster ON ground_state.goals (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_goals_payload ON ground_state.goals USING gin (payload);

-- ---------------------------------------------------------------------------
-- profiles
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.profiles (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id       UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id     UUID        REFERENCES ground_state.genre_clusters(id),
    entity_slug    TEXT        NOT NULL,
    name           TEXT        NOT NULL,
    archetype_ref  TEXT,
    payload        JSONB       NOT NULL,
    source_hash    TEXT        NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_profiles_natural_key
    ON ground_state.profiles (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_profiles_genre   ON ground_state.profiles (genre_id);
CREATE INDEX idx_profiles_cluster ON ground_state.profiles (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_profiles_payload ON ground_state.profiles USING gin (payload);

-- ---------------------------------------------------------------------------
-- tropes
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.tropes (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id    UUID        REFERENCES ground_state.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    trope_family  TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_tropes_natural_key
    ON ground_state.tropes (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_tropes_genre   ON ground_state.tropes (genre_id);
CREATE INDEX idx_tropes_cluster ON ground_state.tropes (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_tropes_payload ON ground_state.tropes USING gin (payload);

-- ---------------------------------------------------------------------------
-- narrative_shapes
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.narrative_shapes (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id    UUID        REFERENCES ground_state.genre_clusters(id),
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
    ON ground_state.narrative_shapes (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_narrative_shapes_genre   ON ground_state.narrative_shapes (genre_id);
CREATE INDEX idx_narrative_shapes_cluster ON ground_state.narrative_shapes (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_narrative_shapes_payload ON ground_state.narrative_shapes USING gin (payload);

-- ---------------------------------------------------------------------------
-- ontological_posture
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.ontological_posture (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id             UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id           UUID        REFERENCES ground_state.genre_clusters(id),
    entity_slug          TEXT        NOT NULL,
    name                 TEXT        NOT NULL,
    boundary_stability   TEXT,
    payload              JSONB       NOT NULL,
    source_hash          TEXT        NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_ontological_posture_natural_key
    ON ground_state.ontological_posture (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_ontological_posture_genre   ON ground_state.ontological_posture (genre_id);
CREATE INDEX idx_ontological_posture_cluster ON ground_state.ontological_posture (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_ontological_posture_payload ON ground_state.ontological_posture USING gin (payload);

-- ---------------------------------------------------------------------------
-- spatial_topology
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.spatial_topology (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id             UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id           UUID        REFERENCES ground_state.genre_clusters(id),
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
    ON ground_state.spatial_topology (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_spatial_topology_genre   ON ground_state.spatial_topology (genre_id);
CREATE INDEX idx_spatial_topology_cluster ON ground_state.spatial_topology (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_spatial_topology_payload ON ground_state.spatial_topology USING gin (payload);

-- ---------------------------------------------------------------------------
-- place_entities
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.place_entities (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id    UUID        REFERENCES ground_state.genre_clusters(id),
    entity_slug   TEXT        NOT NULL,
    name          TEXT        NOT NULL,
    place_type    TEXT,
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_place_entities_natural_key
    ON ground_state.place_entities (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_place_entities_genre   ON ground_state.place_entities (genre_id);
CREATE INDEX idx_place_entities_cluster ON ground_state.place_entities (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_place_entities_payload ON ground_state.place_entities USING gin (payload);

-- ---------------------------------------------------------------------------
-- archetype_dynamics
-- ---------------------------------------------------------------------------
-- Cross-archetype relational dynamics. archetype_a and archetype_b are slugs
-- referencing archetypes within the same genre context.
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.archetype_dynamics (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL REFERENCES ground_state.genres(id),
    cluster_id    UUID        REFERENCES ground_state.genre_clusters(id),
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
    ON ground_state.archetype_dynamics (genre_id, entity_slug, COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID));
CREATE INDEX idx_archetype_dynamics_genre   ON ground_state.archetype_dynamics (genre_id);
CREATE INDEX idx_archetype_dynamics_cluster ON ground_state.archetype_dynamics (cluster_id) WHERE cluster_id IS NOT NULL;
CREATE INDEX idx_archetype_dynamics_payload ON ground_state.archetype_dynamics USING gin (payload);

-- ---------------------------------------------------------------------------
-- genre_dimensions
-- ---------------------------------------------------------------------------
-- SPECIAL: one row per genre, no entity_slug, no cluster_id.
-- Stores the complete dimensional analysis for a genre (all 34 dimensions).
-- ---------------------------------------------------------------------------
CREATE TABLE ground_state.genre_dimensions (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    genre_id      UUID        NOT NULL UNIQUE REFERENCES ground_state.genres(id),
    payload       JSONB       NOT NULL,
    source_hash   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_genre_dimensions_payload ON ground_state.genre_dimensions USING gin (payload);

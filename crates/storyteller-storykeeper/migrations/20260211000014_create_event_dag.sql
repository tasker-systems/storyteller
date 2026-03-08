-- =============================================================================
-- TAS-243: Event Dependency DAG Tables
-- =============================================================================
-- The Event DAG lives in PostgreSQL relational tables, NOT in Apache AGE.
-- Structure is static (authored); only resolution state changes at runtime.
-- Key algorithms (Kahn's topological sort, exclusion cascade, evaluable
-- frontier, amplification compounding) run in petgraph, not Cypher.
--
-- See docs/technical/age-persistence/event-dag-age.md for full rationale.
-- SQL functions adapted from tasker-core's proven DAG patterns.
-- =============================================================================

-- DAG nodes: narrative conditions that can be resolved during play
CREATE TABLE event_conditions (
    id                   UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id             UUID NOT NULL REFERENCES stories(id),
    name                 TEXT NOT NULL,
    description          TEXT,
    condition_type       condition_type NOT NULL,
    -- What must be true for this condition to resolve
    resolution_predicate JSONB NOT NULL,
    -- Scenes where resolution can occur
    resolving_scene_ids  UUID[] DEFAULT '{}',
    -- Narrative weight for prioritization
    narrative_weight     REAL NOT NULL DEFAULT 1.0,
    -- Amplification base value (before compounding along amplifies edges)
    base_amplification   REAL NOT NULL DEFAULT 1.0,
    -- Sub-graph layer scoping
    layer_id             UUID REFERENCES sub_graph_layers(id),
    metadata             JSONB,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, name)
);

CREATE INDEX idx_event_conditions_story ON event_conditions(story_id);
CREATE INDEX idx_event_conditions_type ON event_conditions(story_id, condition_type);
CREATE INDEX idx_event_conditions_layer ON event_conditions(layer_id);
CREATE INDEX idx_event_conditions_resolving_scenes
    ON event_conditions USING GIN (resolving_scene_ids);

-- DAG edges: typed relationships between conditions
CREATE TABLE event_dependencies (
    id                UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id          UUID NOT NULL REFERENCES stories(id),
    from_condition_id UUID NOT NULL REFERENCES event_conditions(id),
    to_condition_id   UUID NOT NULL REFERENCES event_conditions(id),
    dependency_type   dependency_type NOT NULL,
    -- Amplification weight (only meaningful for 'amplifies' type)
    amplification_weight REAL DEFAULT 1.0,
    description       TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, from_condition_id, to_condition_id, dependency_type)
);

CREATE INDEX idx_event_deps_story ON event_dependencies(story_id);
CREATE INDEX idx_event_deps_from ON event_dependencies(from_condition_id);
CREATE INDEX idx_event_deps_to ON event_dependencies(to_condition_id);
CREATE INDEX idx_event_deps_type ON event_dependencies(dependency_type);

-- Session-scoped resolution state tracking
CREATE TABLE event_condition_states (
    id                    UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id            UUID NOT NULL REFERENCES sessions(id),
    condition_id          UUID NOT NULL REFERENCES event_conditions(id),
    resolution_state      resolution_state NOT NULL DEFAULT 'unresolved',
    -- Computed consequence magnitude (after amplification compounding)
    consequence_magnitude REAL NOT NULL DEFAULT 1.0,
    -- When and where resolved
    resolved_at_turn      INT,
    resolved_at           TIMESTAMPTZ,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, condition_id)
);

CREATE INDEX idx_event_states_session ON event_condition_states(session_id);
CREATE INDEX idx_event_states_state ON event_condition_states(session_id, resolution_state);

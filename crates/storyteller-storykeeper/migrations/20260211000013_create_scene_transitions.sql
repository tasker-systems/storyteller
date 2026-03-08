-- =============================================================================
-- TAS-243: Scene Transitions + Scene Activation States
-- =============================================================================
-- scene_transitions: Authored metadata for each possible transition between
-- scenes. Rich departure conditions, approach effects, and momentum transfer
-- live here. The AGE :TRANSITIONS_TO edge carries only a transition_weight
-- float for Cypher-level ranking — this table holds the full picture.
--
-- scene_activation_states: Session-scoped tracking of how each scene relates
-- to a specific player's session. Cannot live on AGE vertices (which are
-- story-scoped). Tracks activation lifecycle, dynamic mass adjustment, and
-- visit history for diminishing returns on connective space.
-- =============================================================================

CREATE TABLE scene_transitions (
    id                   UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id             UUID NOT NULL REFERENCES stories(id),
    from_scene_id        UUID NOT NULL REFERENCES scenes(id),
    to_scene_id          UUID NOT NULL REFERENCES scenes(id),
    outcome_label        TEXT NOT NULL,
    -- Departure conditions and effects
    departure_conditions JSONB,
    approach_effects     JSONB,
    momentum_transfer    JSONB,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (story_id, from_scene_id, to_scene_id, outcome_label)
);

CREATE INDEX idx_scene_transitions_story ON scene_transitions(story_id);
CREATE INDEX idx_scene_transitions_from ON scene_transitions(from_scene_id);

CREATE TABLE scene_activation_states (
    id                 UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id         UUID NOT NULL REFERENCES sessions(id),
    scene_id           UUID NOT NULL REFERENCES scenes(id),
    state              scene_activation NOT NULL DEFAULT 'dormant',
    -- Dynamic mass adjustment (session-specific, from approach satisfaction)
    dynamic_adjustment REAL NOT NULL DEFAULT 0.0,
    -- Visit tracking for connective space diminishing returns
    visit_count        INT NOT NULL DEFAULT 0,
    last_visited_turn  INT,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, scene_id)
);

CREATE INDEX idx_scene_activation_session ON scene_activation_states(session_id);
CREATE INDEX idx_scene_activation_state ON scene_activation_states(session_id, state);

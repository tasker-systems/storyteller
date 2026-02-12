-- =============================================================================
-- TAS-242: Turns Table + Deferred FKs on Entities
-- =============================================================================
-- Turn records — the atomic unit of play. Belongs to a scene instance.
-- Also adds deferred FKs from entities.first/last_seen_turn_id.
-- =============================================================================

CREATE TABLE turns (
    id                       UUID PRIMARY KEY DEFAULT uuidv7(),
    session_id               UUID NOT NULL REFERENCES sessions(id),
    scene_instance_id        UUID NOT NULL REFERENCES scene_instances(id),
    turn_number              INT NOT NULL,
    player_input             TEXT NOT NULL,
    narrator_rendering       TEXT,
    classification           JSONB,
    committed_classification JSONB,
    predictions              JSONB,
    resolver_output          JSONB,
    provisional_status       provisional_status NOT NULL DEFAULT 'hypothesized',
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    rendered_at              TIMESTAMPTZ,
    committed_at             TIMESTAMPTZ
);

-- Add deferred FKs on entities for turn references
ALTER TABLE entities
    ADD CONSTRAINT fk_first_seen_turn
    FOREIGN KEY (first_seen_turn_id) REFERENCES turns(id);

ALTER TABLE entities
    ADD CONSTRAINT fk_last_seen_turn
    FOREIGN KEY (last_seen_turn_id) REFERENCES turns(id);

CREATE INDEX idx_turns_session ON turns(session_id);
CREATE INDEX idx_turns_scene_instance ON turns(scene_instance_id);
CREATE INDEX idx_turns_instance_order ON turns(scene_instance_id, turn_number);
CREATE UNIQUE INDEX idx_turns_session_number ON turns(session_id, turn_number);

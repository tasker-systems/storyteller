-- =============================================================================
-- TAS-242: Sessions Table
-- =============================================================================
-- Session lifecycle. current_scene_instance_id FK is deferred to the
-- scene_instances migration.
-- =============================================================================

CREATE TABLE sessions (
    id                        UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id                  UUID NOT NULL REFERENCES stories(id),
    player_id                 UUID NOT NULL REFERENCES players(id),
    current_scene_instance_id UUID,
    status                    session_status NOT NULL DEFAULT 'created',
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at                  TIMESTAMPTZ
);

CREATE INDEX idx_sessions_story ON sessions(story_id);
CREATE INDEX idx_sessions_player ON sessions(player_id);
CREATE INDEX idx_sessions_status ON sessions(status) WHERE status != 'ended';

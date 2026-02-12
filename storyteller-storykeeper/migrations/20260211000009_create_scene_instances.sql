-- =============================================================================
-- TAS-242: Scene Instances Table + Deferred FK on Sessions
-- =============================================================================
-- A specific playthrough of a scene within a session. Tracks POV character,
-- re-entry numbering, and entry conditions.
-- =============================================================================

CREATE TABLE scene_instances (
    id               UUID PRIMARY KEY DEFAULT uuidv7(),
    scene_id         UUID NOT NULL REFERENCES scenes(id),
    session_id       UUID NOT NULL REFERENCES sessions(id),
    player_entity_id UUID NOT NULL REFERENCES entities(id),
    instance_number  INT NOT NULL,
    status           scene_instance_status NOT NULL DEFAULT 'active',
    entry_conditions JSONB,
    entered_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    exited_at        TIMESTAMPTZ,
    UNIQUE (session_id, scene_id, instance_number)
);

-- Add deferred FK for sessions.current_scene_instance_id
ALTER TABLE sessions
    ADD CONSTRAINT fk_current_scene_instance
    FOREIGN KEY (current_scene_instance_id) REFERENCES scene_instances(id);

CREATE INDEX idx_scene_instances_session ON scene_instances(session_id);
CREATE INDEX idx_scene_instances_scene ON scene_instances(scene_id);
CREATE INDEX idx_scene_instances_active ON scene_instances(session_id, status)
    WHERE status = 'active';

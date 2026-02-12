-- =============================================================================
-- TAS-242: Event Ledger Table
-- =============================================================================
-- The system's memory. Append-only record of every committed event.
-- Source of truth for entity weight computation, relational cascade,
-- composition detection, and cross-scene event queries.
--
-- story_id, session_id, and scene_id are denormalized for query efficiency.
-- event_kind is NOT NULL with 'unknown' default — events always have a kind.
-- =============================================================================

CREATE TABLE event_ledger (
    id                      UUID PRIMARY KEY DEFAULT uuidv7(),
    story_id                UUID NOT NULL REFERENCES stories(id),
    session_id              UUID NOT NULL REFERENCES sessions(id),
    scene_id                UUID NOT NULL REFERENCES scenes(id),
    scene_instance_id       UUID REFERENCES scene_instances(id),
    turn_id                 UUID REFERENCES turns(id),
    layer_id                UUID REFERENCES sub_graph_layers(id),
    event_type              event_type NOT NULL,
    event_kind              event_kind NOT NULL DEFAULT 'unknown',
    priority                event_priority NOT NULL,
    participants            JSONB NOT NULL DEFAULT '[]',
    relational_implications JSONB NOT NULL DEFAULT '[]',
    source                  JSONB NOT NULL,
    confidence              JSONB NOT NULL,
    payload                 JSONB NOT NULL,
    committed_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Single-column indexes for simple lookups
CREATE INDEX idx_event_ledger_story ON event_ledger(story_id);
CREATE INDEX idx_event_ledger_turn ON event_ledger(turn_id);

-- Composite indexes for Storykeeper access patterns
CREATE INDEX idx_event_ledger_scene_time ON event_ledger(scene_id, committed_at);
CREATE INDEX idx_event_ledger_instance_time ON event_ledger(scene_instance_id, committed_at);
CREATE INDEX idx_event_ledger_kind_time ON event_ledger(event_kind, committed_at)
    WHERE event_type = 'atom';

-- GIN index for entity participation queries
CREATE INDEX idx_event_ledger_participants ON event_ledger
    USING GIN (participants jsonb_path_ops);

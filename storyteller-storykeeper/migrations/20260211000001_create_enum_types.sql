-- =============================================================================
-- TAS-242: PostgreSQL ENUM Types for Storyteller Domain Model
-- =============================================================================
-- All enum types created before any tables. Variants are declared in semantic
-- order — for ordinal types, PostgreSQL sorts by declaration order.
-- =============================================================================

-- Ordinal: sorts immediate → deferred
CREATE TYPE event_priority AS ENUM (
    'immediate', 'high', 'normal', 'low', 'deferred'
);

-- Discriminator: atom vs compound event
CREATE TYPE event_type AS ENUM (
    'atom', 'compound'
);

-- Discriminator: what kind of atomic event
CREATE TYPE event_kind AS ENUM (
    'state_assertion', 'action_occurrence', 'spatial_change',
    'relational_shift', 'information_transfer', 'unknown'
);

-- Ordinal state machine: hypothesized → rendered → committed
CREATE TYPE provisional_status AS ENUM (
    'hypothesized', 'rendered', 'committed'
);

-- Discriminator: scene gravitational classification
CREATE TYPE scene_type AS ENUM (
    'gravitational', 'connective', 'gate', 'threshold'
);

-- Discriminator: how an entity entered the narrative
CREATE TYPE entity_origin AS ENUM (
    'authored', 'promoted', 'generated'
);

-- Discriminator: entity lifecycle scope
CREATE TYPE persistence_mode AS ENUM (
    'permanent', 'scene_local', 'ephemeral'
);

-- Ordinal: promotion through lifecycle tiers
CREATE TYPE promotion_tier AS ENUM (
    'unmentioned', 'mentioned', 'referenced', 'tracked', 'persistent'
);

-- Ordinal state machine: session lifecycle
CREATE TYPE session_status AS ENUM (
    'created', 'active', 'suspended', 'ended'
);

-- State machine: scene instance lifecycle
CREATE TYPE scene_instance_status AS ENUM (
    'active', 'completed', 'abandoned'
);

-- Discriminator: sub-graph narrative layer type
CREATE TYPE layer_type AS ENUM (
    'memory', 'dream', 'fairy_tale', 'parallel_pov', 'embedded_text', 'epistle'
);

-- Discriminator: scene origin tracking (authored by designer, collaborative, or runtime-generated)
CREATE TYPE scene_provenance AS ENUM (
    'authored', 'collaborative', 'generated'
);

-- State machine: session-scoped scene activation lifecycle
CREATE TYPE scene_activation AS ENUM (
    'dormant', 'approaching', 'active', 'completed', 'bypassed'
);

-- Discriminator: event DAG condition classification
CREATE TYPE condition_type AS ENUM (
    'prerequisite', 'discovery', 'gate', 'emergent', 'exclusion'
);

-- Discriminator: event DAG dependency edge type
CREATE TYPE dependency_type AS ENUM (
    'requires', 'excludes', 'enables', 'amplifies'
);

-- State machine: event condition resolution lifecycle
CREATE TYPE resolution_state AS ENUM (
    'unresolved', 'resolved', 'excluded', 'unreachable'
);

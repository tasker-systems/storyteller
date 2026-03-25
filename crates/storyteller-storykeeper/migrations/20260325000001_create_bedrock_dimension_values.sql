-- SPDX-License-Identifier: AGPL-3.0-only
-- Copyright (c) 2026 Tasker Systems. All rights reserved.
-- See LICENSING.md for details.

-- =============================================================================
-- Dimensional Value Extraction
-- =============================================================================
-- Promotes structured dimensional data from primitive entity payloads into
-- queryable, composable form. One row per entity-dimension pair.
--
-- Value types:
--   normalized    — [0.0, 1.0] continuous (warmth, authority, knowability)
--   bipolar       — [-1.0, 1.0] continuous (traversal_cost deltas)
--   categorical   — enum text (tension_signature, enclosure, edge_type)
--   weighted_tags — dict<str, float> JSONB (power_treatment, identity_treatment)
--   set           — list<str> JSONB (currencies, magic types, locus_of_power)
--
-- Populated by the Python narrative-data loader during phase 2.
-- =============================================================================

CREATE TABLE bedrock.dimension_values (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Entity reference (polymorphic — no FK on primitive_id)
    primitive_table   VARCHAR     NOT NULL,
    primitive_id      UUID        NOT NULL,
    genre_id          UUID        NOT NULL REFERENCES bedrock.genres(id),
    -- Dimension identity
    dimension_slug    TEXT        NOT NULL,
    dimension_group   TEXT        NOT NULL,
    value_type        VARCHAR     NOT NULL,
    -- Typed value columns (one populated per row)
    numeric_value     REAL,
    categorical_value TEXT,
    complex_value     JSONB,
    -- Provenance
    source_path       TEXT,
    tier              VARCHAR     NOT NULL DEFAULT 'core',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Primary query paths
CREATE INDEX idx_dv_entity ON bedrock.dimension_values(primitive_table, primitive_id);
CREATE INDEX idx_dv_dimension ON bedrock.dimension_values(dimension_slug);
CREATE INDEX idx_dv_genre_dimension ON bedrock.dimension_values(genre_id, dimension_slug);
CREATE INDEX idx_dv_value_type ON bedrock.dimension_values(value_type);
CREATE INDEX idx_dv_complex ON bedrock.dimension_values USING gin (complex_value) WHERE complex_value IS NOT NULL;
CREATE UNIQUE INDEX idx_dv_natural_key ON bedrock.dimension_values(primitive_table, primitive_id, dimension_slug);

-- SPDX-License-Identifier: AGPL-3.0-only
-- Copyright (c) 2026 Tasker Systems. All rights reserved.
-- See LICENSING.md for details.

-- bedrock.genre_context: returns all bedrock data for a genre in one query.
-- Returns NULL when the genre slug is not found.
-- Each primitive type returns full row data (all columns) so downstream Rust
-- code can deserialize into typed Record structs without further unwrapping.

CREATE OR REPLACE FUNCTION bedrock.genre_context(p_genre_slug TEXT)
RETURNS JSONB AS $$
DECLARE
    v_genre_id UUID;
BEGIN
    SELECT id INTO v_genre_id FROM bedrock.genres WHERE slug = p_genre_slug;
    IF v_genre_id IS NULL THEN
        RETURN NULL;
    END IF;

    RETURN jsonb_build_object(
        'genre_slug', p_genre_slug,
        'genre', (
            SELECT jsonb_build_object(
                'id', g.id,
                'slug', g.slug,
                'name', g.name,
                'description', g.description,
                'payload', g.payload,
                'source_hash', g.source_hash,
                'created_at', g.created_at,
                'updated_at', g.updated_at
            )
            FROM bedrock.genres g
            WHERE g.id = v_genre_id
        ),
        'archetypes', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', a.id,
                'genre_id', a.genre_id,
                'cluster_id', a.cluster_id,
                'entity_slug', a.entity_slug,
                'name', a.name,
                'archetype_family', a.archetype_family,
                'primary_scale', a.primary_scale,
                'payload', a.payload,
                'source_hash', a.source_hash,
                'created_at', a.created_at,
                'updated_at', a.updated_at
            ))
            FROM bedrock.archetypes a
            WHERE a.genre_id = v_genre_id AND a.cluster_id IS NULL
        ),
        'dynamics', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', d.id,
                'genre_id', d.genre_id,
                'cluster_id', d.cluster_id,
                'entity_slug', d.entity_slug,
                'name', d.name,
                'edge_type', d.edge_type,
                'scale', d.scale,
                'payload', d.payload,
                'source_hash', d.source_hash,
                'created_at', d.created_at,
                'updated_at', d.updated_at
            ))
            FROM bedrock.dynamics d
            WHERE d.genre_id = v_genre_id AND d.cluster_id IS NULL
        ),
        'settings', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', s.id,
                'genre_id', s.genre_id,
                'cluster_id', s.cluster_id,
                'entity_slug', s.entity_slug,
                'name', s.name,
                'setting_type', s.setting_type,
                'payload', s.payload,
                'source_hash', s.source_hash,
                'created_at', s.created_at,
                'updated_at', s.updated_at
            ))
            FROM bedrock.settings s
            WHERE s.genre_id = v_genre_id AND s.cluster_id IS NULL
        ),
        'goals', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', gl.id,
                'genre_id', gl.genre_id,
                'cluster_id', gl.cluster_id,
                'entity_slug', gl.entity_slug,
                'name', gl.name,
                'goal_scale', gl.goal_scale,
                'payload', gl.payload,
                'source_hash', gl.source_hash,
                'created_at', gl.created_at,
                'updated_at', gl.updated_at
            ))
            FROM bedrock.goals gl
            WHERE gl.genre_id = v_genre_id AND gl.cluster_id IS NULL
        ),
        'profiles', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', pr.id,
                'genre_id', pr.genre_id,
                'cluster_id', pr.cluster_id,
                'entity_slug', pr.entity_slug,
                'name', pr.name,
                'payload', pr.payload,
                'source_hash', pr.source_hash,
                'created_at', pr.created_at,
                'updated_at', pr.updated_at
            ))
            FROM bedrock.profiles pr
            WHERE pr.genre_id = v_genre_id AND pr.cluster_id IS NULL
        ),
        'tropes', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', t.id,
                'genre_id', t.genre_id,
                'cluster_id', t.cluster_id,
                'entity_slug', t.entity_slug,
                'name', t.name,
                'trope_family_id', t.trope_family_id,
                'payload', t.payload,
                'source_hash', t.source_hash,
                'created_at', t.created_at,
                'updated_at', t.updated_at
            ))
            FROM bedrock.tropes t
            WHERE t.genre_id = v_genre_id AND t.cluster_id IS NULL
        ),
        'narrative_shapes', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', ns.id,
                'genre_id', ns.genre_id,
                'cluster_id', ns.cluster_id,
                'entity_slug', ns.entity_slug,
                'name', ns.name,
                'shape_type', ns.shape_type,
                'beat_count', ns.beat_count,
                'payload', ns.payload,
                'source_hash', ns.source_hash,
                'created_at', ns.created_at,
                'updated_at', ns.updated_at
            ))
            FROM bedrock.narrative_shapes ns
            WHERE ns.genre_id = v_genre_id AND ns.cluster_id IS NULL
        ),
        'ontological_posture', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', op.id,
                'genre_id', op.genre_id,
                'cluster_id', op.cluster_id,
                'entity_slug', op.entity_slug,
                'name', op.name,
                'boundary_stability', op.boundary_stability,
                'payload', op.payload,
                'source_hash', op.source_hash,
                'created_at', op.created_at,
                'updated_at', op.updated_at
            ))
            FROM bedrock.ontological_posture op
            WHERE op.genre_id = v_genre_id AND op.cluster_id IS NULL
        ),
        'spatial_topology', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', st.id,
                'genre_id', st.genre_id,
                'cluster_id', st.cluster_id,
                'entity_slug', st.entity_slug,
                'name', st.name,
                'friction_type', st.friction_type,
                'directionality_type', st.directionality_type,
                'payload', st.payload,
                'source_hash', st.source_hash,
                'created_at', st.created_at,
                'updated_at', st.updated_at
            ))
            FROM bedrock.spatial_topology st
            WHERE st.genre_id = v_genre_id AND st.cluster_id IS NULL
        ),
        'place_entities', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', pe.id,
                'genre_id', pe.genre_id,
                'cluster_id', pe.cluster_id,
                'entity_slug', pe.entity_slug,
                'name', pe.name,
                'topological_role', pe.topological_role,
                'payload', pe.payload,
                'source_hash', pe.source_hash,
                'created_at', pe.created_at,
                'updated_at', pe.updated_at
            ))
            FROM bedrock.place_entities pe
            WHERE pe.genre_id = v_genre_id AND pe.cluster_id IS NULL
        ),
        'archetype_dynamics', (
            SELECT jsonb_agg(jsonb_build_object(
                'id', ad.id,
                'genre_id', ad.genre_id,
                'cluster_id', ad.cluster_id,
                'entity_slug', ad.entity_slug,
                'name', ad.name,
                'archetype_a', ad.archetype_a,
                'archetype_b', ad.archetype_b,
                'payload', ad.payload,
                'source_hash', ad.source_hash,
                'created_at', ad.created_at,
                'updated_at', ad.updated_at
            ))
            FROM bedrock.archetype_dynamics ad
            WHERE ad.genre_id = v_genre_id AND ad.cluster_id IS NULL
        ),
        'genre_dimensions', (
            SELECT jsonb_build_object(
                'id', gd.id,
                'genre_id', gd.genre_id,
                'payload', gd.payload,
                'source_hash', gd.source_hash,
                'created_at', gd.created_at,
                'updated_at', gd.updated_at
            )
            FROM bedrock.genre_dimensions gd
            WHERE gd.genre_id = v_genre_id
            LIMIT 1
        )
    );
END;
$$ LANGUAGE plpgsql STABLE;

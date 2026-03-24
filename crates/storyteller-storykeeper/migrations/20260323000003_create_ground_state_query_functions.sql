-- SPDX-License-Identifier: AGPL-3.0-only
-- Copyright (c) 2026 Tasker Systems. All rights reserved.
-- See LICENSING.md for details.

-- ground_state.genre_context: returns all ground-state data for a genre in one query.
-- Returns NULL when the genre slug is not found.

CREATE OR REPLACE FUNCTION ground_state.genre_context(p_genre_slug TEXT)
RETURNS JSONB AS $$
DECLARE
    v_genre_id UUID;
BEGIN
    SELECT id INTO v_genre_id FROM ground_state.genres WHERE slug = p_genre_slug;
    IF v_genre_id IS NULL THEN
        RETURN NULL;
    END IF;

    RETURN jsonb_build_object(
        'genre_slug', p_genre_slug,
        'genre', (SELECT payload FROM ground_state.genres WHERE id = v_genre_id),
        'archetypes', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                       FROM ground_state.archetypes
                       WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'dynamics', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                     FROM ground_state.dynamics
                     WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'settings', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                     FROM ground_state.settings
                     WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'goals', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                  FROM ground_state.goals
                  WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'profiles', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                     FROM ground_state.profiles
                     WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'tropes', (SELECT jsonb_agg(jsonb_build_object(
                        'slug', t.entity_slug,
                        'data', t.payload,
                        'family_slug', tf.slug,
                        'family_name', tf.name))
                   FROM ground_state.tropes t
                   LEFT JOIN ground_state.trope_families tf ON t.trope_family_id = tf.id
                   WHERE t.genre_id = v_genre_id AND t.cluster_id IS NULL),
        'narrative_shapes', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                            FROM ground_state.narrative_shapes
                            WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'ontological_posture', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                               FROM ground_state.ontological_posture
                               WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'spatial_topology', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                            FROM ground_state.spatial_topology
                            WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'place_entities', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                          FROM ground_state.place_entities
                          WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'archetype_dynamics', (SELECT jsonb_agg(jsonb_build_object('slug', entity_slug, 'data', payload))
                              FROM ground_state.archetype_dynamics
                              WHERE genre_id = v_genre_id AND cluster_id IS NULL),
        'genre_dimensions', (SELECT payload FROM ground_state.genre_dimensions
                            WHERE genre_id = v_genre_id LIMIT 1)
    );
END;
$$ LANGUAGE plpgsql STABLE;

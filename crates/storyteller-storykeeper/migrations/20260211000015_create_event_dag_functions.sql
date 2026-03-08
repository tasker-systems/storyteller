-- =============================================================================
-- TAS-243: Event DAG SQL Functions and View
-- =============================================================================
-- Recursive CTE patterns adapted from tasker-core's proven DAG framework.
-- These provide PostgreSQL-level DAG analysis; petgraph handles the
-- algorithms that are more natural in Rust (Kahn's sort, exclusion cascade,
-- amplification compounding).
--
-- See docs/technical/age-persistence/event-dag-age.md for full rationale.
-- =============================================================================

-- ---------------------------------------------------------------------------
-- calculate_event_dependency_levels()
-- ---------------------------------------------------------------------------
-- Walks from root conditions (no incoming 'requires' edges) outward.
-- MAX(level) handles diamond dependencies where a node is reachable via
-- multiple paths of different lengths.
-- Adapted from tasker-core's calculate_dependency_levels().
-- ---------------------------------------------------------------------------
CREATE FUNCTION calculate_event_dependency_levels(input_story_id uuid)
RETURNS TABLE(condition_id uuid, dependency_level integer)
LANGUAGE plpgsql STABLE
AS $$
BEGIN
    RETURN QUERY
    WITH RECURSIVE dependency_levels AS (
        -- Base case: root conditions (no 'requires' dependencies)
        SELECT
            ec.id AS condition_id,
            0 AS level
        FROM event_conditions ec
        WHERE ec.story_id = input_story_id
            AND NOT EXISTS (
                SELECT 1 FROM event_dependencies ed
                WHERE ed.to_condition_id = ec.id
                    AND ed.dependency_type = 'requires'
            )

        UNION ALL

        -- Recursive case: conditions whose 'requires' parents are at current level
        SELECT
            ed.to_condition_id AS condition_id,
            dl.level + 1 AS level
        FROM dependency_levels dl
        JOIN event_dependencies ed ON ed.from_condition_id = dl.condition_id
        JOIN event_conditions ec ON ec.id = ed.to_condition_id
        WHERE ec.story_id = input_story_id
            AND ed.dependency_type = 'requires'
            AND dl.level < 50
    )
    SELECT
        dl.condition_id,
        MAX(dl.level) AS dependency_level
    FROM dependency_levels dl
    GROUP BY dl.condition_id
    ORDER BY dependency_level, condition_id;
END;
$$;

-- ---------------------------------------------------------------------------
-- get_evaluable_frontier()
-- ---------------------------------------------------------------------------
-- Returns conditions whose 'requires' parents are all resolved, that are not
-- yet resolved or excluded, and that are not excluded by any resolved
-- exclusion dependency. The DAG equivalent of tasker-core's "ready steps".
-- ---------------------------------------------------------------------------
CREATE FUNCTION get_evaluable_frontier(
    input_session_id uuid,
    input_story_id uuid
)
RETURNS TABLE(
    condition_id uuid,
    condition_name text,
    condition_type condition_type,
    narrative_weight real,
    dependency_level integer,
    total_requires integer,
    resolved_requires integer
)
LANGUAGE plpgsql STABLE
AS $$
BEGIN
    RETURN QUERY
    WITH levels AS (
        SELECT * FROM calculate_event_dependency_levels(input_story_id)
    ),
    -- Current resolution state for this session
    states AS (
        SELECT ecs.condition_id, ecs.resolution_state
        FROM event_condition_states ecs
        WHERE ecs.session_id = input_session_id
    ),
    -- Count requires dependencies and how many are resolved
    requires_status AS (
        SELECT
            ec.id AS condition_id,
            COUNT(ed.from_condition_id) AS total_requires,
            COUNT(ed.from_condition_id) FILTER (
                WHERE COALESCE(s.resolution_state, 'unresolved') = 'resolved'
            ) AS resolved_requires
        FROM event_conditions ec
        LEFT JOIN event_dependencies ed
            ON ed.to_condition_id = ec.id AND ed.dependency_type = 'requires'
        LEFT JOIN states s ON s.condition_id = ed.from_condition_id
        WHERE ec.story_id = input_story_id
        GROUP BY ec.id
    )
    SELECT
        ec.id AS condition_id,
        ec.name AS condition_name,
        ec.condition_type,
        ec.narrative_weight,
        COALESCE(l.dependency_level, 0),
        COALESCE(rs.total_requires, 0)::integer,
        COALESCE(rs.resolved_requires, 0)::integer
    FROM event_conditions ec
    JOIN requires_status rs ON rs.condition_id = ec.id
    LEFT JOIN levels l ON l.condition_id = ec.id
    LEFT JOIN states s ON s.condition_id = ec.id
    WHERE ec.story_id = input_story_id
        -- Not yet resolved or excluded
        AND COALESCE(s.resolution_state, 'unresolved') = 'unresolved'
        -- All requires-parents are resolved
        AND rs.total_requires = rs.resolved_requires
        -- Not excluded by any resolved exclusion
        AND NOT EXISTS (
            SELECT 1
            FROM event_dependencies excl
            JOIN states excl_state ON excl_state.condition_id = excl.from_condition_id
            WHERE excl.to_condition_id = ec.id
                AND excl.dependency_type = 'excludes'
                AND excl_state.resolution_state = 'resolved'
        )
    ORDER BY l.dependency_level, ec.narrative_weight DESC;
END;
$$;

-- ---------------------------------------------------------------------------
-- get_transitive_dependencies()
-- ---------------------------------------------------------------------------
-- Walks the dependency chain from a target condition back to its transitive
-- parents. Returns all ancestors with distance and dependency type.
-- Adapted from tasker-core's get_step_transitive_dependencies().
-- ---------------------------------------------------------------------------
CREATE FUNCTION get_transitive_dependencies(target_condition_id uuid)
RETURNS TABLE(
    condition_id uuid,
    condition_name text,
    dep_type dependency_type,
    distance integer
)
LANGUAGE plpgsql STABLE
AS $$
BEGIN
    RETURN QUERY
    WITH RECURSIVE transitive_deps AS (
        -- Base case: direct parents
        SELECT
            ec.id AS condition_id,
            ec.name AS condition_name,
            ed.dependency_type,
            1 AS distance
        FROM event_dependencies ed
        JOIN event_conditions ec ON ec.id = ed.from_condition_id
        WHERE ed.to_condition_id = target_condition_id

        UNION ALL

        -- Recursive case: parents of parents
        SELECT
            ec.id AS condition_id,
            ec.name AS condition_name,
            ed.dependency_type,
            td.distance + 1
        FROM transitive_deps td
        JOIN event_dependencies ed ON ed.to_condition_id = td.condition_id
        JOIN event_conditions ec ON ec.id = ed.from_condition_id
        WHERE td.distance < 50
    )
    SELECT
        td.condition_id,
        td.condition_name,
        td.dependency_type,
        td.distance
    FROM transitive_deps td
    ORDER BY td.distance ASC, td.condition_id;
END;
$$;

-- ---------------------------------------------------------------------------
-- event_dag_overview view
-- ---------------------------------------------------------------------------
-- Structural overview of the Event DAG: parent/child counts, root/leaf
-- detection, and dependency depth. Adapted from tasker-core's
-- step_dag_relationships view.
-- ---------------------------------------------------------------------------
CREATE VIEW event_dag_overview AS
SELECT
    ec.id AS condition_id,
    ec.story_id,
    ec.name,
    ec.condition_type,
    ec.narrative_weight,
    COALESCE(parent_data.parent_count, 0) AS parent_count,
    COALESCE(child_data.child_count, 0) AS child_count,
    COALESCE(parent_data.parent_count, 0) = 0 AS is_root,
    COALESCE(child_data.child_count, 0) = 0 AS is_leaf,
    depth_info.dependency_level
FROM event_conditions ec
LEFT JOIN (
    SELECT to_condition_id, COUNT(*) AS parent_count
    FROM event_dependencies
    WHERE dependency_type = 'requires'
    GROUP BY to_condition_id
) parent_data ON parent_data.to_condition_id = ec.id
LEFT JOIN (
    SELECT from_condition_id, COUNT(*) AS child_count
    FROM event_dependencies
    WHERE dependency_type = 'requires'
    GROUP BY from_condition_id
) child_data ON child_data.from_condition_id = ec.id
LEFT JOIN (
    WITH RECURSIVE step_depths AS (
        SELECT ec_inner.id AS condition_id, 0 AS depth_from_root, ec_inner.story_id
        FROM event_conditions ec_inner
        WHERE NOT EXISTS (
            SELECT 1 FROM event_dependencies e
            WHERE e.to_condition_id = ec_inner.id AND e.dependency_type = 'requires'
        )
        UNION ALL
        SELECT e.to_condition_id, sd.depth_from_root + 1, sd.story_id
        FROM step_depths sd
        JOIN event_dependencies e ON e.from_condition_id = sd.condition_id
        WHERE e.dependency_type = 'requires' AND sd.depth_from_root < 50
    )
    SELECT condition_id, MIN(depth_from_root) AS dependency_level
    FROM step_depths
    GROUP BY condition_id
) depth_info ON depth_info.condition_id = ec.id;

#!/usr/bin/env bash
# =============================================================================
# Generate Database Schema ER Diagram (Mermaid)
# =============================================================================
#
# Parses SQL migration files from storyteller-storykeeper to extract table
# definitions, enum types, foreign key relationships, and SQL functions,
# then generates a Mermaid erDiagram plus supporting reference tables.
#
# Also reads graph-schema.toml for planned AGE graph labels (TAS-244/245).
#
# Source files:
#   crates/storyteller-storykeeper/migrations/*.sql
#   tools/cargo-make/scripts/graph-schema.toml
#
# Compatible with macOS bash 3.2 (no associative arrays or GNU extensions).
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
OUTPUT="${REPO_ROOT}/docs/generated/database-schema.md"
MIGRATIONS_DIR="${REPO_ROOT}/crates/storyteller-storykeeper/migrations"
GRAPH_TOML="${SCRIPT_DIR}/graph-schema.toml"

if [[ ! -d "${MIGRATIONS_DIR}" ]]; then
    echo "ERROR: Migrations directory not found: ${MIGRATIONS_DIR}"
    exit 1
fi

echo "Generating database schema ER diagram..."
echo "  Source: ${MIGRATIONS_DIR}"

TMPDIR_WORK=$(mktemp -d)
trap 'rm -rf "${TMPDIR_WORK}"' EXIT

# ---------------------------------------------------------------------------
# Parse table definitions from all migration files
# Output: TABLE|name and COL|table|col|type|pk|fk_target lines
# ---------------------------------------------------------------------------
parse_tables() {
    local current_table=""
    local in_table=false

    for file in "${MIGRATIONS_DIR}"/*.sql; do
        while IFS= read -r line; do
            local stripped
            stripped=$(echo "${line}" | sed 's/^[[:space:]]*//')

            # Detect CREATE TABLE
            if echo "${stripped}" | grep -qi '^CREATE[[:space:]]*TABLE[[:space:]]'; then
                current_table=$(echo "${stripped}" | sed -E 's/CREATE TABLE ([a-z_]+).*/\1/')
                in_table=true
                echo "TABLE|${current_table}"
                continue
            fi

            # Skip non-table CREATE statements
            if echo "${stripped}" | grep -qi '^CREATE[[:space:]]*\(VIEW\|TYPE\|EXTENSION\|FUNCTION\|INDEX\|UNIQUE\)'; then
                in_table=false
                continue
            fi

            # End of table on ALTER TABLE or other DDL
            if echo "${stripped}" | grep -qi '^ALTER[[:space:]]*TABLE'; then
                in_table=false
                continue
            fi

            if [[ "${in_table}" != true ]]; then
                continue
            fi

            # End of CREATE TABLE block
            case "${stripped}" in
                ")"*) in_table=false; continue ;;
            esac

            # Skip constraints, comments, empty lines, UNIQUE lines
            if echo "${stripped}" | grep -qi '^CONSTRAINT\|^--\|^UNIQUE'; then
                continue
            fi
            if [[ -z "${stripped}" ]]; then
                continue
            fi

            # Parse column definition
            col_name=$(echo "${stripped}" | sed -E 's/^([a-z_]+)[[:space:]].*/\1/')
            # Validate it looks like a column name
            case "${col_name}" in
                [a-z]*)  ;; # valid
                *) continue ;;
            esac

            # Determine simplified type
            col_type="other"
            if echo "${stripped}" | grep -qi 'uuid\[\]'; then
                col_type="uuid_array"
            elif echo "${stripped}" | grep -qi 'uuid'; then
                col_type="uuid"
            elif echo "${stripped}" | grep -qi 'character[[:space:]]*varying\|varchar'; then
                col_type="varchar"
            elif echo "${stripped}" | grep -qi '[[:space:]]int[[:space:]]\|[[:space:]]int,\|[[:space:]]int$\|integer'; then
                col_type="integer"
            elif echo "${stripped}" | grep -qi 'boolean'; then
                col_type="boolean"
            elif echo "${stripped}" | grep -qi 'timestamptz\|timestamp'; then
                col_type="timestamptz"
            elif echo "${stripped}" | grep -qi 'jsonb'; then
                col_type="jsonb"
            elif echo "${stripped}" | grep -qi 'text'; then
                col_type="text"
            elif echo "${stripped}" | grep -qi 'bigint'; then
                col_type="bigint"
            elif echo "${stripped}" | grep -qi 'real'; then
                col_type="real"
            else
                # Check for enum types (lowercase type names that aren't SQL keywords)
                local type_word
                type_word=$(echo "${stripped}" | sed -E 's/^[a-z_]+[[:space:]]+([a-z_]+).*/\1/')
                case "${type_word}" in
                    event_priority|event_type|event_kind|provisional_status|scene_type| \
                    entity_origin|persistence_mode|promotion_tier|session_status| \
                    scene_instance_status|layer_type|scene_provenance|scene_activation| \
                    condition_type|dependency_type|resolution_state)
                        col_type="enum(${type_word})"
                        ;;
                esac
            fi

            # Check if PK (uuid column with uuidv7() default)
            is_pk="false"
            if [[ "${col_name}" == "id" ]] && [[ "${col_type}" == "uuid" ]]; then
                if echo "${stripped}" | grep -qi 'uuidv7\|PRIMARY[[:space:]]*KEY'; then
                    is_pk="true"
                fi
            fi

            # Check for FK (inline REFERENCES clause)
            fk_target=""
            if echo "${stripped}" | grep -qi 'REFERENCES'; then
                fk_target=$(echo "${stripped}" | sed -E 's/.*REFERENCES ([a-z_]+)\(([a-z_]+)\).*/\1.\2/')
            fi

            echo "COL|${current_table}|${col_name}|${col_type}|${is_pk}|${fk_target}"
        done < "${file}"
    done
}

# ---------------------------------------------------------------------------
# Parse enum types from migration files
# Output: ENUM|name|value1,value2,...
# ---------------------------------------------------------------------------
parse_enums() {
    local in_enum=false
    local enum_name=""
    local enum_values=""

    for file in "${MIGRATIONS_DIR}"/*.sql; do
        while IFS= read -r line; do
            local stripped
            stripped=$(echo "${line}" | sed 's/^[[:space:]]*//')

            # Detect CREATE TYPE ... AS ENUM
            if echo "${stripped}" | grep -qi '^CREATE[[:space:]]*TYPE.*AS[[:space:]]*ENUM'; then
                enum_name=$(echo "${stripped}" | sed -E "s/CREATE TYPE ([a-z_]+) AS ENUM.*/\1/")
                # Check if single-line enum
                if echo "${stripped}" | grep -q ');'; then
                    enum_values=$(echo "${stripped}" | sed -E "s/.*\(([^)]+)\).*/\1/" | sed "s/'//g" | sed 's/[[:space:]]*//g')
                    echo "ENUM|${enum_name}|${enum_values}"
                    enum_name=""
                else
                    in_enum=true
                    enum_values=""
                fi
                continue
            fi

            if [[ "${in_enum}" == true ]]; then
                # End of enum
                if echo "${stripped}" | grep -q ');'; then
                    # Capture any trailing values
                    local val
                    val=$(echo "${stripped}" | sed "s/[);]//g" | sed "s/'//g" | sed 's/^[[:space:]]*//' | sed 's/[[:space:]]*$//')
                    if [[ -n "${val}" ]]; then
                        if [[ -n "${enum_values}" ]]; then
                            enum_values="${enum_values},${val}"
                        else
                            enum_values="${val}"
                        fi
                    fi
                    echo "ENUM|${enum_name}|${enum_values}"
                    in_enum=false
                    enum_name=""
                    enum_values=""
                else
                    # Accumulate values — strip quotes, commas, whitespace
                    local val
                    val=$(echo "${stripped}" | sed "s/'//g" | sed 's/,/ /g' | sed 's/^[[:space:]]*//' | sed 's/[[:space:]]*$//' | tr -s ' ' | sed 's/ /,/g')
                    # Skip comments
                    if echo "${val}" | grep -q '^--'; then
                        continue
                    fi
                    if [[ -n "${val}" ]] && ! echo "${val}" | grep -q '^('; then
                        # Strip leading paren
                        val=$(echo "${val}" | sed 's/^(//')
                        if [[ -n "${val}" ]]; then
                            if [[ -n "${enum_values}" ]]; then
                                enum_values="${enum_values},${val}"
                            else
                                enum_values="${val}"
                            fi
                        fi
                    fi
                fi
            fi
        done < "${file}"
    done
}

# ---------------------------------------------------------------------------
# Parse SQL functions and views
# Output: FUNC|name|args or VIEW|name
# ---------------------------------------------------------------------------
parse_functions() {
    local in_func_sig=false
    local func_accum=""

    for file in "${MIGRATIONS_DIR}"/*.sql; do
        while IFS= read -r line; do
            local stripped
            stripped=$(echo "${line}" | sed 's/^[[:space:]]*//')

            # Continue accumulating multiline function signature
            if [[ "${in_func_sig}" == true ]]; then
                func_accum="${func_accum} ${stripped}"
                if echo "${func_accum}" | grep -q ')'; then
                    local func_sig
                    func_sig=$(echo "${func_accum}" | sed -E 's/CREATE FUNCTION ([a-z_]+)\(([^)]*)\).*/\1(\2)/' | sed 's/  */ /g')
                    echo "FUNC|${func_sig}"
                    in_func_sig=false
                    func_accum=""
                fi
                continue
            fi

            # Detect CREATE FUNCTION
            if echo "${stripped}" | grep -qi '^CREATE[[:space:]]*FUNCTION'; then
                # Check if signature is complete on this line
                if echo "${stripped}" | grep -q ')'; then
                    local func_sig
                    func_sig=$(echo "${stripped}" | sed -E 's/CREATE FUNCTION ([a-z_]+\([^)]*\)).*/\1/')
                    echo "FUNC|${func_sig}"
                else
                    # Multiline signature — start accumulating
                    in_func_sig=true
                    func_accum="${stripped}"
                fi
            fi

            # Detect CREATE VIEW
            if echo "${stripped}" | grep -qi '^CREATE[[:space:]]*VIEW'; then
                local view_name
                view_name=$(echo "${stripped}" | sed -E 's/CREATE VIEW ([a-z_]+).*/\1/')
                echo "VIEW|${view_name}"
            fi
        done < "${file}"
    done
}

# ---------------------------------------------------------------------------
# Parse graph-schema.toml for AGE vertex/edge definitions
# Simple line-based parser (no TOML library needed)
# ---------------------------------------------------------------------------
parse_graph_schema() {
    if [[ ! -f "${GRAPH_TOML}" ]]; then
        return
    fi

    local section=""
    local label="" graph="" description="" from="" to=""
    local props=""

    flush_entry() {
        if [[ -n "${label}" ]]; then
            if [[ "${section}" == "vertex" ]]; then
                echo "VERTEX|${label}|${graph}|${description}|${props}"
            elif [[ "${section}" == "edge" ]]; then
                echo "EDGE|${label}|${graph}|${from}|${to}|${description}|${props}"
            fi
        fi
        label="" graph="" description="" from="" to="" props=""
    }

    while IFS= read -r line; do
        local stripped
        stripped=$(echo "${line}" | sed 's/^[[:space:]]*//' | sed 's/[[:space:]]*$//')

        # Skip comments and empty lines
        if [[ -z "${stripped}" ]] || echo "${stripped}" | grep -q '^#'; then
            continue
        fi

        # Section headers
        if [[ "${stripped}" == "[[vertex]]" ]]; then
            flush_entry
            section="vertex"
            continue
        fi
        if [[ "${stripped}" == "[[edge]]" ]]; then
            flush_entry
            section="edge"
            continue
        fi

        # Key-value pairs
        local key val
        key=$(echo "${stripped}" | sed -E 's/^([a-z_]+)[[:space:]]*=.*/\1/')
        val=$(echo "${stripped}" | sed -E 's/^[a-z_]+[[:space:]]*=[[:space:]]*//' | sed 's/^"//' | sed 's/"$//')

        case "${key}" in
            label) label="${val}" ;;
            graph) graph="${val}" ;;
            description) description="${val}" ;;
            from) from="${val}" ;;
            to) to="${val}" ;;
            properties)
                props=$(echo "${val}" | sed 's/^\[//' | sed 's/\]$//' | sed 's/"//g')
                ;;
        esac
    done < "${GRAPH_TOML}"
    flush_entry
}

# ---------------------------------------------------------------------------
# Collect parsed data into temp files
# ---------------------------------------------------------------------------
parse_tables > "${TMPDIR_WORK}/table_data.txt"
parse_enums > "${TMPDIR_WORK}/enum_data.txt"
parse_functions > "${TMPDIR_WORK}/func_data.txt"
parse_graph_schema > "${TMPDIR_WORK}/graph_data.txt"

# Extract table names
TABLES=()
while IFS='|' read -r type name rest; do
    if [[ "${type}" == "TABLE" ]]; then
        TABLES+=("${name}")
    fi
done < "${TMPDIR_WORK}/table_data.txt"

# Extract FK relationships
FK_LINES=()
while IFS='|' read -r type tbl col ctype is_pk fk_target; do
    if [[ "${type}" == "COL" ]] && [[ -n "${fk_target}" ]]; then
        tgt_table=$(echo "${fk_target}" | cut -d. -f1)
        tgt_col=$(echo "${fk_target}" | cut -d. -f2)
        FK_LINES+=("${tbl}|${col}|${tgt_table}|${tgt_col}")
    fi
done < "${TMPDIR_WORK}/table_data.txt"

# Also capture deferred FKs from ALTER TABLE ... ADD CONSTRAINT ... FOREIGN KEY
parse_deferred_fks() {
    for file in "${MIGRATIONS_DIR}"/*.sql; do
        local current_table=""
        while IFS= read -r line; do
            local stripped
            stripped=$(echo "${line}" | sed 's/^[[:space:]]*//')

            if echo "${stripped}" | grep -qi '^ALTER[[:space:]]*TABLE'; then
                current_table=$(echo "${stripped}" | sed -E 's/ALTER TABLE ([a-z_]+).*/\1/')
            fi

            if echo "${stripped}" | grep -qi 'FOREIGN[[:space:]]*KEY'; then
                if [[ -n "${current_table}" ]]; then
                    local src_col tgt_table tgt_col
                    src_col=$(echo "${stripped}" | sed -E 's/.*FOREIGN KEY \(([^)]+)\).*/\1/')
                    tgt_table=$(echo "${stripped}" | sed -E 's/.*REFERENCES ([a-z_]+)\(.*/\1/')
                    tgt_col=$(echo "${stripped}" | sed -E 's/.*REFERENCES [a-z_]+\(([^)]+)\).*/\1/')
                    echo "${current_table}|${src_col}|${tgt_table}|${tgt_col}"
                fi
            fi
        done < "${file}"
    done
}

parse_deferred_fks > "${TMPDIR_WORK}/deferred_fk_data.txt"

# Merge deferred FKs
while IFS= read -r fk_line; do
    if [[ -n "${fk_line}" ]]; then
        FK_LINES+=("${fk_line}")
    fi
done < "${TMPDIR_WORK}/deferred_fk_data.txt"

# Write merged FKs
printf '%s\n' "${FK_LINES[@]}" > "${TMPDIR_WORK}/all_fk_data.txt"

# ---------------------------------------------------------------------------
# Generate output
# ---------------------------------------------------------------------------
mkdir -p "$(dirname "${OUTPUT}")"

{
    cat <<'HEADER'
# Storyteller Database Schema

> Auto-generated from SQL migration analysis. Do not edit manually.
>
> Regenerate with: `cargo make generate-db-schema`

The Storyteller database uses PostgreSQL 18 with Apache AGE for graph queries.
All tables use UUID v7 primary keys (`uuidv7()`) for time-ordered identifiers.
The schema lives in the `public` schema (no schema prefix).

## Entity Relationship Diagram

```mermaid
erDiagram
HEADER

    # Emit table entities with columns
    for table in "${TABLES[@]}"; do
        echo "    ${table} {"
        while IFS='|' read -r type tbl col ctype is_pk fk_target; do
            if [[ "${type}" != "COL" ]] || [[ "${tbl}" != "${table}" ]]; then
                continue
            fi
            marker=""
            if [[ "${is_pk}" == "true" ]]; then
                marker=" PK"
            elif [[ -n "${fk_target}" ]]; then
                marker=" FK"
            fi
            # Simplify enum type for display
            display_type="${ctype}"
            if echo "${ctype}" | grep -q '^enum('; then
                display_type="enum"
            fi
            echo "        ${display_type} ${col}${marker}"
        done < "${TMPDIR_WORK}/table_data.txt"
        echo "    }"
    done

    echo ""

    # Emit relationships from all foreign keys
    while IFS='|' read -r src_table src_col tgt_table tgt_col; do
        if [[ -n "${src_table}" ]] && [[ -n "${tgt_table}" ]]; then
            echo "    ${tgt_table} ||--o{ ${src_table} : \"${src_col}\""
        fi
    done < "${TMPDIR_WORK}/all_fk_data.txt"

    echo '```'
    echo ""

    # --- Enum Types ---
    echo "## Enum Types"
    echo ""
    echo "| Enum | Values |"
    echo "|------|--------|"

    while IFS='|' read -r type name values; do
        if [[ "${type}" == "ENUM" ]]; then
            # Format values as inline code
            formatted=$(echo "${values}" | sed 's/,/, /g')
            echo "| \`${name}\` | ${formatted} |"
        fi
    done < "${TMPDIR_WORK}/enum_data.txt"

    echo ""

    # --- Tables ---
    echo "## Tables"
    echo ""
    echo "| # | Table | Columns | Description |"
    echo "|---|-------|---------|-------------|"

    table_num=0
    for table in "${TABLES[@]}"; do
        table_num=$((table_num + 1))
        col_count=0
        while IFS='|' read -r type tbl col ctype is_pk fk_target; do
            if [[ "${type}" == "COL" ]] && [[ "${tbl}" == "${table}" ]]; then
                col_count=$((col_count + 1))
            fi
        done < "${TMPDIR_WORK}/table_data.txt"

        # Derive description from table name
        desc=""
        case "${table}" in
            stories) desc="Top-level story container" ;;
            settings) desc="Spatial locations for scenes (setting topology vertices)" ;;
            sub_graph_layers) desc="Narrative sub-graph layers for tales-within-tales" ;;
            scenes) desc="Scene templates with gravitational mass and cast" ;;
            players) desc="Player identity" ;;
            entities) desc="Entity lifecycle tracking with promotion tiers" ;;
            sessions) desc="Player session lifecycle" ;;
            scene_instances) desc="Specific playthrough of a scene within a session" ;;
            characters) desc="Versioned character sheets (tensor snapshots)" ;;
            turns) desc="Atomic unit of play — player input through rendering" ;;
            event_ledger) desc="Append-only record of committed events" ;;
            scene_transitions) desc="Authored metadata for possible scene transitions" ;;
            scene_activation_states) desc="Session-scoped scene activation lifecycle" ;;
            event_conditions) desc="DAG nodes — narrative conditions resolvable during play" ;;
            event_dependencies) desc="DAG edges — typed relationships between conditions" ;;
            event_condition_states) desc="Session-scoped event condition resolution tracking" ;;
            *) desc="" ;;
        esac
        echo "| ${table_num} | \`${table}\` | ${col_count} | ${desc} |"
    done

    echo ""

    # --- Foreign Key Relationships ---
    echo "## Foreign Key Relationships"
    echo ""
    echo "| Source Table | Column | Target Table | Target Column |"
    echo "|-------------|--------|-------------|---------------|"

    while IFS='|' read -r src_table src_col tgt_table tgt_col; do
        if [[ -n "${src_table}" ]] && [[ -n "${tgt_table}" ]]; then
            echo "| \`${src_table}\` | \`${src_col}\` | \`${tgt_table}\` | \`${tgt_col}\` |"
        fi
    done < "${TMPDIR_WORK}/all_fk_data.txt"

    echo ""

    # --- SQL Functions and Views ---
    echo "## SQL Functions and Views"
    echo ""

    has_func=false
    while IFS='|' read -r type name; do
        if [[ "${type}" == "FUNC" ]]; then
            if [[ "${has_func}" == false ]]; then
                echo "### Functions"
                echo ""
                echo "| Function | Description |"
                echo "|----------|-------------|"
                has_func=true
            fi
            desc=""
            case "${name}" in
                calculate_event_dependency_levels*) desc="Compute topological levels in the Event DAG via recursive CTE" ;;
                get_evaluable_frontier*) desc="Find conditions whose requirements are met and can be evaluated" ;;
                get_transitive_dependencies*) desc="Walk dependency chain to find all transitive ancestors" ;;
                *) desc="" ;;
            esac
            echo "| \`${name}\` | ${desc} |"
        fi
    done < "${TMPDIR_WORK}/func_data.txt"

    echo ""

    has_view=false
    while IFS='|' read -r type name; do
        if [[ "${type}" == "VIEW" ]]; then
            if [[ "${has_view}" == false ]]; then
                echo "### Views"
                echo ""
                echo "| View | Description |"
                echo "|------|-------------|"
                has_view=true
            fi
            desc=""
            case "${name}" in
                event_dag_overview) desc="Structural overview: parent/child counts, root/leaf detection, dependency depth" ;;
                *) desc="" ;;
            esac
            echo "| \`${name}\` | ${desc} |"
        fi
    done < "${TMPDIR_WORK}/func_data.txt"

    echo ""

    # --- AGE Graph Schema (Planned) ---
    if [[ -f "${GRAPH_TOML}" ]] && [[ -s "${TMPDIR_WORK}/graph_data.txt" ]]; then
        echo "## Apache AGE Graph Schema (Planned)"
        echo ""
        echo "> These graph labels are **planned for TAS-244/245** and are not yet migrated."
        echo "> Source: \`tools/cargo-make/scripts/graph-schema.toml\`"
        echo ""
        echo '```mermaid'
        echo 'graph LR'

        # Emit vertices
        while IFS='|' read -r type label graph description props; do
            if [[ "${type}" == "VERTEX" ]]; then
                echo "    ${label}[\":${label}<br/>${graph}\"]"
            fi
        done < "${TMPDIR_WORK}/graph_data.txt"

        # Emit edges
        while IFS='|' read -r type label graph efrom eto description props; do
            if [[ "${type}" == "EDGE" ]]; then
                echo "    ${efrom} -->|:${label}| ${eto}"
            fi
        done < "${TMPDIR_WORK}/graph_data.txt"

        echo '```'
        echo ""

        echo "### Vertex Labels"
        echo ""
        echo "| Label | Graph | Properties | Description |"
        echo "|-------|-------|------------|-------------|"

        while IFS='|' read -r type label graph description props; do
            if [[ "${type}" == "VERTEX" ]]; then
                formatted_props=$(echo "${props}" | sed 's/, /\`, \`/g')
                if [[ -n "${formatted_props}" ]]; then
                    formatted_props="\`${formatted_props}\`"
                fi
                echo "| \`:${label}\` | ${graph} | ${formatted_props} | ${description} |"
            fi
        done < "${TMPDIR_WORK}/graph_data.txt"

        echo ""
        echo "### Edge Labels"
        echo ""
        echo "| Label | Graph | From | To | Properties | Description |"
        echo "|-------|-------|------|----|------------|-------------|"

        while IFS='|' read -r type label graph efrom eto description props; do
            if [[ "${type}" == "EDGE" ]]; then
                formatted_props=$(echo "${props}" | sed 's/, /\`, \`/g')
                if [[ -n "${formatted_props}" ]]; then
                    formatted_props="\`${formatted_props}\`"
                fi
                echo "| \`:${label}\` | ${graph} | :${efrom} | :${eto} | ${formatted_props} | ${description} |"
            fi
        done < "${TMPDIR_WORK}/graph_data.txt"

        echo ""
    fi

    echo "---"
    echo ""
    echo "*Generated by \`generate-db-schema.sh\` from storyteller-storykeeper SQL migration analysis*"

} > "${OUTPUT}"

# Count results
enum_count=$(grep -c '^ENUM|' "${TMPDIR_WORK}/enum_data.txt" || true)
func_count=$(grep -c '^FUNC\|^VIEW' "${TMPDIR_WORK}/func_data.txt" || true)

echo "  Output: ${OUTPUT}"
echo "Database schema ER diagram generated (${#TABLES[@]} tables, ${enum_count} enums, ${func_count} functions/views)."

# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Ground-state loader: populates PostgreSQL ground_state tables from the narrative corpus.

Two-phase load:
  Phase 1 — Reference entities: genres, clusters, cluster members, state variables, dimensions.
  Phase 2 — Primitive types: one file per (genre × type) or (cluster × type), upserted with
             source-hash drift detection so re-runs are idempotent.
"""

import hashlib
import json
import logging
from dataclasses import dataclass, field
from pathlib import Path

import psycopg
from psycopg.rows import dict_row

from narrative_data.config import GENRE_NATIVE_TYPES, PRIMITIVE_TYPES
from narrative_data.persistence.reference_data import (
    extract_cluster_metadata,
    extract_dimensions,
    extract_genres,
    extract_state_variables,
)

log = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Table map: type slug → ground_state table name
# ---------------------------------------------------------------------------

TABLE_MAP: dict[str, str] = {
    "archetypes": "archetypes",
    "dynamics": "dynamics",
    "settings": "settings",
    "goals": "goals",
    "profiles": "profiles",
    "tropes": "tropes",
    "narrative-shapes": "narrative_shapes",
    "ontological-posture": "ontological_posture",
    "spatial-topology": "spatial_topology",
    "place-entities": "place_entities",
    "archetype-dynamics": "archetype_dynamics",
    "genre-dimensions": "genre_dimensions",
}

ALL_PRIMITIVE_TYPES: list[str] = PRIMITIVE_TYPES + GENRE_NATIVE_TYPES


# ---------------------------------------------------------------------------
# LoadReport
# ---------------------------------------------------------------------------


@dataclass
class LoadReport:
    """Aggregate counts from a load run."""

    inserted: int = 0
    updated: int = 0
    pruned: int = 0
    skipped: int = 0
    errors: int = 0
    error_details: list[str] = field(default_factory=list)

    def __add__(self, other: "LoadReport") -> "LoadReport":
        return LoadReport(
            inserted=self.inserted + other.inserted,
            updated=self.updated + other.updated,
            pruned=self.pruned + other.pruned,
            skipped=self.skipped + other.skipped,
            errors=self.errors + other.errors,
            error_details=self.error_details + other.error_details,
        )

    def summary(self) -> str:
        return (
            f"inserted={self.inserted} updated={self.updated} "
            f"pruned={self.pruned} skipped={self.skipped} errors={self.errors}"
        )


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _slugify(name: str) -> str:
    """Convert a canonical name to a slug: lowercase, spaces → hyphens."""
    return name.strip().lower().replace(" ", "-").replace("_", "-")


def _entity_slug(entity: dict) -> str | None:
    """Derive the entity slug from canonical_name, name, pairing_name, or default_subject."""
    for key in ("canonical_name", "pairing_name", "name", "default_subject"):
        val = entity.get(key)
        if val and isinstance(val, str):
            return _slugify(val)
    return None


def _source_hash(raw_bytes: bytes) -> str:
    """Return the SHA256 hex digest of raw file bytes."""
    return hashlib.sha256(raw_bytes).hexdigest()


def _promoted_columns(type_name: str, entity: dict) -> dict:
    """Extract type-specific promoted columns beyond the shared set."""
    if type_name == "archetypes":
        return {
            "archetype_family": entity.get("universality"),
            "primary_scale": None,
        }
    if type_name == "settings":
        # setting_type: try communicability.spatial.enclosure or narrative_function
        communicability = entity.get("communicability") or {}
        spatial = communicability.get("spatial") if isinstance(communicability, dict) else None
        if isinstance(spatial, dict):
            setting_type = spatial.get("enclosure")
        elif isinstance(spatial, str):
            setting_type = spatial
        else:
            setting_type = entity.get("narrative_function")
            setting_type = setting_type if isinstance(setting_type, str) else None
        return {"setting_type": setting_type}
    if type_name == "dynamics":
        return {
            "edge_type": entity.get("edge_type"),
            "scale": entity.get("scale"),
        }
    if type_name == "goals":
        return {"goal_scale": entity.get("scale")}
    if type_name == "profiles":
        return {"archetype_ref": entity.get("provenance")}
    if type_name == "tropes":
        # trope_family: from genre_derivation or narrative_function list
        derivation = entity.get("genre_derivation")
        if isinstance(derivation, str) and derivation:
            trope_family = derivation.split(":")[0].strip() if ":" in derivation else derivation
        else:
            trope_family = None
        return {"trope_family": trope_family}
    if type_name == "narrative-shapes":
        tension_profile = entity.get("tension_profile") or {}
        shape_type = tension_profile.get("family") if isinstance(tension_profile, dict) else None
        beats = entity.get("beats") or []
        beat_count = len(beats) if isinstance(beats, list) else None
        return {"shape_type": shape_type, "beat_count": beat_count}
    if type_name == "ontological-posture":
        boundary = entity.get("self_other_boundary") or {}
        stability = boundary.get("stability") if isinstance(boundary, dict) else None
        return {"boundary_stability": stability}
    if type_name == "spatial-topology":
        friction = entity.get("friction") or {}
        friction_type = friction.get("type") if isinstance(friction, dict) else None
        directionality = entity.get("directionality") or {}
        directionality_type = (
            directionality.get("type") if isinstance(directionality, dict) else None
        )
        return {"friction_type": friction_type, "directionality_type": directionality_type}
    if type_name == "place-entities":
        entity_props = entity.get("entity_properties") or {}
        place_type = entity_props.get("type") if isinstance(entity_props, dict) else None
        return {"place_type": place_type}
    if type_name == "archetype-dynamics":
        return {
            "archetype_a": entity.get("archetype_a"),
            "archetype_b": entity.get("archetype_b"),
        }
    return {}


def _entity_name(type_name: str, entity: dict) -> str:
    """Return a human-readable name for the entity."""
    for key in ("canonical_name", "pairing_name", "name", "default_subject"):
        val = entity.get(key)
        if val and isinstance(val, str):
            return val.strip()
    return "unknown"


# ---------------------------------------------------------------------------
# Phase 1: Reference entity loaders
# ---------------------------------------------------------------------------


def load_reference_entities(
    conn: psycopg.Connection,
    corpus_dir: Path,
    dry_run: bool = False,
) -> dict[str, str]:
    """Load genres, clusters, state variables, and dimensions.

    Returns a slug→UUID lookup map covering:
      - genre slugs → genre UUIDs
      - cluster slugs → cluster UUIDs
    """
    slug_map: dict[str, str] = {}

    genres = extract_genres(corpus_dir)
    slug_map.update(_upsert_genres(conn, genres, dry_run=dry_run))

    clusters = extract_cluster_metadata()
    cluster_map = _upsert_clusters(conn, clusters, slug_map, dry_run=dry_run)
    slug_map.update(cluster_map)

    state_variables = extract_state_variables(corpus_dir)
    _upsert_state_variables(conn, state_variables, dry_run=dry_run)

    dimensions = extract_dimensions()
    _upsert_dimensions(conn, dimensions, dry_run=dry_run)

    if not dry_run:
        conn.commit()

    log.info(
        "Phase 1 complete: %d genres, %d clusters, %d state_variables, %d dimensions",
        len(genres),
        len(clusters),
        len(state_variables),
        len(dimensions),
    )
    return slug_map


def _upsert_genres(
    conn: psycopg.Connection,
    genres: list[dict],
    dry_run: bool = False,
) -> dict[str, str]:
    """Upsert genres; return slug→UUID map."""
    slug_map: dict[str, str] = {}
    for genre in genres:
        slug = genre["slug"]
        payload_bytes = json.dumps(genre["payload"], sort_keys=True).encode()
        h = _source_hash(payload_bytes)
        if dry_run:
            slug_map[slug] = "dry-run-uuid"
            continue
        with conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                """
                INSERT INTO ground_state.genres (slug, name, description, payload, source_hash)
                VALUES (%s, %s, %s, %s::jsonb, %s)
                ON CONFLICT (slug) DO UPDATE
                  SET name = EXCLUDED.name,
                      description = EXCLUDED.description,
                      payload = EXCLUDED.payload,
                      source_hash = EXCLUDED.source_hash,
                      updated_at = now()
                  WHERE ground_state.genres.source_hash != EXCLUDED.source_hash
                RETURNING id
                """,
                (slug, genre["name"], genre["description"], json.dumps(genre["payload"]), h),
            )
            # Even if no update occurred, fetch the existing id
            if cur.rowcount == 0:
                cur.execute("SELECT id FROM ground_state.genres WHERE slug = %s", (slug,))
            row = cur.fetchone()
            slug_map[slug] = str(row["id"])
    return slug_map


def _upsert_clusters(
    conn: psycopg.Connection,
    clusters: list[dict],
    genre_slug_map: dict[str, str],
    dry_run: bool = False,
) -> dict[str, str]:
    """Upsert genre_clusters and genre_cluster_members; return cluster slug→UUID map."""
    slug_map: dict[str, str] = {}
    for cluster in clusters:
        slug = cluster["slug"]
        payload = {"genres": cluster["genres"]}
        payload_bytes = json.dumps(payload, sort_keys=True).encode()
        h = _source_hash(payload_bytes)
        if dry_run:
            slug_map[slug] = "dry-run-uuid"
            continue
        with conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                """
                INSERT INTO ground_state.genre_clusters
                    (slug, name, description, payload, source_hash)
                VALUES (%s, %s, NULL, %s::jsonb, %s)
                ON CONFLICT (slug) DO UPDATE
                  SET name = EXCLUDED.name,
                      payload = EXCLUDED.payload,
                      source_hash = EXCLUDED.source_hash,
                      updated_at = now()
                  WHERE ground_state.genre_clusters.source_hash != EXCLUDED.source_hash
                RETURNING id
                """,
                (slug, cluster["name"], json.dumps(payload), h),
            )
            if cur.rowcount == 0:
                cur.execute(
                    "SELECT id FROM ground_state.genre_clusters WHERE slug = %s", (slug,)
                )
            row = cur.fetchone()
            cluster_id = str(row["id"])
            slug_map[slug] = cluster_id

            # Sync cluster members
            for genre_slug in cluster["genres"]:
                genre_id = genre_slug_map.get(genre_slug)
                if not genre_id:
                    log.warning("Cluster %s references unknown genre %s", slug, genre_slug)
                    continue
                cur.execute(
                    """
                    INSERT INTO ground_state.genre_cluster_members (genre_id, cluster_id)
                    VALUES (%s, %s)
                    ON CONFLICT DO NOTHING
                    """,
                    (genre_id, cluster_id),
                )
    return slug_map


def _upsert_state_variables(
    conn: psycopg.Connection,
    state_variables: list[dict],
    dry_run: bool = False,
) -> None:
    """Upsert state_variables (no drift detection needed — no source_hash column)."""
    for sv in state_variables:
        if dry_run:
            continue
        with conn.cursor() as cur:
            cur.execute(
                """
                INSERT INTO ground_state.state_variables
                    (slug, name, description, default_range, payload)
                VALUES (%s, %s, %s, %s::jsonb, %s::jsonb)
                ON CONFLICT (slug) DO UPDATE
                  SET name = EXCLUDED.name,
                      description = EXCLUDED.description,
                      default_range = EXCLUDED.default_range,
                      payload = EXCLUDED.payload,
                      updated_at = now()
                """,
                (
                    sv["slug"],
                    sv["name"],
                    sv["description"],
                    json.dumps(sv["default_range"]) if sv["default_range"] is not None else None,
                    json.dumps(sv["payload"]),
                ),
            )


def _upsert_dimensions(
    conn: psycopg.Connection,
    dimensions: list[dict],
    dry_run: bool = False,
) -> None:
    """Upsert dimensions (no source_hash column)."""
    for dim in dimensions:
        if dry_run:
            continue
        with conn.cursor() as cur:
            cur.execute(
                """
                INSERT INTO ground_state.dimensions
                    (slug, name, dimension_group, description, payload)
                VALUES (%s, %s, %s, %s, NULL)
                ON CONFLICT (slug) DO UPDATE
                  SET name = EXCLUDED.name,
                      dimension_group = EXCLUDED.dimension_group,
                      description = EXCLUDED.description,
                      updated_at = now()
                """,
                (dim["slug"], dim["name"], dim["dimension_group"], dim["description"]),
            )


# ---------------------------------------------------------------------------
# Phase 2: Primitive type loader
# ---------------------------------------------------------------------------


def _corpus_files_for_type(
    type_name: str,
    corpus_dir: Path,
    genre_filter: list[str] | None = None,
) -> list[tuple[Path, str | None, str | None]]:
    """Yield (file_path, genre_slug_or_None, cluster_slug_or_None) tuples for a type.

    For discovery types: corpus_dir/discovery/{type}/{genre}.json
    For genre-native types: corpus_dir/genres/{genre}/{type}.json
    Cluster files: corpus_dir/discovery/{type}/cluster-{name}.json → cluster_slug
    genre-dimensions: corpus_dir/genres/{genre}/genre-dimensions.json
    """
    results: list[tuple[Path, str | None, str | None]] = []

    if type_name == "genre-dimensions":
        genres_dir = corpus_dir / "genres"
        if not genres_dir.exists():
            return results
        for genre_dir in sorted(genres_dir.iterdir()):
            if not genre_dir.is_dir():
                continue
            genre_slug = genre_dir.name
            if genre_filter and genre_slug not in genre_filter:
                continue
            p = genre_dir / "genre-dimensions.json"
            if p.exists():
                results.append((p, genre_slug, None))
        return results

    if type_name in GENRE_NATIVE_TYPES:
        genres_dir = corpus_dir / "genres"
        if not genres_dir.exists():
            return results
        for genre_dir in sorted(genres_dir.iterdir()):
            if not genre_dir.is_dir():
                continue
            genre_slug = genre_dir.name
            if genre_filter and genre_slug not in genre_filter:
                continue
            p = genre_dir / f"{type_name}.json"
            if p.exists():
                results.append((p, genre_slug, None))
        return results

    # Discovery type
    type_dir = corpus_dir / "discovery" / type_name
    if not type_dir.exists():
        return results

    for f in sorted(type_dir.glob("*.json")):
        if f.name == "manifest.json":
            continue
        stem = f.stem  # e.g. "folk-horror" or "cluster-fantasy"
        if stem.startswith("cluster-"):
            cluster_slug = stem
            genre_slug = None
        else:
            genre_slug = stem
            cluster_slug = None

        if genre_filter and genre_slug and genre_slug not in genre_filter:
            continue

        results.append((f, genre_slug, cluster_slug))

    return results


def _upsert_genre_dimensions_row(
    conn: psycopg.Connection,
    genre_id: str,
    payload: dict,
    h: str,
    dry_run: bool = False,
) -> tuple[str, str]:
    """Upsert a genre_dimensions row. Returns ('inserted'|'updated'|'skipped', '')."""
    if dry_run:
        return "skipped", ""
    with conn.cursor(row_factory=dict_row) as cur:
        cur.execute(
            """
            INSERT INTO ground_state.genre_dimensions (genre_id, payload, source_hash)
            VALUES (%s, %s::jsonb, %s)
            ON CONFLICT (genre_id) DO UPDATE
              SET payload = EXCLUDED.payload,
                  source_hash = EXCLUDED.source_hash,
                  updated_at = now()
              WHERE ground_state.genre_dimensions.source_hash != EXCLUDED.source_hash
            """,
            (genre_id, json.dumps(payload), h),
        )
        if cur.rowcount > 0:
            # Check if it was an insert or update by checking if created_at ≈ updated_at
            cur.execute(
                "SELECT (created_at = updated_at) AS is_new FROM ground_state.genre_dimensions "
                "WHERE genre_id = %s",
                (genre_id,),
            )
            row = cur.fetchone()
            return ("inserted" if row and row["is_new"] else "updated"), ""
        return "skipped", ""


def _upsert_primitive_row(
    conn: psycopg.Connection,
    table: str,
    genre_id: str,
    cluster_id: str | None,
    entity_slug: str,
    name: str,
    payload: dict,
    h: str,
    extra_cols: dict,
    dry_run: bool = False,
) -> tuple[str, str]:
    """Upsert one row in a primitive table. Returns ('inserted'|'updated'|'skipped', '')."""
    if dry_run:
        return "skipped", ""

    # Build column list from extra_cols
    extra_names = list(extra_cols.keys())
    extra_vals = [extra_cols[k] for k in extra_names]

    base_cols = ["genre_id", "cluster_id", "entity_slug", "name", "payload", "source_hash"]
    base_vals = [genre_id, cluster_id, entity_slug, name, json.dumps(payload), h]

    all_cols = base_cols + extra_names
    all_vals = base_vals + extra_vals

    col_list = ", ".join(all_cols)
    # payload needs ::jsonb cast
    placeholders_casted = ", ".join(
        ["%s::jsonb" if c in ("payload",) else "%s" for c in all_cols]
    )

    update_sets = ", ".join(
        [
            f"{c} = EXCLUDED.{c}"
            for c in all_cols
            if c not in ("genre_id", "cluster_id", "entity_slug")
        ]
        + ["updated_at = now()"]
    )

    sql = f"""
        INSERT INTO ground_state.{table} ({col_list})
        VALUES ({placeholders_casted})
        ON CONFLICT (genre_id, entity_slug,
            COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID))
        DO UPDATE SET {update_sets}
        WHERE ground_state.{table}.source_hash != EXCLUDED.source_hash
    """

    with conn.cursor(row_factory=dict_row) as cur:
        cur.execute(sql, all_vals)
        if cur.rowcount > 0:
            cur.execute(
                f"SELECT (created_at = updated_at) AS is_new "
                f"FROM ground_state.{table} "
                f"WHERE genre_id = %s AND entity_slug = %s "
                f"AND COALESCE(cluster_id, '00000000-0000-0000-0000-000000000000'::UUID) = "
                f"COALESCE(%s::UUID, '00000000-0000-0000-0000-000000000000'::UUID)",
                (genre_id, entity_slug, cluster_id),
            )
            row = cur.fetchone()
            return ("inserted" if row and row["is_new"] else "updated"), ""
        return "skipped", ""


def load_primitive_type(
    conn: psycopg.Connection,
    type_name: str,
    corpus_dir: Path,
    slug_map: dict[str, str],
    genre_filter: list[str] | None = None,
    dry_run: bool = False,
) -> LoadReport:
    """Load all corpus files for one primitive type.

    Resolves genre_slug and cluster_slug to UUIDs via slug_map.
    Uses source-hash drift detection for idempotent upserts.
    """
    report = LoadReport()
    table = TABLE_MAP.get(type_name)
    if not table:
        log.warning("Unknown type %s — skipping", type_name)
        report.errors += 1
        report.error_details.append(f"Unknown type: {type_name}")
        return report

    files = _corpus_files_for_type(type_name, corpus_dir, genre_filter)
    if not files:
        log.info("No files found for type %s", type_name)
        return report

    for file_path, genre_slug, cluster_slug in files:
        raw = file_path.read_bytes()
        h = _source_hash(raw)

        genre_id = slug_map.get(genre_slug) if genre_slug else None
        cluster_id = slug_map.get(cluster_slug) if cluster_slug else None

        if genre_slug and not genre_id:
            log.warning("Unknown genre slug %s in %s — skipping file", genre_slug, file_path)
            report.skipped += 1
            continue

        try:
            data = json.loads(raw)
        except json.JSONDecodeError as exc:
            log.error("JSON decode error in %s: %s", file_path, exc)
            report.errors += 1
            report.error_details.append(f"{file_path}: {exc}")
            continue

        if type_name == "genre-dimensions":
            if not genre_id:
                report.skipped += 1
                continue
            try:
                outcome, _ = _upsert_genre_dimensions_row(
                    conn, genre_id, data, h, dry_run=dry_run
                )
                _tally(report, outcome)
            except Exception as exc:
                log.error("Error upserting genre_dimensions for %s: %s", genre_slug, exc)
                report.errors += 1
                report.error_details.append(f"genre_dimensions/{genre_slug}: {exc}")
            continue

        if not isinstance(data, list):
            log.warning("Expected list in %s, got %s — skipping", file_path, type(data).__name__)
            report.skipped += 1
            continue

        for entity in data:
            if not isinstance(entity, dict):
                report.skipped += 1
                continue

            slug = _entity_slug(entity)
            if not slug:
                log.warning("Cannot derive slug for entity in %s: %s", file_path, entity)
                report.skipped += 1
                continue

            name = _entity_name(type_name, entity)

            # For cluster files, require cluster_id; for genre files, require genre_id
            if cluster_slug and not cluster_id:
                report.skipped += 1
                continue

            # genre_id required for all primitive tables
            effective_genre_id = genre_id
            if not effective_genre_id and cluster_slug:
                # Cluster-only entities still need a genre_id for the FK.
                # Cluster files encode a list of genre variants — we use the cluster's
                # first member genre as a synthetic genre anchor when genre_slug is absent.
                # Better: skip until we have a proper genre_id source.
                report.skipped += 1
                continue

            try:
                extra = _promoted_columns(type_name, entity)
                outcome, _ = _upsert_primitive_row(
                    conn,
                    table,
                    effective_genre_id,
                    cluster_id,
                    slug,
                    name,
                    entity,
                    h,
                    extra,
                    dry_run=dry_run,
                )
                _tally(report, outcome)
            except Exception as exc:
                source = genre_slug or cluster_slug
                log.error("Error upserting %s/%s entity %s: %s", type_name, source, slug, exc)
                report.errors += 1
                report.error_details.append(f"{type_name}/{source}/{slug}: {exc}")

        if not dry_run:
            conn.commit()

    return report


def _tally(report: LoadReport, outcome: str) -> None:
    if outcome == "inserted":
        report.inserted += 1
    elif outcome == "updated":
        report.updated += 1
    else:
        report.skipped += 1


# ---------------------------------------------------------------------------
# Main orchestration
# ---------------------------------------------------------------------------


def load_ground_state(
    conn: psycopg.Connection,
    corpus_dir: Path,
    types: list[str] | None = None,
    genre_filter: list[str] | None = None,
    refs_only: bool = False,
    skip_prune: bool = True,
    dry_run: bool = False,
) -> LoadReport:
    """Orchestrate the full ground-state load.

    Phase 1: reference entities (genres, clusters, state_variables, dimensions)
    Phase 2: primitive types (one or all, filtered by genre if given)
    """
    conn.autocommit = False

    slug_map = load_reference_entities(conn, corpus_dir, dry_run=dry_run)

    if refs_only:
        log.info("refs-only mode — skipping primitive type load")
        return LoadReport()

    active_types = types if types else ALL_PRIMITIVE_TYPES
    total = LoadReport()

    for type_name in active_types:
        log.info("Loading primitive type: %s", type_name)
        report = load_primitive_type(
            conn,
            type_name,
            corpus_dir,
            slug_map,
            genre_filter=genre_filter,
            dry_run=dry_run,
        )
        log.info("  %s: %s", type_name, report.summary())
        total = total + report

    log.info("Load complete. Total: %s", total.summary())
    return total

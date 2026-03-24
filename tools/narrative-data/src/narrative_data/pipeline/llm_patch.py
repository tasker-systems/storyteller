# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""LLM-assisted patch fills for sparse fields in structured narrative data.

Provides targeted LLM extraction for fields that require semantic understanding:

- ``extract_valence``: classifies the relational valence of a dynamics entity
- ``extract_currencies``: extracts relational currencies mentioned in source markdown
- ``extract_scale_manifestations``: fills orbital/arc/scene manifestation descriptions
- ``fill_all_llm_patch``: orchestrates LLM patch fills across the corpus

Currently only handles the ``dynamics`` type.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from narrative_data.ollama import OllamaClient

# Valid valence values from the dynamics schema ValenceLiteral
_VALID_VALENCE: frozenset[str] = frozenset(
    ["sacred", "hostile", "nurturing", "indifferent", "parasitic", "transactional", "protective"]
)

_CURRENCIES_SCHEMA: dict = {
    "type": "object",
    "properties": {
        "currencies": {
            "type": "array",
            "items": {"type": "string"},
        }
    },
    "required": ["currencies"],
}

_SCALE_MANIFESTATIONS_SCHEMA: dict = {
    "type": "object",
    "properties": {
        "orbital": {"type": ["string", "null"]},
        "arc": {"type": ["string", "null"]},
        "scene": {"type": ["string", "null"]},
    },
    "required": ["orbital", "arc", "scene"],
}


def _find_entity_section(md_content: str, entity_name: str) -> str:
    """Extract the markdown section nearest to a given entity name.

    Re-uses the same logic as ``postprocess._find_entity_section`` but kept
    local so this module has no cross-dependency on postprocess internals.

    Args:
        md_content: Full source markdown text.
        entity_name: The ``canonical_name`` of the entity to locate.

    Returns:
        The relevant markdown section, or an empty string if not found.
    """
    escaped = re.escape(entity_name)
    pattern = re.compile(rf"^.*{escaped}.*$", re.IGNORECASE | re.MULTILINE)
    m = pattern.search(md_content)
    if not m:
        return ""

    start = m.start()
    line = m.group(0).lstrip()
    heading_match = re.match(r"^(#{1,6})\s", line)
    if heading_match:
        level = len(heading_match.group(1))
        end_pattern = re.compile(rf"^#{{1,{level}}}\s", re.MULTILINE)
        end_m = end_pattern.search(md_content, m.end())
        end = end_m.start() if end_m else len(md_content)
    else:
        lines = md_content[start:].split("\n")
        section_lines = lines[:20]
        end = start + len("\n".join(section_lines))

    return md_content[start:end]


def extract_valence(entity: dict, md_content: str, client: OllamaClient) -> dict:
    """Fill the ``valence`` field of a dynamics entity using LLM classification.

    Skips silently if ``valence`` is already a non-None, non-empty value.

    Args:
        entity: A dynamics entity dict (will not be mutated).
        md_content: Full source markdown for the genre file.
        client: An ``OllamaClient`` instance for LLM calls.

    Returns:
        A new dict with ``valence`` populated (or unchanged if already set or
        LLM response is not a valid valence value).
    """
    from narrative_data.config import STRUCTURING_MODEL

    result = dict(entity)
    existing = result.get("valence")
    if existing is not None and (not isinstance(existing, str) or existing.strip() != ""):
        return result

    entity_name = str(result.get("canonical_name", ""))
    edge_type = str(result.get("edge_type", ""))
    role_slots = result.get("role_slots", [])
    role_descriptions = ", ".join(
        str(slot.get("role", "")) for slot in role_slots if isinstance(slot, dict)
    )

    section = _find_entity_section(md_content, entity_name) if entity_name else ""
    section_snippet = section[:600] if section else "(no source section found)"

    valid_values = ", ".join(sorted(_VALID_VALENCE))
    prompt = (
        f"You are classifying the relational valence of a narrative dynamic.\n\n"
        f"Dynamic name: {entity_name}\n"
        f"Edge type: {edge_type}\n"
        f"Role slots: {role_descriptions}\n\n"
        f"Source description:\n{section_snippet}\n\n"
        f"Choose exactly one valence from this list: {valid_values}\n\n"
        f"Respond with only the single valence word, nothing else."
    )

    raw = client.generate(model=STRUCTURING_MODEL, prompt=prompt, temperature=0.1)
    value = raw.strip().lower().split()[0] if raw.strip() else ""

    if value in _VALID_VALENCE:
        result = dict(result)
        result["valence"] = value

    return result


def extract_currencies(entity: dict, md_content: str, client: OllamaClient) -> dict:
    """Fill the ``currencies`` field (list[str]) from source markdown using structured LLM.

    Skips silently if ``currencies`` is already a non-empty list.

    Args:
        entity: A dynamics entity dict (will not be mutated).
        md_content: Full source markdown for the genre file.
        client: An ``OllamaClient`` instance for LLM calls.

    Returns:
        A new dict with ``currencies`` populated (or unchanged if already set).
    """
    from narrative_data.config import STRUCTURING_MODEL

    result = dict(entity)
    existing = result.get("currencies")
    if isinstance(existing, list) and len(existing) > 0:
        return result

    entity_name = str(result.get("canonical_name", ""))
    edge_type = str(result.get("edge_type", ""))

    section = _find_entity_section(md_content, entity_name) if entity_name else ""
    section_snippet = section[:800] if section else "(no source section found)"

    prompt = (
        f"You are extracting relational currencies from a narrative dynamic description.\n\n"
        f"Relational currencies are things exchanged, withheld, or leveraged between characters "
        f"in this dynamic — for example: loyalty, secrets, safety, obligation, shame, love, debt.\n\n"
        f"Dynamic name: {entity_name}\n"
        f"Edge type: {edge_type}\n\n"
        f"Source description:\n{section_snippet}\n\n"
        f'Return a JSON object with a single key "currencies" containing an array of short '
        f"strings (2–6 words each). Return 2–6 currencies. If none are apparent, return an "
        f"empty array."
    )

    try:
        parsed = client.generate_structured(
            model=STRUCTURING_MODEL,
            prompt=prompt,
            schema=_CURRENCIES_SCHEMA,
            temperature=0.1,
        )
        currencies = parsed.get("currencies", [])
        if isinstance(currencies, list):
            cleaned = [str(c).strip() for c in currencies if str(c).strip()]
            if cleaned:
                result = dict(result)
                result["currencies"] = cleaned
    except (KeyError, TypeError, ValueError, json.JSONDecodeError):
        pass

    return result


def extract_scale_manifestations(entity: dict, md_content: str, client: OllamaClient) -> dict:
    """Fill the ``scale_manifestations`` nested object using structured LLM extraction.

    Fills the ``orbital``, ``arc``, and ``scene`` sub-fields.  Skips silently
    if ``scale_manifestations`` is already a dict with at least one non-None value.

    Args:
        entity: A dynamics entity dict (will not be mutated).
        md_content: Full source markdown for the genre file.
        client: An ``OllamaClient`` instance for LLM calls.

    Returns:
        A new dict with ``scale_manifestations`` populated (or unchanged if already set).
    """
    from narrative_data.config import STRUCTURING_MODEL

    result = dict(entity)
    existing = result.get("scale_manifestations")
    # Skip if any sub-field is already populated
    if isinstance(existing, dict) and any(v is not None for v in existing.values()):
        return result

    entity_name = str(result.get("canonical_name", ""))
    edge_type = str(result.get("edge_type", ""))

    section = _find_entity_section(md_content, entity_name) if entity_name else ""
    section_snippet = section[:800] if section else "(no source section found)"

    prompt = (
        f"You are extracting scale manifestation descriptions for a narrative dynamic.\n\n"
        f"Scale manifestations describe how this dynamic appears at each narrative timescale:\n"
        f"- orbital: across the whole story (character arcs, overarching themes)\n"
        f"- arc: across a story arc or act (escalating tensions, turning points)\n"
        f"- scene: within a single scene (immediate tension, action, dialogue)\n\n"
        f"Dynamic name: {entity_name}\n"
        f"Edge type: {edge_type}\n\n"
        f"Source description:\n{section_snippet}\n\n"
        f"Return a JSON object with keys 'orbital', 'arc', and 'scene'. "
        f"Each value should be a short descriptive string (1–2 sentences) or null if the scale "
        f"is not relevant to this dynamic."
    )

    try:
        parsed = client.generate_structured(
            model=STRUCTURING_MODEL,
            prompt=prompt,
            schema=_SCALE_MANIFESTATIONS_SCHEMA,
            temperature=0.1,
        )
        orbital = parsed.get("orbital")
        arc = parsed.get("arc")
        scene = parsed.get("scene")
        # Only update if at least one field has a value
        if any(v is not None for v in (orbital, arc, scene)):
            result = dict(result)
            result["scale_manifestations"] = {
                "orbital": orbital,
                "arc": arc,
                "scene": scene,
            }
    except (KeyError, TypeError, ValueError, json.JSONDecodeError):
        pass

    return result


def fill_all_llm_patch(
    corpus_dir: Path,
    client: OllamaClient,
    types: list[str] | None,
    genres: list[str] | None,
    dry_run: bool = False,
) -> dict[str, dict]:
    """Walk the corpus and apply LLM patch fill functions per type.

    Currently handles only ``dynamics`` type.  Reads source markdown,
    applies ``extract_valence``, ``extract_currencies``, and
    ``extract_scale_manifestations`` to each entity, then writes back if
    any field changed (unless *dry_run*).

    Args:
        corpus_dir: The ``narrative-data/`` root directory.
        client: An ``OllamaClient`` instance for LLM calls.
        types: List of type slugs to process.  If None, processes all
               supported LLM patch types (``dynamics`` only for now).
        genres: List of genre slugs to restrict processing to.  If None,
                all found files are processed.
        dry_run: When True, computes changes but does not write any files.

    Returns:
        Dict mapping type slug → summary dict with keys:
        ``files_processed``, ``entities_updated``, ``entities_skipped``.
    """
    supported_patch_types = ["dynamics"]
    type_list = types if types is not None else supported_patch_types

    summary: dict[str, dict] = {}

    for type_slug in type_list:
        if type_slug not in supported_patch_types:
            continue

        files_processed = 0
        entities_updated = 0
        entities_skipped = 0

        type_dir = corpus_dir / "discovery" / type_slug
        if not type_dir.exists():
            summary[type_slug] = {
                "files_processed": 0,
                "entities_updated": 0,
                "entities_skipped": 0,
            }
            continue

        for json_path in sorted(type_dir.glob("*.json")):
            if json_path.name in ("manifest.json",):
                continue
            if json_path.name.endswith(".errors.json"):
                continue
            genre_slug = json_path.stem
            if genres and genre_slug not in genres:
                continue

            try:
                raw = json.loads(json_path.read_text())
            except (json.JSONDecodeError, OSError):
                continue

            if not isinstance(raw, list):
                continue

            entities: list[dict] = [e for e in raw if isinstance(e, dict)]
            files_processed += 1

            # Load companion markdown for context
            md_content: str = ""
            md_path = type_dir / f"{genre_slug}.md"
            if md_path.exists():
                try:
                    md_content = md_path.read_text()
                except OSError:
                    md_content = ""

            updated_entities: list[dict] = []
            file_changed = False

            for entity in entities:
                filled = entity
                filled = extract_valence(filled, md_content, client)
                filled = extract_currencies(filled, md_content, client)
                filled = extract_scale_manifestations(filled, md_content, client)

                if filled != entity:
                    entities_updated += 1
                    file_changed = True
                else:
                    entities_skipped += 1

                updated_entities.append(filled)

            if file_changed and not dry_run:
                json_path.write_text(json.dumps(updated_entities, indent=2))

        summary[type_slug] = {
            "files_processed": files_processed,
            "entities_updated": entities_updated,
            "entities_skipped": entities_skipped,
        }

    return summary

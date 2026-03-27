# Tome Phase 2: Mutual Production Graph — Implementation Plan

> **For agentic workers:** This plan mixes Python tooling (Tasks 1-4) with collaborative elicitation (Tasks 5-7) and validation (Tasks 8-10). Tasks 1-4 are code; Tasks 5-7 are LLM-driven pipeline execution; Tasks 8-10 are analysis and export. Use superpowers:executing-plans for inline execution with review checkpoints.

**Goal:** Produce a curated mutual production graph mapping relationships between 51 Tome axes, with pairwise edges, compound edges, and cluster annotations.

**Architecture:** Python script generates exhaustive combinatorial pairs as per-domain-pair markdown files. qwen3.5:35b annotates each chunk. Human curates annotations. Export script produces canonical `edges.json`.

**Tech Stack:** Python 3.11+, Click CLI, httpx (Ollama), Pydantic, existing narrative-data pipeline patterns (manifest, prompt builder, two-stage elicitation).

**Spec:** `docs/superpowers/specs/2026-03-26-tome-phase-2-mutual-production-graph-design.md`

---

### Task 1: Schema Formalization — Update Domain Files

**Files:**
- Modify: `storyteller-data/narrative-data/tome/domains/material-conditions.json`
- Modify: `storyteller-data/narrative-data/tome/domains/economic-forms.json`
- Modify: `storyteller-data/narrative-data/tome/domains/political-structures.json`
- Modify: `storyteller-data/narrative-data/tome/domains/social-forms.json`
- Modify: `storyteller-data/narrative-data/tome/domains/history-as-force.json`
- Modify: `storyteller-data/narrative-data/tome/domains/aesthetic-cultural-forms.json`

- [ ] **Step 1: Update axes to set type where identified**

Change `axis_type` from `"categorical"` to `"set"` for these axes:
- `production-mode` (economic-forms)
- `labor-organization` (economic-forms)
- `trauma-transmission-mode` (history-as-force)
- `aesthetic-register` (aesthetic-cultural-forms)
- `knowledge-system-structure` (social-forms)
- `historical-memory-depth` (history-as-force)

Use `jq` or Python to make the changes. For each axis, change only the `axis_type` field — values lists remain the same.

- [ ] **Step 2: Add dual_mode field to stated/operative axes**

Add the `dual_mode` schema field to these axes (value is null/empty for now — world positions fill it):
- `authority-legitimation` (political-structures)
- `social-stratification` (social-forms)
- `relationship-to-past` (history-as-force)
- `social-mobility` (social-forms)

For each, add after `axis_type`:
```json
"dual_mode": {
  "type": "stated_operative",
  "description": "This axis carries both a stated value (how the society presents itself) and an operative value (how it actually functions). The gap between them is narratively productive."
}
```

- [ ] **Step 3: Verify all domain files are valid JSON**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
for f in narrative-data/tome/domains/*.json; do python3 -m json.tool "$f" > /dev/null && echo "OK: $f" || echo "FAIL: $f"; done
```

Expected: all 6 files OK.

- [ ] **Step 4: Commit schema formalization**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/domains/*.json
git commit -m "feat: formalize set type and stated/operative pair in Tome axis schema"
```

---

### Task 2: Combinatorial Generation Script

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/__init__.py`
- Create: `tools/narrative-data/src/narrative_data/tome/generate_edges.py`
- Create: `tools/narrative-data/prompts/tome/edge-annotation.md`

- [ ] **Step 1: Create the tome module**

```bash
mkdir -p /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data/src/narrative_data/tome
```

```python
# tools/narrative-data/src/narrative_data/tome/__init__.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSE file in the project root for licensing details.
```

- [ ] **Step 2: Write the generation script**

```python
# tools/narrative-data/src/narrative_data/tome/generate_edges.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSE file in the project root for licensing details.
"""Generate exhaustive edge combinatorics for Tome mutual production graph.

Reads domain JSON files, generates per-domain-pair markdown files with
axis descriptions and pair tables, and creates a manifest for pipeline tracking.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from itertools import product
from pathlib import Path

from narrative_data.config import resolve_output_path


DOMAIN_SLUGS = [
    "material-conditions",
    "economic-forms",
    "political-structures",
    "social-forms",
    "history-as-force",
    "aesthetic-cultural-forms",
]

EDGE_TYPES_DESCRIPTION = """## Edge Types

| Type | Meaning | Agent Interpretation |
|------|---------|---------------------|
| **produces** | A gives rise to B | If A is present, B is expected |
| **constrains** | A limits what B can be | Some B values are implausible given A |
| **enables** | A makes B possible (not required) | If A absent, B needs alternative explanation |
| **transforms** | A changes B's character over time | A's presence shifts B dynamically |
| **none** | No meaningful relationship | Skip — provide brief rationale |

## Compound Edges

If two or more source axes *jointly* produce an effect on the target that neither
captures alone, note this as a compound edge at the end of the section:

```
COMPOUND: [axis-a, axis-b] -> target-axis | type | description
```
"""


def load_domain(domains_dir: Path, slug: str) -> dict:
    """Load a domain JSON file and return the parsed dict."""
    path = domains_dir / f"{slug}.json"
    with open(path) as f:
        return json.load(f)


def format_axis_context(axis: dict) -> str:
    """Format an axis as a concise context block for the prompt."""
    values = axis.get("values", "")
    if isinstance(values, list):
        values_str = ", ".join(values)
    elif isinstance(values, dict):
        if "sub_dimensions" in values:
            values_str = f"profile: {', '.join(values['sub_dimensions'])}"
        elif "low_label" in values:
            values_str = f"bipolar: {values['low_label']} ↔ {values['high_label']}"
        else:
            values_str = json.dumps(values)
    else:
        values_str = str(values)

    return (
        f"- **{axis['slug']}** ({axis['axis_type']}): "
        f"{axis['description'][:200]} "
        f"[{values_str}]"
    )


def extract_seed_edges(domains: dict[str, dict]) -> dict[tuple[str, str], dict]:
    """Extract seed edges from Phase 1 _commentary and _suggestions fields.

    Returns a dict keyed by (from_axis, to_axis) with edge info where identifiable.
    """
    seeds: dict[tuple[str, str], dict] = {}
    # Seed edges are identified by "Phase 2:" references in commentary
    # This is a best-effort extraction — not all references name specific target axes
    # The pre-population is partial; most will still be blank
    return seeds


def generate_domain_pair_file(
    source_domain: dict,
    target_domain: dict,
    output_dir: Path,
    seed_edges: dict[tuple[str, str], dict],
) -> dict:
    """Generate a markdown file for one domain pair.

    Returns manifest entry dict.
    """
    source_slug = source_domain["domain"]["slug"]
    target_slug = target_domain["domain"]["slug"]
    source_axes = source_domain["axes"]
    target_axes = target_domain["axes"]

    is_within = source_slug == target_slug

    # Build the markdown
    lines: list[str] = []
    lines.append(f"# Edges: {source_domain['domain']['name']}")
    if not is_within:
        lines.append(f" → {target_domain['domain']['name']}")
    lines.append("")
    lines.append(f"Source domain: **{source_domain['domain']['name']}**")
    lines.append(f"> {source_domain['domain']['description']}")
    lines.append("")
    if not is_within:
        lines.append(f"Target domain: **{target_domain['domain']['name']}**")
        lines.append(f"> {target_domain['domain']['description']}")
        lines.append("")

    # Source axes context
    lines.append("## Source Axes")
    lines.append("")
    for axis in source_axes:
        lines.append(format_axis_context(axis))
    lines.append("")

    # Target axes context (skip if within-domain)
    if not is_within:
        lines.append("## Target Axes")
        lines.append("")
        for axis in target_axes:
            lines.append(format_axis_context(axis))
        lines.append("")

    # Edge type definitions
    lines.append(EDGE_TYPES_DESCRIPTION)
    lines.append("")

    # Pair table
    lines.append("## Edge Assessment")
    lines.append("")
    lines.append("| From | To | Type | Weight | Description |")
    lines.append("|------|----|------|--------|-------------|")

    pair_count = 0
    for source_axis in source_axes:
        for target_axis in target_axes:
            if source_axis["slug"] == target_axis["slug"]:
                continue  # skip self-pairs
            seed = seed_edges.get((source_axis["slug"], target_axis["slug"]))
            if seed:
                lines.append(
                    f"| {source_axis['slug']} | {target_axis['slug']} "
                    f"| {seed.get('edge_type', '')} | {seed.get('weight', '')} "
                    f"| {seed.get('description', '*[seed]*')} |"
                )
            else:
                lines.append(
                    f"| {source_axis['slug']} | {target_axis['slug']} | | | |"
                )
            pair_count += 1

    lines.append("")
    lines.append("## Compound Edges")
    lines.append("")
    lines.append("*(Note any compound edges discovered during assessment)*")
    lines.append("")
    lines.append("## Cluster Annotation")
    lines.append("")
    lines.append("*(Aggregate insight about how axes in this region of the graph interact)*")
    lines.append("")

    # Write file
    pair_dir = output_dir / source_slug
    pair_dir.mkdir(parents=True, exist_ok=True)

    if is_within:
        filename = "within.md"
    else:
        filename = f"{target_slug}.md"

    filepath = pair_dir / filename
    filepath.write_text("\n".join(lines))

    return {
        "source_domain": source_slug,
        "target_domain": target_slug if not is_within else source_slug,
        "within_domain": is_within,
        "pair_count": pair_count,
        "status": "pending",
        "filepath": str(filepath),
    }


def generate_all(data_path: Path) -> None:
    """Generate all 21 domain-pair edge files and manifest."""
    domains_dir = data_path / "narrative-data" / "tome" / "domains"
    edges_dir = data_path / "narrative-data" / "tome" / "edges"
    edges_dir.mkdir(parents=True, exist_ok=True)

    # Load all domains
    domains = {}
    for slug in DOMAIN_SLUGS:
        domains[slug] = load_domain(domains_dir, slug)

    # Extract seed edges (best-effort from commentary)
    seed_edges = extract_seed_edges(domains)

    # Generate within-domain files (6)
    manifest_entries = {}
    for slug in DOMAIN_SLUGS:
        key = f"{slug}/within"
        entry = generate_domain_pair_file(
            domains[slug], domains[slug], edges_dir, seed_edges
        )
        manifest_entries[key] = entry

    # Generate cross-domain files (15)
    for i, source_slug in enumerate(DOMAIN_SLUGS):
        for target_slug in DOMAIN_SLUGS[i + 1 :]:
            key = f"{source_slug}/{target_slug}"
            entry = generate_domain_pair_file(
                domains[source_slug], domains[target_slug], edges_dir, seed_edges
            )
            manifest_entries[key] = entry

    # Write manifest
    manifest = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "total_pairs": sum(e["pair_count"] for e in manifest_entries.values()),
        "total_chunks": len(manifest_entries),
        "entries": manifest_entries,
    }
    manifest_path = edges_dir / "manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    print(f"Generated {len(manifest_entries)} edge files")
    print(f"Total pairs: {manifest['total_pairs']}")
    print(f"Output: {edges_dir}")
    print(f"Manifest: {manifest_path}")
```

- [ ] **Step 3: Write the edge annotation prompt template**

```bash
mkdir -p /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data/prompts/tome
```

Create `tools/narrative-data/prompts/tome/edge-annotation.md`:

```markdown
You are analyzing relationships between world-building axes in a narrative engine.
You are given two sets of axes from different domains (or the same domain). Your task
is to assess whether each pair of axes has a meaningful mutual production relationship.

For each pair in the table below, determine:
1. **Type**: produces, constrains, enables, transforms, or none
2. **Weight**: 0.0-1.0 (how strongly should an agent consider this relationship)
3. **Description**: One sentence explaining the relationship

Guidelines:
- **produces**: A gives rise to B. If A is present, B is expected.
- **constrains**: A limits what B can be. Some B values are implausible given A.
- **enables**: A makes B possible but doesn't require it. If A absent, B needs alternative explanation.
- **transforms**: A changes B's character over time. A's presence shifts B dynamically.
- **none**: No meaningful relationship between these axes.

Be selective — most pairs will be `none`. Only mark a relationship if it would
genuinely help an agent reason about world coherence. A weak or speculative
relationship is worse than `none`.

If you notice that two source axes *jointly* produce an effect on a target that
neither captures alone, note it as a COMPOUND edge at the end.

{axis_context}

Fill in the Type, Weight, and Description columns for each row:

{pair_table}
```

- [ ] **Step 4: Add CLI command**

Add to `tools/narrative-data/src/narrative_data/cli.py`:

```python
@cli.group()
def tome() -> None:
    """Tome world-building axis and edge operations."""


@tome.command("generate-edges")
def tome_generate_edges() -> None:
    """Generate exhaustive edge combinatorics for mutual production graph."""
    from narrative_data.tome.generate_edges import generate_all

    data_path = resolve_data_path()
    generate_all(data_path)
```

- [ ] **Step 5: Test generation**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
uv run narrative-data tome generate-edges
```

Expected: 21 markdown files in `storyteller-data/narrative-data/tome/edges/`, manifest.json created.

- [ ] **Step 6: Commit generation script**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/ tools/narrative-data/prompts/tome/
git commit -m "feat: add Tome edge combinatorial generation script and prompt template"
```

---

### Task 3: LLM Annotation Script

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/annotate_edges.py`

- [ ] **Step 1: Write the annotation script**

```python
# tools/narrative-data/src/narrative_data/tome/annotate_edges.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSE file in the project root for licensing details.
"""Annotate edge markdown files using qwen3.5:35b via Ollama.

Reads each pending domain-pair markdown file, sends the axis context
and pair table to the model, and writes the annotated result back.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from narrative_data.config import ELICITATION_MODEL, resolve_data_path
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.invalidation import (
    archive_existing,
    compute_prompt_hash,
    load_manifest,
    save_manifest,
)


PROMPT_TEMPLATE_PATH = (
    Path(__file__).parents[4] / "prompts" / "tome" / "edge-annotation.md"
)


def build_annotation_prompt(markdown_content: str) -> str:
    """Build the annotation prompt from template and markdown content.

    Extracts the axis context sections and pair table from the markdown,
    substitutes them into the prompt template.
    """
    template = PROMPT_TEMPLATE_PATH.read_text()

    # Split markdown into context and table sections
    # The markdown has: axis descriptions, edge type definitions, then the table
    sections = markdown_content.split("## Edge Assessment")

    axis_context = sections[0] if sections else ""
    pair_table = sections[1].split("## Compound Edges")[0] if len(sections) > 1 else ""

    return template.replace("{axis_context}", axis_context).replace(
        "{pair_table}", pair_table
    )


def annotate_chunk(
    client: OllamaClient,
    manifest_key: str,
    entry: dict,
    model: str = ELICITATION_MODEL,
) -> dict:
    """Annotate a single domain-pair chunk.

    Returns updated manifest entry.
    """
    filepath = Path(entry["filepath"])
    markdown_content = filepath.read_text()

    prompt = build_annotation_prompt(markdown_content)
    prompt_hash = compute_prompt_hash(prompt)

    # Generate annotation
    response = client.generate(
        model=model,
        prompt=prompt,
        temperature=0.3,  # Lower temperature for analytical work
        timeout=600.0,
    )

    # Archive existing and write annotated version
    archive_existing(filepath)

    # Reconstruct the file: keep the header/context, replace the table with
    # the model's annotated version, keep compound/cluster sections
    header = markdown_content.split("## Edge Assessment")[0]
    annotated = header + "## Edge Assessment\n\n" + response

    filepath.write_text(annotated)

    return {
        **entry,
        "status": "annotated",
        "prompt_hash": prompt_hash,
        "annotated_at": datetime.now(timezone.utc).isoformat(),
        "model": model,
    }


def annotate_all(
    data_path: Path,
    chunks: list[str] | None = None,
    model: str = ELICITATION_MODEL,
    force: bool = False,
) -> None:
    """Annotate all pending chunks (or specified chunks)."""
    edges_dir = data_path / "narrative-data" / "tome" / "edges"
    manifest_path = edges_dir / "manifest.json"
    manifest = load_manifest(manifest_path)

    client = OllamaClient()

    entries = manifest.get("entries", {})
    target_keys = chunks or [
        k for k, v in entries.items() if v.get("status") == "pending"
    ]

    for key in target_keys:
        entry = entries.get(key)
        if not entry:
            print(f"Unknown chunk: {key}")
            continue

        if entry.get("status") != "pending" and not force:
            print(f"Skipping {key} (status: {entry['status']})")
            continue

        print(f"Annotating {key} ({entry['pair_count']} pairs)...")
        updated = annotate_chunk(client, key, entry, model=model)
        entries[key] = updated
        save_manifest(manifest_path, manifest)
        print(f"  Done: {key}")

    print(f"\nAnnotated {len(target_keys)} chunks")
```

- [ ] **Step 2: Add CLI command**

Add to the `tome` group in `cli.py`:

```python
@tome.command("annotate-edges")
@click.option("--chunks", default=None, help="Comma-separated chunk keys to annotate")
@click.option("--force", is_flag=True, default=False, help="Re-annotate even if already done")
def tome_annotate_edges(chunks: str | None, force: bool) -> None:
    """Annotate edge files using qwen3.5:35b."""
    from narrative_data.tome.annotate_edges import annotate_all

    data_path = resolve_data_path()
    chunk_list = _parse_list(chunks) if chunks else None
    annotate_all(data_path, chunks=chunk_list, force=force)
```

- [ ] **Step 3: Commit annotation script**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/annotate_edges.py
git commit -m "feat: add Tome edge LLM annotation script"
```

---

### Task 4: Export Script

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/export_edges.py`

- [ ] **Step 1: Write the export script**

```python
# tools/narrative-data/src/narrative_data/tome/export_edges.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSE file in the project root for licensing details.
"""Export curated edge markdown files to canonical edges.json.

Reads all curated domain-pair markdown files, parses the annotated tables,
and produces edges.json (meaningful relationships) and edges-rejected.json
(pairs marked none with rationale).
"""

from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from pathlib import Path

from narrative_data.config import resolve_data_path
from narrative_data.pipeline.invalidation import load_manifest


def parse_edge_table(content: str) -> tuple[list[dict], list[dict]]:
    """Parse a markdown edge table into edges and rejected pairs.

    Returns (edges, rejected) where each is a list of dicts.
    """
    edges = []
    rejected = []

    # Find the table section
    table_match = re.search(
        r"\| From \| To \| Type \| Weight \| Description \|.*?\n\|[-|]+\|\n(.*?)(?=\n##|\Z)",
        content,
        re.DOTALL,
    )
    if not table_match:
        return edges, rejected

    for line in table_match.group(1).strip().split("\n"):
        line = line.strip()
        if not line or not line.startswith("|"):
            continue

        cells = [c.strip() for c in line.split("|")[1:-1]]
        if len(cells) < 5:
            continue

        from_axis, to_axis, edge_type, weight_str, description = cells[:5]

        if not edge_type or edge_type.strip() == "":
            continue

        edge_type = edge_type.strip().lower()

        if edge_type == "none":
            rejected.append(
                {
                    "from_axis": from_axis.strip(),
                    "to_axis": to_axis.strip(),
                    "rationale": description.strip(),
                }
            )
            continue

        if edge_type not in ("produces", "constrains", "enables", "transforms"):
            continue

        try:
            weight = float(weight_str.strip()) if weight_str.strip() else 0.5
        except ValueError:
            weight = 0.5

        edges.append(
            {
                "from_axis": from_axis.strip(),
                "to_axis": to_axis.strip(),
                "edge_type": edge_type,
                "weight": weight,
                "description": description.strip(),
            }
        )

    return edges, rejected


def parse_compound_edges(content: str) -> list[dict]:
    """Parse compound edges from the Compound Edges section."""
    compounds = []

    compound_match = re.search(
        r"## Compound Edges\n\n(.*?)(?=\n##|\Z)", content, re.DOTALL
    )
    if not compound_match:
        return compounds

    for line in compound_match.group(1).strip().split("\n"):
        line = line.strip()
        if line.startswith("COMPOUND:"):
            # Parse: COMPOUND: [axis-a, axis-b] -> target | type | description
            match = re.match(
                r"COMPOUND:\s*\[([^\]]+)\]\s*->\s*(\S+)\s*\|\s*(\S+)\s*\|\s*(.+)",
                line,
            )
            if match:
                from_axes = [a.strip() for a in match.group(1).split(",")]
                compounds.append(
                    {
                        "type": "compound",
                        "from_axes": from_axes,
                        "to_axis": match.group(2).strip(),
                        "edge_type": match.group(3).strip(),
                        "description": match.group(4).strip(),
                    }
                )

    return compounds


def parse_cluster_annotation(content: str) -> str:
    """Parse the cluster annotation section."""
    match = re.search(
        r"## Cluster Annotation\n\n(.*?)(?=\n##|\Z)", content, re.DOTALL
    )
    if not match:
        return ""
    text = match.group(1).strip()
    if text.startswith("*("):
        return ""  # Still placeholder
    return text


def export_all(data_path: Path) -> None:
    """Export all curated edge files to edges.json and edges-rejected.json."""
    edges_dir = data_path / "narrative-data" / "tome" / "edges"
    manifest_path = edges_dir / "manifest.json"
    manifest = load_manifest(manifest_path)

    all_edges = []
    all_rejected = []
    all_compounds = []
    cluster_annotations = {}

    for key, entry in manifest.get("entries", {}).items():
        filepath = Path(entry["filepath"])
        if not filepath.exists():
            print(f"Warning: {filepath} not found, skipping")
            continue

        content = filepath.read_text()

        # Add domain info to each edge
        source_domain = entry["source_domain"]
        target_domain = entry["target_domain"]

        edges, rejected = parse_edge_table(content)
        for edge in edges:
            edge["from_domain"] = source_domain
            edge["to_domain"] = target_domain
            edge["provenance"] = "systematic"
        all_edges.extend(edges)

        for rej in rejected:
            rej["from_domain"] = source_domain
            rej["to_domain"] = target_domain
        all_rejected.extend(rejected)

        compounds = parse_compound_edges(content)
        for comp in compounds:
            comp["from_domains"] = [source_domain] * len(comp.get("from_axes", []))
            comp["to_domain"] = target_domain
            comp["provenance"] = "systematic"
        all_compounds.extend(compounds)

        annotation = parse_cluster_annotation(content)
        if annotation:
            cluster_annotations[key] = annotation

    # Write canonical edges.json
    output = {
        "exported_at": datetime.now(timezone.utc).isoformat(),
        "edge_count": len(all_edges),
        "compound_edge_count": len(all_compounds),
        "edges": all_edges,
        "compound_edges": all_compounds,
        "_cluster_annotations": cluster_annotations,
    }

    tome_dir = data_path / "narrative-data" / "tome"
    edges_path = tome_dir / "edges.json"
    with open(edges_path, "w") as f:
        json.dump(output, f, indent=2)

    # Write rejected pairs
    rejected_path = tome_dir / "edges-rejected.json"
    with open(rejected_path, "w") as f:
        json.dump(
            {
                "exported_at": datetime.now(timezone.utc).isoformat(),
                "rejected_count": len(all_rejected),
                "rejected": all_rejected,
            },
            f,
            indent=2,
        )

    print(f"Exported {len(all_edges)} edges + {len(all_compounds)} compound edges")
    print(f"Rejected {len(all_rejected)} pairs")
    print(f"Cluster annotations: {len(cluster_annotations)}")
    print(f"Output: {edges_path}")
```

- [ ] **Step 2: Add CLI command**

Add to the `tome` group in `cli.py`:

```python
@tome.command("export-edges")
def tome_export_edges() -> None:
    """Export curated edge files to canonical edges.json."""
    from narrative_data.tome.export_edges import export_all

    data_path = resolve_data_path()
    export_all(data_path)
```

- [ ] **Step 3: Commit export script**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/export_edges.py
git commit -m "feat: add Tome edge export script"
```

---

### Task 5: Generate Edge Files

- [ ] **Step 1: Run the generation script**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
uv run narrative-data tome generate-edges
```

Expected: 21 markdown files generated, manifest.json created.

- [ ] **Step 2: Verify output structure**

```bash
find /Users/petetaylor/projects/tasker-systems/storyteller-data/narrative-data/tome/edges -name "*.md" | sort
cat /Users/petetaylor/projects/tasker-systems/storyteller-data/narrative-data/tome/edges/manifest.json | python3 -m json.tool | head -20
```

Expected: 21 .md files organized by source domain, manifest shows all entries as `pending`.

- [ ] **Step 3: Commit generated files**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/edges/
git commit -m "feat: generate exhaustive Tome edge combinatorics (21 domain-pair files)"
```

---

### Task 6: LLM Annotation Pass

- [ ] **Step 1: Start Ollama with qwen3.5:35b**

```bash
ollama run qwen3.5:35b --keepalive 60m
```

- [ ] **Step 2: Run annotation on all chunks**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
uv run narrative-data tome annotate-edges
```

This will process all 21 chunks sequentially. Each chunk takes ~2-5 minutes.
Monitor manifest.json for progress.

- [ ] **Step 3: Verify annotations**

Spot-check 2-3 annotated files to ensure the model produced reasonable edge assessments.

```bash
head -60 /Users/petetaylor/projects/tasker-systems/storyteller-data/narrative-data/tome/edges/material-conditions/economic-forms.md
```

- [ ] **Step 4: Commit annotated files**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/edges/
git commit -m "feat: qwen3.5:35b annotation pass on all 21 Tome edge chunks"
```

---

### Task 7: Human Curation Pass

- [ ] **Step 1: Curate each annotated chunk**

Work through all 21 annotated markdown files. For each:
- Correct edge types and descriptions where the model's assessment is wrong
- Adjust weights
- Add compound edges the model missed
- Write cluster annotations
- Flag any discovered axes or axis refinements

Priority order (highest-value domain pairs first):
1. material-conditions/within
2. material-conditions/economic-forms
3. economic-forms/political-structures
4. material-conditions/political-structures
5. social-forms/within
6. political-structures/social-forms
7. history-as-force/within
8. All remaining cross-domain pairs

- [ ] **Step 2: Update manifest status**

After curating each chunk, update its manifest entry status to `"curated"`.

- [ ] **Step 3: Commit curated files**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/edges/
git commit -m "feat: human curation pass on Tome edge annotations"
```

---

### Task 8: Export Canonical Edge Graph

- [ ] **Step 1: Run the export script**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
uv run narrative-data tome export-edges
```

Expected: `edges.json` and `edges-rejected.json` created in `storyteller-data/narrative-data/tome/`.

- [ ] **Step 2: Verify edge counts and distribution**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
python3 -c "
import json
with open('narrative-data/tome/edges.json') as f:
    data = json.load(f)
print(f'Pairwise edges: {data[\"edge_count\"]}')
print(f'Compound edges: {data[\"compound_edge_count\"]}')
print(f'Cluster annotations: {len(data[\"_cluster_annotations\"])}')

# Distribution by edge type
from collections import Counter
types = Counter(e['edge_type'] for e in data['edges'])
print(f'By type: {dict(types)}')

# Distribution by domain pair
pairs = Counter(f\"{e['from_domain']} -> {e['to_domain']}\" for e in data['edges'])
for pair, count in pairs.most_common():
    print(f'  {pair}: {count}')
"
```

- [ ] **Step 3: Commit canonical graph**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/edges.json narrative-data/tome/edges-rejected.json
git commit -m "feat: export canonical Tome mutual production graph"
```

---

### Task 9: Coherence Testing

- [ ] **Step 1: McCallister's Barn trace**

Manually trace through `edges.json`: geography-climate(temperate) → production-mode → economic-volatility → exchange-and-obligation → land-tenure-system → legacy-visibility. Each step must follow an edge in the graph. Note any missing edges.

- [ ] **Step 2: Cyberpunk Augmentation Debt trace**

Trace: technological-ceiling(post-digital) + biological-plasticity(augmentable) → exchange-and-obligation(financialized-credit) → labor-organization(gig-precarious) → wealth-concentration(extreme) → social-mobility(stated:fluid, operative:frozen). Note any missing edges.

- [ ] **Step 3: Implausible World Detection test**

Check: geography-climate(arid-desert) + resource-profile(fisheries:abundant, timber:dominant). Query edges.json for constrains edges that are violated. Verify the graph surfaces the contradictions.

- [ ] **Step 4: Fix any gaps found**

If coherence tests reveal missing edges, add them to the appropriate domain-pair markdown file, re-export, and re-test.

- [ ] **Step 5: Commit any fixes**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git add narrative-data/tome/
git commit -m "fix: add edges discovered during coherence testing"
```

---

### Task 10: Programmatic Chain Generation (Stress Test)

**Files:**
- Create: `tools/narrative-data/src/narrative_data/tome/chain_generator.py`

- [ ] **Step 1: Write the chain generation script**

```python
# tools/narrative-data/src/narrative_data/tome/chain_generator.py
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSE file in the project root for licensing details.
"""Generate edge chains from the mutual production graph to validate coherence.

Starts from seed positions on material-conditions axes, follows edges
forward through domains, and generates world-position sketches at
varying chain depths.
"""

from __future__ import annotations

import json
import random
from pathlib import Path

from narrative_data.config import resolve_data_path


def load_graph(data_path: Path) -> dict:
    """Load the canonical edges.json."""
    path = data_path / "narrative-data" / "tome" / "edges.json"
    with open(path) as f:
        return json.load(f)


def load_axes(data_path: Path) -> dict[str, dict]:
    """Load all axes keyed by slug."""
    axes = {}
    domains_dir = data_path / "narrative-data" / "tome" / "domains"
    for f in domains_dir.glob("*.json"):
        with open(f) as fh:
            domain = json.load(fh)
            for axis in domain["axes"]:
                axes[axis["slug"]] = axis
    return axes


def get_outgoing_edges(graph: dict, from_axis: str) -> list[dict]:
    """Get all edges originating from a given axis."""
    return [e for e in graph["edges"] if e["from_axis"] == from_axis]


def random_position(axis: dict) -> str:
    """Generate a random position for an axis based on its type."""
    axis_type = axis["axis_type"]
    values = axis.get("values", [])

    if axis_type in ("categorical", "set") and isinstance(values, list):
        return random.choice(values)
    elif axis_type == "ordinal" and isinstance(values, list):
        return random.choice(values)
    elif axis_type in ("numeric", "bipolar"):
        return f"{random.random():.2f}"
    elif axis_type == "profile" and isinstance(values, dict):
        levels = values.get("levels", ["moderate"])
        subs = values.get("sub_dimensions", [])
        return ", ".join(f"{s}:{random.choice(levels)}" for s in subs[:3])
    return "unknown"


def generate_chain(
    graph: dict,
    axes: dict[str, dict],
    seed_axis: str,
    seed_position: str,
    max_depth: int = 5,
) -> list[dict]:
    """Follow edges from a seed axis, building a world-position chain.

    Returns a list of chain steps, each with axis, position, edge used.
    """
    chain = [{"axis": seed_axis, "position": seed_position, "via_edge": None}]
    visited = {seed_axis}

    current = seed_axis
    for _ in range(max_depth):
        outgoing = get_outgoing_edges(graph, current)
        # Filter to unvisited targets, prefer higher weight
        candidates = [e for e in outgoing if e["to_axis"] not in visited]
        if not candidates:
            break

        candidates.sort(key=lambda e: e.get("weight", 0.5), reverse=True)
        edge = candidates[0]
        target = edge["to_axis"]

        if target not in axes:
            break

        position = random_position(axes[target])
        chain.append(
            {
                "axis": target,
                "position": position,
                "via_edge": {
                    "type": edge["edge_type"],
                    "weight": edge.get("weight", 0.5),
                    "description": edge.get("description", ""),
                },
            }
        )

        visited.add(target)
        current = target

    return chain


def generate_world_sketches(
    data_path: Path,
    n_sketches: int = 10,
    max_depth: int = 5,
    seed_domain: str = "material-conditions",
) -> list[dict]:
    """Generate n world sketches by random chain traversal.

    Returns list of sketches, each with chain and metadata.
    """
    graph = load_graph(data_path)
    axes = load_axes(data_path)

    # Get seed axes from the specified domain
    seed_axes = [slug for slug, axis in axes.items() if axis.get("domain") == seed_domain]

    sketches = []
    for i in range(n_sketches):
        seed = random.choice(seed_axes)
        position = random_position(axes[seed])
        chain = generate_chain(graph, axes, seed, position, max_depth)

        sketches.append(
            {
                "sketch_id": i + 1,
                "seed_axis": seed,
                "seed_position": position,
                "chain_depth": len(chain),
                "domains_touched": list({axes[step["axis"]]["domain"] for step in chain if step["axis"] in axes}),
                "chain": chain,
            }
        )

    return sketches


def run_stress_test(data_path: Path, n_sketches: int = 20) -> None:
    """Run the chain generation stress test and print results."""
    sketches = generate_world_sketches(data_path, n_sketches=n_sketches)

    print(f"Generated {len(sketches)} world sketches\n")

    for sketch in sketches:
        print(f"--- Sketch {sketch['sketch_id']} ---")
        print(f"Seed: {sketch['seed_axis']} = {sketch['seed_position']}")
        print(f"Depth: {sketch['chain_depth']}, Domains: {', '.join(sketch['domains_touched'])}")
        for step in sketch["chain"]:
            if step["via_edge"]:
                edge = step["via_edge"]
                print(f"  → {step['axis']} = {step['position']}")
                print(f"    via {edge['type']} (w={edge['weight']}): {edge['description'][:80]}")
            else:
                print(f"  * {step['axis']} = {step['position']} (seed)")
        print()

    # Summary statistics
    depths = [s["chain_depth"] for s in sketches]
    domain_counts = [len(s["domains_touched"]) for s in sketches]
    print(f"Chain depth: min={min(depths)}, max={max(depths)}, avg={sum(depths)/len(depths):.1f}")
    print(f"Domains touched: min={min(domain_counts)}, max={max(domain_counts)}, avg={sum(domain_counts)/len(domain_counts):.1f}")

    # Flag short chains (may indicate disconnected graph regions)
    short = [s for s in sketches if s["chain_depth"] <= 2]
    if short:
        print(f"\nWarning: {len(short)} sketches had depth ≤ 2 (possible disconnected regions)")
        for s in short:
            print(f"  Sketch {s['sketch_id']}: seed={s['seed_axis']}")
```

- [ ] **Step 2: Add CLI command**

Add to the `tome` group in `cli.py`:

```python
@tome.command("stress-test")
@click.option("--count", default=20, help="Number of world sketches to generate")
def tome_stress_test(count: int) -> None:
    """Run chain generation stress test on the mutual production graph."""
    from narrative_data.tome.chain_generator import run_stress_test

    data_path = resolve_data_path()
    run_stress_test(data_path, n_sketches=count)
```

- [ ] **Step 3: Run the stress test**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller/tools/narrative-data
uv run narrative-data tome stress-test --count 20
```

Expected: 20 world sketches with varying chain depths and domain coverage. Flag any disconnected regions.

- [ ] **Step 4: Commit chain generator**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/tome/chain_generator.py
git commit -m "feat: add Tome edge chain generator for coherence stress testing"
```

---

### Task 11: Final Commit and Session Save

- [ ] **Step 1: Commit all CLI changes**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller
git add tools/narrative-data/src/narrative_data/cli.py
git commit -m "feat: add tome CLI commands (generate-edges, annotate-edges, export-edges, stress-test)"
```

- [ ] **Step 2: Push storyteller-data changes**

```bash
cd /Users/petetaylor/projects/tasker-systems/storyteller-data
git push
```

- [ ] **Step 3: Save session note**

```bash
temper session save "Tome Phase 2: Mutual Production Graph" \
  --ticket 2026-03-26-tome-phase-2-mutual-production-graph \
  --project storyteller
```

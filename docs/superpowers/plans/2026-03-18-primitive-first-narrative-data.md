# Primitive-First Narrative Data Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure the narrative-data pipeline from genre-centric derivative categories to primitive-first discovery with layered elaboration, adding pipeline control plane for observability and resumability.

**Architecture:** Four-phase pipeline (extract → synthesize → elicit → elaborate) with append-only JSONL event log tracking all actions. Discovery phases extract primitives from the existing 30-genre corpus, cluster synthesis deduplicates, Layer 0 elicits standalone descriptions, Layer 1 produces genre-specific elaborations. Existing genre region infrastructure is preserved.

**Tech Stack:** Python 3.11+, Click CLI, httpx (Ollama), Pydantic, Rich (terminal output). Models: qwen3.5:35b (elicitation), qwen2.5:7b-instruct (structuring). Tests: pytest with mocks.

**Spec:** `docs/superpowers/specs/2026-03-18-primitive-first-narrative-data-design.md`

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `src/narrative_data/pipeline/events.py` | Pipeline JSONL event log: append, read, state derivation |
| `src/narrative_data/discovery/__init__.py` | Package marker |
| `src/narrative_data/discovery/commands.py` | Phase 1 extraction + Phase 2 synthesis orchestration |
| `src/narrative_data/primitive/__init__.py` | Package marker |
| `src/narrative_data/primitive/commands.py` | Phase 3 Layer 0 elicitation + structuring |
| `prompts/discovery/extract-archetypes.md` | Phase 1 archetype extraction template |
| `prompts/discovery/extract-dynamics.md` | Phase 1 dynamics extraction template |
| `prompts/discovery/extract-goals.md` | Phase 1 goals extraction template |
| `prompts/discovery/extract-profiles.md` | Phase 1 profiles extraction template |
| `prompts/discovery/extract-settings.md` | Phase 1 settings extraction template |
| `prompts/discovery/synthesize-archetypes.md` | Phase 2 archetype cluster synthesis template |
| `prompts/discovery/synthesize-dynamics.md` | Phase 2 dynamics cluster synthesis template |
| `prompts/discovery/synthesize-goals.md` | Phase 2 goals cluster synthesis template |
| `prompts/discovery/synthesize-profiles.md` | Phase 2 profiles cluster synthesis template |
| `prompts/discovery/synthesize-settings.md` | Phase 2 settings cluster synthesis template |
| `prompts/primitive/archetypes.md` | Phase 3 standalone archetype elicitation template |
| `prompts/primitive/dynamics.md` | Phase 3 standalone dynamics template |
| `prompts/primitive/goals.md` | Phase 3 standalone goals template |
| `prompts/primitive/profiles.md` | Phase 3 standalone profiles template |
| `prompts/primitive/settings.md` | Phase 3 standalone settings template |
| `prompts/genre/elaborate-archetypes.md` | Phase 4a genre × archetype template |
| `prompts/genre/elaborate-dynamics.md` | Phase 4a genre × dynamics template |
| `prompts/genre/elaborate-goals.md` | Phase 4a genre × goals template |
| `prompts/genre/elaborate-profiles.md` | Phase 4a genre × profiles template |
| `prompts/genre/elaborate-settings.md` | Phase 4a genre × settings template |
| `tests/test_events.py` | Pipeline event log tests |
| `tests/test_discovery.py` | Discovery command tests |
| `tests/test_primitive.py` | Primitive command tests |

### Modified Files

| File | Changes |
|------|---------|
| `src/narrative_data/config.py` | Add GENRE_CLUSTERS, PRIMITIVE_TYPES, GENRE_NATIVE_TYPES, MODIFIER_REGIONS |
| `src/narrative_data/cli.py` | Add `discover`, `primitive`, `pipeline` subgroups; add `genre elaborate`, `genre elicit-native`; restrict `genre elicit` to region-only |
| `src/narrative_data/genre/commands.py` | Remove `_load_descriptor_context()`; add `elaborate_genre()`, `elicit_native()`; restrict `elicit_genre()` categories |
| `src/narrative_data/prompts.py` | Add `build_discovery()` and `build_synthesis()` methods to PromptBuilder |
| `prompts/genre/tropes.md` | Remove descriptor injection references (if any in template text) |
| `prompts/genre/narrative-shapes.md` | Remove descriptor injection references (if any in template text) |
| `tests/test_commands.py` | Update genre elicit tests for region-only restriction; add elaborate/elicit-native tests |
| `tests/test_config.py` | Add tests for new config constants |
| `tests/test_prompts.py` | Add tests for new PromptBuilder methods |
| `tests/test_cli.py` | Add tests for new CLI subgroups and commands |

---

### Task 1: Pipeline Event Log

**Files:**
- Create: `src/narrative_data/pipeline/events.py`
- Test: `tests/test_events.py`

This is the foundation — every subsequent task emits events through this module.

- [ ] **Step 1: Write tests for event append and read**

```python
# tests/test_events.py
import json
from pathlib import Path

from narrative_data.pipeline.events import append_event, read_events


class TestAppendEvent:
    def test_creates_file_on_first_append(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        append_event(log_path, event="extract_started", phase=1, type="archetypes", genre="folk-horror")
        assert log_path.exists()
        lines = log_path.read_text().strip().split("\n")
        assert len(lines) == 1
        data = json.loads(lines[0])
        assert data["event"] == "extract_started"
        assert data["phase"] == 1
        assert data["type"] == "archetypes"
        assert data["genre"] == "folk-horror"
        assert "timestamp" in data

    def test_appends_to_existing_file(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        append_event(log_path, event="extract_started", phase=1, type="archetypes", genre="folk-horror")
        append_event(log_path, event="extract_completed", phase=1, type="archetypes", genre="folk-horror",
                     output="discovery/archetypes/folk-horror.raw.md", content_digest="sha256:abc")
        lines = log_path.read_text().strip().split("\n")
        assert len(lines) == 2

    def test_extra_fields_preserved(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        append_event(log_path, event="review_gate", phase=2, type="archetypes",
                     decision="approved", primitives=["mentor", "trickster"], note="looks good")
        data = json.loads(log_path.read_text().strip())
        assert data["primitives"] == ["mentor", "trickster"]
        assert data["note"] == "looks good"


class TestReadEvents:
    def test_empty_file(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        assert read_events(log_path) == []

    def test_nonexistent_file(self, tmp_path):
        log_path = tmp_path / "nonexistent.jsonl"
        assert read_events(log_path) == []

    def test_reads_all_events(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        append_event(log_path, event="extract_started", phase=1, type="archetypes", genre="folk-horror")
        append_event(log_path, event="extract_completed", phase=1, type="archetypes", genre="folk-horror",
                     output="x.md", content_digest="sha256:abc")
        events = read_events(log_path)
        assert len(events) == 2
        assert events[0]["event"] == "extract_started"
        assert events[1]["event"] == "extract_completed"
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_events.py -v`
Expected: FAIL — `ModuleNotFoundError: No module named 'narrative_data.pipeline.events'`

- [ ] **Step 3: Implement event append and read**

```python
# src/narrative_data/pipeline/events.py
"""Pipeline event log: append-only JSONL for tracking pipeline progress."""

import json
from pathlib import Path

from narrative_data.utils import now_iso


def append_event(log_path: Path, *, event: str, phase: int, type: str, **kwargs: object) -> None:
    """Append a single event to the pipeline JSONL log."""
    entry = {"event": event, "phase": phase, "type": type, "timestamp": now_iso(), **kwargs}
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("a") as f:
        f.write(json.dumps(entry, default=str) + "\n")


def read_events(log_path: Path) -> list[dict]:
    """Read all events from the pipeline JSONL log."""
    if not log_path.exists():
        return []
    events = []
    for line in log_path.read_text().strip().split("\n"):
        if line:
            events.append(json.loads(line))
    return events
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_events.py -v`
Expected: PASS

- [ ] **Step 5: Write tests for state derivation**

```python
# Append to tests/test_events.py
from narrative_data.pipeline.events import derive_state


class TestDeriveState:
    def _log(self, tmp_path, events_data):
        """Helper to write multiple events."""
        log_path = tmp_path / "pipeline.jsonl"
        for ev in events_data:
            append_event(log_path, **ev)
        return log_path

    def test_empty_state(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        state = derive_state(log_path, "archetypes")
        assert state["phase1_completed"] == set()
        assert state["phase2_completed"] == set()
        assert state["phase2_gate"] is None
        assert state["phase3_completed"] == set()
        assert state["phase4_completed"] == set()

    def test_phase1_progress(self, tmp_path):
        log_path = self._log(tmp_path, [
            {"event": "extract_completed", "phase": 1, "type": "archetypes", "genre": "folk-horror",
             "output": "x.md", "content_digest": "sha256:a"},
            {"event": "extract_completed", "phase": 1, "type": "archetypes", "genre": "cosmic-horror",
             "output": "y.md", "content_digest": "sha256:b"},
        ])
        state = derive_state(log_path, "archetypes")
        assert state["phase1_completed"] == {"folk-horror", "cosmic-horror"}

    def test_filters_by_type(self, tmp_path):
        log_path = self._log(tmp_path, [
            {"event": "extract_completed", "phase": 1, "type": "archetypes", "genre": "folk-horror",
             "output": "x.md", "content_digest": "sha256:a"},
            {"event": "extract_completed", "phase": 1, "type": "dynamics", "genre": "folk-horror",
             "output": "y.md", "content_digest": "sha256:b"},
        ])
        state = derive_state(log_path, "archetypes")
        assert state["phase1_completed"] == {"folk-horror"}
        state_dyn = derive_state(log_path, "dynamics")
        assert state_dyn["phase1_completed"] == {"folk-horror"}

    def test_review_gate_captured(self, tmp_path):
        log_path = self._log(tmp_path, [
            {"event": "review_gate", "phase": 2, "type": "archetypes",
             "decision": "approved", "primitives": ["mentor", "trickster"]},
        ])
        state = derive_state(log_path, "archetypes")
        assert state["phase2_gate"] == {"decision": "approved", "primitives": ["mentor", "trickster"]}

    def test_phase4_tracks_genre_primitive_pairs(self, tmp_path):
        log_path = self._log(tmp_path, [
            {"event": "elaborate_completed", "phase": 4, "type": "archetypes",
             "genre": "folk-horror", "primitive": "mentor", "output": "x.md", "content_digest": "sha256:a"},
        ])
        state = derive_state(log_path, "archetypes")
        assert ("folk-horror", "mentor") in state["phase4_completed"]
```

- [ ] **Step 6: Implement state derivation**

```python
# Append to src/narrative_data/pipeline/events.py

def derive_state(log_path: Path, primitive_type: str) -> dict:
    """Derive current pipeline state for a primitive type by replaying the event log."""
    events = read_events(log_path)
    state = {
        "phase1_completed": set(),   # genre slugs
        "phase2_completed": set(),   # cluster names
        "phase2_gate": None,         # review gate event dict or None
        "phase3_completed": set(),   # primitive slugs
        "phase4_completed": set(),   # (genre, primitive) tuples
        "phase4_native_completed": set(),  # (genre, native_type) tuples
    }
    for ev in events:
        if ev.get("type") != primitive_type:
            continue
        match ev["event"]:
            case "extract_completed":
                state["phase1_completed"].add(ev["genre"])
            case "synthesize_completed":
                state["phase2_completed"].add(ev["cluster"])
            case "review_gate" if ev.get("phase") == 2 and ev.get("decision") == "approved":
                state["phase2_gate"] = {
                    "decision": ev["decision"],
                    "primitives": ev.get("primitives", []),
                }
            case "elicit_completed" if ev.get("phase") == 3:
                state["phase3_completed"].add(ev["primitive"])
            case "elaborate_completed":
                state["phase4_completed"].add((ev["genre"], ev["primitive"]))
            case "elicit_native_completed":
                state["phase4_native_completed"].add((ev["genre"], ev["native_type"]))
    return state
```

- [ ] **Step 7: Run all event tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_events.py -v`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add tests/test_events.py src/narrative_data/pipeline/events.py
git commit -m "feat: add pipeline event log with append, read, and state derivation"
```

---

### Task 2: Configuration Updates

**Files:**
- Modify: `src/narrative_data/config.py`
- Test: `tests/test_config.py`

Add genre clusters, primitive types, genre-native types, and modifier regions to config.

- [ ] **Step 1: Write tests for new config constants**

```python
# Append to tests/test_config.py

from narrative_data.config import GENRE_CLUSTERS, PRIMITIVE_TYPES, GENRE_NATIVE_TYPES, MODIFIER_REGIONS


class TestNewConstants:
    def test_genre_clusters_cover_all_standalone_regions(self):
        from narrative_data.genre.commands import GENRE_REGIONS
        all_clustered = set()
        for genres in GENRE_CLUSTERS.values():
            all_clustered.update(genres)
        # All non-modifier regions should appear in exactly one cluster
        standalone = set(GENRE_REGIONS) - set(MODIFIER_REGIONS)
        assert standalone == all_clustered

    def test_primitive_types_are_strings(self):
        assert all(isinstance(t, str) for t in PRIMITIVE_TYPES)
        assert "archetypes" in PRIMITIVE_TYPES
        assert len(PRIMITIVE_TYPES) == 5

    def test_genre_native_types(self):
        assert "tropes" in GENRE_NATIVE_TYPES
        assert "narrative-shapes" in GENRE_NATIVE_TYPES
        assert len(GENRE_NATIVE_TYPES) == 2

    def test_modifier_regions(self):
        assert "solarpunk" in MODIFIER_REGIONS
        assert len(MODIFIER_REGIONS) == 4
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_config.py::TestNewConstants -v`
Expected: FAIL — `ImportError`

- [ ] **Step 3: Add constants to config.py**

Add to `src/narrative_data/config.py` after the existing constants:

```python
PRIMITIVE_TYPES: list[str] = [
    "archetypes", "dynamics", "goals", "profiles", "settings",
]

GENRE_NATIVE_TYPES: list[str] = [
    "tropes", "narrative-shapes",
]

MODIFIER_REGIONS: list[str] = [
    "solarpunk", "historical-fiction", "literary-fiction", "magical-realism",
]

GENRE_CLUSTERS: dict[str, list[str]] = {
    "horror": ["folk-horror", "cosmic-horror", "horror-comedy"],
    "fantasy": [
        "high-epic-fantasy", "dark-fantasy", "cozy-fantasy",
        "fairy-tale-mythic", "urban-fantasy", "quiet-contemplative-fantasy",
    ],
    "sci-fi": ["hard-sci-fi", "space-opera", "cyberpunk"],
    "mystery-thriller": [
        "nordic-noir", "cozy-mystery", "psychological-thriller", "domestic-noir",
    ],
    "romance": ["romantasy", "historical-romance", "contemporary-romance"],
    "realism-gothic-other": [
        "southern-gothic", "westerns", "swashbuckling-adventure",
        "survival-fiction", "working-class-realism",
        "pastoral-rural-fiction", "classical-tragedy",
    ],
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_config.py -v`
Expected: PASS

- [ ] **Step 5: Run full test suite to check for regressions**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All existing tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/narrative_data/config.py tests/test_config.py
git commit -m "feat: add genre clusters, primitive types, and modifier regions to config"
```

---

### Task 3: PromptBuilder Extensions

**Files:**
- Modify: `src/narrative_data/prompts.py`
- Test: `tests/test_prompts.py`

Add methods for discovery and synthesis prompts that handle the new context patterns (genre content injection for Phase 1, multi-file concatenation for Phase 2).

- [ ] **Step 1: Write tests for new PromptBuilder methods**

```python
# Append to tests/test_prompts.py

class TestBuildDiscovery:
    def test_injects_genre_content(self, tmp_path):
        prompts_dir = tmp_path / "prompts"
        (prompts_dir / "discovery").mkdir(parents=True)
        (prompts_dir / "discovery" / "extract-archetypes.md").write_text(
            "Extract archetypes for {target_name}.\n\n## Genre Region Description\n\n{genre_content}"
        )
        builder = PromptBuilder(prompts_dir=prompts_dir)
        result = builder.build_discovery(
            primitive_type="archetypes",
            target_name="Folk Horror",
            genre_content="Rich description of folk horror...",
        )
        assert "Folk Horror" in result
        assert "Rich description of folk horror..." in result

    def test_appends_commentary(self, tmp_path):
        prompts_dir = tmp_path / "prompts"
        (prompts_dir / "discovery").mkdir(parents=True)
        (prompts_dir / "discovery" / "extract-archetypes.md").write_text("Extract for {target_name}.\n{genre_content}")
        (prompts_dir / "_commentary.md").write_text("Add commentary.")
        builder = PromptBuilder(prompts_dir=prompts_dir)
        result = builder.build_discovery("archetypes", "Test", "content")
        assert "Add commentary." in result


class TestBuildSynthesis:
    def test_injects_extractions(self, tmp_path):
        prompts_dir = tmp_path / "prompts"
        (prompts_dir / "discovery").mkdir(parents=True)
        (prompts_dir / "discovery" / "synthesize-archetypes.md").write_text(
            "Synthesize {primitive_type} for {cluster_name} ({genre_count} genres).\n\n{extractions}"
        )
        builder = PromptBuilder(prompts_dir=prompts_dir)
        extractions = {"folk-horror": "archetype data 1", "cosmic-horror": "archetype data 2"}
        result = builder.build_synthesis(
            primitive_type="archetypes",
            cluster_name="Horror",
            extractions=extractions,
        )
        assert "Horror" in result
        assert "folk-horror" in result
        assert "archetype data 1" in result
        assert "2 genres" in result
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_prompts.py::TestBuildDiscovery tests/test_prompts.py::TestBuildSynthesis -v`
Expected: FAIL — `AttributeError: 'PromptBuilder' object has no attribute 'build_discovery'`

- [ ] **Step 3: Implement new PromptBuilder methods**

Add to `src/narrative_data/prompts.py` in the `PromptBuilder` class:

```python
def build_discovery(
    self, primitive_type: str, target_name: str, genre_content: str,
) -> str:
    """Build a Phase 1 discovery extraction prompt."""
    template_path = self.prompts_dir / "discovery" / f"extract-{primitive_type}.md"
    if not template_path.exists():
        raise FileNotFoundError(f"Discovery prompt template not found: {template_path}")
    prompt = template_path.read_text()
    prompt = prompt.replace("{target_name}", target_name)
    prompt = prompt.replace("{genre_content}", genre_content)
    prompt += "\n\n" + self._load_commentary_directive()
    return prompt

def build_synthesis(
    self, primitive_type: str, cluster_name: str, extractions: dict[str, str],
) -> str:
    """Build a Phase 2 cluster synthesis prompt."""
    template_path = self.prompts_dir / "discovery" / f"synthesize-{primitive_type}.md"
    if not template_path.exists():
        raise FileNotFoundError(f"Synthesis prompt template not found: {template_path}")
    prompt = template_path.read_text()
    prompt = prompt.replace("{primitive_type}", primitive_type)
    prompt = prompt.replace("{cluster_name}", cluster_name)
    prompt = prompt.replace("{genre_count}", str(len(extractions)))
    # Concatenate extractions with genre headers
    extraction_text = ""
    for genre_slug, content in extractions.items():
        extraction_text += f"### {genre_slug}\n\n{content}\n\n"
    prompt = prompt.replace("{extractions}", extraction_text)
    prompt += "\n\n" + self._load_commentary_directive()
    return prompt
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_prompts.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/narrative_data/prompts.py tests/test_prompts.py
git commit -m "feat: add build_discovery and build_synthesis methods to PromptBuilder"
```

---

### Task 4: Discovery Commands (Phase 1 + Phase 2)

**Files:**
- Create: `src/narrative_data/discovery/__init__.py`
- Create: `src/narrative_data/discovery/commands.py`
- Test: `tests/test_discovery.py`

Orchestration for Phase 1 (per-genre extraction) and Phase 2 (cluster synthesis). Each operation emits pipeline events.

- [ ] **Step 1: Create package marker**

```python
# src/narrative_data/discovery/__init__.py
```

- [ ] **Step 2: Write tests for Phase 1 extraction**

Tests create mock prompt templates in tmp dirs so they don't depend on Task 8 prompt authoring.

```python
# tests/test_discovery.py
from pathlib import Path
from unittest.mock import MagicMock

from narrative_data.discovery.commands import extract_primitives, synthesize_cluster
from narrative_data.pipeline.events import read_events


def _make_mock_prompts(tmp_path):
    """Create minimal prompt templates for testing."""
    prompts_dir = tmp_path / "prompts"
    (prompts_dir / "discovery").mkdir(parents=True)
    (prompts_dir / "discovery" / "extract-archetypes.md").write_text(
        "Extract archetypes for {target_name}.\n\n{genre_content}"
    )
    (prompts_dir / "discovery" / "synthesize-archetypes.md").write_text(
        "Synthesize {primitive_type} for {cluster_name} ({genre_count} genres).\n\n{extractions}"
    )
    return prompts_dir


class TestExtractPrimitives:
    def test_extracts_for_specified_genres(self, tmp_output_dir):
        client = MagicMock()
        client.generate.return_value = "# Extracted archetypes for Folk Horror\n\n- The Outsider\n- The Land-Keeper"
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        prompts_dir = _make_mock_prompts(output_base)
        # Create a genre region file for context
        genre_dir = output_base / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True)
        (genre_dir / "region.raw.md").write_text("Folk horror region description...")

        extract_primitives(
            client=client,
            output_base=output_base,
            log_path=log_path,
            primitive_type="archetypes",
            genres=["folk-horror"],
            prompts_dir=prompts_dir,
        )

        # Check output file was created
        out_file = output_base / "discovery" / "archetypes" / "folk-horror.raw.md"
        assert out_file.exists()
        assert "Outsider" in out_file.read_text()

        # Check pipeline events were emitted
        events = read_events(log_path)
        assert len(events) == 2
        assert events[0]["event"] == "extract_started"
        assert events[1]["event"] == "extract_completed"
        assert events[1]["genre"] == "folk-horror"

    def test_skips_genre_without_region_file(self, tmp_output_dir):
        client = MagicMock()
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        prompts_dir = _make_mock_prompts(output_base)

        extract_primitives(
            client=client,
            output_base=output_base,
            log_path=log_path,
            primitive_type="archetypes",
            genres=["nonexistent-genre"],
            prompts_dir=prompts_dir,
        )

        assert not client.generate.called
        assert read_events(log_path) == []


class TestSynthesizeCluster:
    def test_synthesizes_from_extraction_files(self, tmp_output_dir):
        client = MagicMock()
        client.generate.return_value = "# Cluster Synthesis: Horror Archetypes\n\n1. The Outsider..."
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        prompts_dir = _make_mock_prompts(output_base)
        # Create extraction files
        disc_dir = output_base / "discovery" / "archetypes"
        disc_dir.mkdir(parents=True)
        (disc_dir / "folk-horror.raw.md").write_text("Folk horror archetypes...")
        (disc_dir / "cosmic-horror.raw.md").write_text("Cosmic horror archetypes...")

        synthesize_cluster(
            client=client,
            output_base=output_base,
            log_path=log_path,
            primitive_type="archetypes",
            cluster_name="horror",
            genres=["folk-horror", "cosmic-horror"],
            prompts_dir=prompts_dir,
        )

        out_file = disc_dir / "cluster-horror.raw.md"
        assert out_file.exists()

        events = read_events(log_path)
        assert len(events) == 2
        assert events[0]["event"] == "synthesize_started"
        assert events[1]["event"] == "synthesize_completed"
        assert events[1]["cluster"] == "horror"
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_discovery.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 4: Implement discovery commands**

```python
# src/narrative_data/discovery/commands.py
"""Discovery pipeline: Phase 1 extraction and Phase 2 cluster synthesis."""

from pathlib import Path

from rich.console import Console

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.events import append_event
from narrative_data.pipeline.invalidation import (
    archive_existing,
    compute_content_digest,
    compute_prompt_hash,
    load_manifest,
    update_manifest_entry,
)
from narrative_data.prompts import PromptBuilder
from narrative_data.utils import now_iso, slug_to_name

console = Console()


def extract_primitives(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    primitive_type: str,
    genres: list[str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
    prompts_dir: Path | None = None,
) -> None:
    """Phase 1: Extract primitive candidates from each genre's region description."""
    builder = PromptBuilder(prompts_dir=prompts_dir) if prompts_dir else PromptBuilder()
    disc_dir = output_base / "discovery" / primitive_type
    disc_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = disc_dir / "manifest.json"

    for genre_slug in genres:
        region_path = output_base / "genres" / genre_slug / "region.raw.md"
        if not region_path.exists():
            console.print(f"[dim]  Skipping {genre_slug} — no region.raw.md[/dim]")
            continue

        output_path = disc_dir / f"{genre_slug}.raw.md"
        manifest_key = f"{genre_slug}"

        # Check staleness
        genre_content = region_path.read_text()
        try:
            prompt = builder.build_discovery(primitive_type, slug_to_name(genre_slug), genre_content)
        except FileNotFoundError:
            console.print(f"[dim]  Skipping {primitive_type} — prompt template missing[/dim]")
            return

        current_hash = compute_prompt_hash(prompt)
        if not force:
            entry = load_manifest(manifest_path).get("entries", {}).get(manifest_key)
            if entry and entry.get("prompt_hash") == current_hash and output_path.exists():
                console.print(f"[dim]  {genre_slug}/{primitive_type} up to date, skipping[/dim]")
                continue

        append_event(log_path, event="extract_started", phase=1, type=primitive_type, genre=genre_slug)
        console.print(f"[cyan]  Extracting {primitive_type} from {genre_slug}…[/cyan]")

        result_text = client.generate(model=model, prompt=prompt)
        archive_existing(output_path)
        output_path.write_text(result_text)

        digest = compute_content_digest(result_text)
        update_manifest_entry(manifest_path, manifest_key, {
            "prompt_hash": current_hash,
            "content_digest": digest,
            "elicited_at": now_iso(),
            "raw_path": str(output_path),
        })

        append_event(log_path, event="extract_completed", phase=1, type=primitive_type,
                     genre=genre_slug, output=str(output_path.relative_to(output_base)),
                     content_digest=digest)


def synthesize_cluster(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    primitive_type: str,
    cluster_name: str,
    genres: list[str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
    prompts_dir: Path | None = None,
) -> None:
    """Phase 2: Synthesize per-genre extractions into a deduplicated cluster list."""
    builder = PromptBuilder(prompts_dir=prompts_dir) if prompts_dir else PromptBuilder()
    disc_dir = output_base / "discovery" / primitive_type
    output_path = disc_dir / f"cluster-{cluster_name}.raw.md"

    # Gather extraction files
    extractions: dict[str, str] = {}
    for genre_slug in genres:
        ext_path = disc_dir / f"{genre_slug}.raw.md"
        if ext_path.exists():
            extractions[genre_slug] = ext_path.read_text()
        else:
            console.print(f"[dim]  No extraction for {genre_slug}, skipping in synthesis[/dim]")

    if not extractions:
        console.print(f"[yellow]  No extractions found for cluster {cluster_name}, skipping[/yellow]")
        return

    try:
        prompt = builder.build_synthesis(primitive_type, slug_to_name(cluster_name), extractions)
    except FileNotFoundError:
        console.print(f"[dim]  Skipping synthesis — prompt template missing[/dim]")
        return

    append_event(log_path, event="synthesize_started", phase=2, type=primitive_type, cluster=cluster_name)
    console.print(f"[cyan]  Synthesizing {primitive_type} for cluster {cluster_name}…[/cyan]")

    result_text = client.generate(model=model, prompt=prompt)
    archive_existing(output_path)
    output_path.write_text(result_text)

    digest = compute_content_digest(result_text)
    append_event(log_path, event="synthesize_completed", phase=2, type=primitive_type,
                 cluster=cluster_name, output=str(output_path.relative_to(output_base)),
                 content_digest=digest, primitives_found=result_text.count("\n##"))
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_discovery.py -v`
Expected: PASS

- [ ] **Step 6: Run full test suite**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/narrative_data/discovery/ tests/test_discovery.py
git commit -m "feat: add discovery commands for Phase 1 extraction and Phase 2 synthesis"
```

---

### Task 5: Primitive Commands (Phase 3)

**Files:**
- Create: `src/narrative_data/primitive/__init__.py`
- Create: `src/narrative_data/primitive/commands.py`
- Test: `tests/test_primitive.py`

Phase 3 Layer 0 standalone elicitation. Uses existing `run_elicitation()` infrastructure.

- [ ] **Step 1: Create package marker**

```python
# src/narrative_data/primitive/__init__.py
```

- [ ] **Step 2: Write tests for primitive elicitation**

Tests create mock prompt templates so they don't depend on Task 8.

```python
# tests/test_primitive.py
from pathlib import Path
from unittest.mock import MagicMock

from narrative_data.primitive.commands import elicit_primitives
from narrative_data.pipeline.events import read_events


def _make_mock_prompts(tmp_path):
    """Create minimal prompt templates for testing."""
    prompts_dir = tmp_path / "prompts"
    (prompts_dir / "primitive").mkdir(parents=True)
    (prompts_dir / "primitive" / "archetypes.md").write_text(
        "Analyze the archetype {target_name}."
    )
    return prompts_dir


class TestElicitPrimitives:
    def test_elicits_specified_primitives(self, tmp_output_dir):
        client = MagicMock()
        client.generate.return_value = "# The Mentor\n\nA rich standalone description..."
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        prompts_dir = _make_mock_prompts(output_base)

        elicit_primitives(
            client=client,
            output_base=output_base,
            log_path=log_path,
            primitive_type="archetypes",
            primitives=["mentor"],
            descriptions={"mentor": "A guide figure who possesses dangerous knowledge"},
            prompts_dir=prompts_dir,
        )

        out_file = output_base / "archetypes" / "mentor" / "raw.md"
        assert out_file.exists()
        assert "Mentor" in out_file.read_text()

        events = read_events(log_path)
        assert len(events) == 2
        assert events[0]["event"] == "elicit_started"
        assert events[0]["primitive"] == "mentor"
        assert events[1]["event"] == "elicit_completed"

    def test_respects_review_gate(self, tmp_output_dir):
        """Primitives list comes from the review gate — test that only listed primitives are elicited."""
        client = MagicMock()
        client.generate.return_value = "Description..."
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        prompts_dir = _make_mock_prompts(output_base)

        elicit_primitives(
            client=client,
            output_base=output_base,
            log_path=log_path,
            primitive_type="archetypes",
            primitives=["mentor", "trickster"],
            descriptions={"mentor": "desc1", "trickster": "desc2"},
            prompts_dir=prompts_dir,
        )

        assert client.generate.call_count == 2
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_primitive.py -v`
Expected: FAIL

- [ ] **Step 4: Implement primitive commands**

```python
# src/narrative_data/primitive/commands.py
"""Primitive elicitation: Phase 3 Layer 0 standalone descriptions."""

from pathlib import Path

from rich.console import Console

from narrative_data.config import ELICITATION_MODEL
from narrative_data.ollama import OllamaClient
from narrative_data.pipeline.events import append_event
from narrative_data.pipeline.invalidation import (
    archive_existing,
    compute_content_digest,
    compute_prompt_hash,
    load_manifest,
    update_manifest_entry,
)
from narrative_data.prompts import PromptBuilder
from narrative_data.utils import now_iso, slug_to_name

console = Console()


def elicit_primitives(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    primitive_type: str,
    primitives: list[str],
    descriptions: dict[str, str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
    prompts_dir: Path | None = None,
) -> None:
    """Phase 3: Elicit standalone Layer 0 descriptions for each primitive."""
    builder = PromptBuilder(prompts_dir=prompts_dir) if prompts_dir else PromptBuilder()
    type_dir = output_base / primitive_type
    manifest_path = type_dir / "manifest.json"

    for prim_slug in primitives:
        prim_dir = type_dir / prim_slug
        prim_dir.mkdir(parents=True, exist_ok=True)
        output_path = prim_dir / "raw.md"

        description = descriptions.get(prim_slug, "")
        prim_name = slug_to_name(prim_slug)

        try:
            prompt = builder.build_stage1(
                domain="primitive",
                category=primitive_type,
                target_name=prim_name,
                context={"synthesis_description": description} if description else None,
            )
        except FileNotFoundError:
            console.print(f"[dim]  Skipping {primitive_type} — prompt template missing[/dim]")
            return

        current_hash = compute_prompt_hash(prompt)
        if not force:
            entry = load_manifest(manifest_path).get("entries", {}).get(prim_slug)
            if entry and entry.get("prompt_hash") == current_hash and output_path.exists():
                console.print(f"[dim]  {prim_slug} up to date, skipping[/dim]")
                continue

        append_event(log_path, event="elicit_started", phase=3, type=primitive_type, primitive=prim_slug)
        console.print(f"[cyan]  Eliciting {primitive_type}/{prim_slug}…[/cyan]")

        result_text = client.generate(model=model, prompt=prompt)
        archive_existing(output_path)
        output_path.write_text(result_text)

        digest = compute_content_digest(result_text)
        update_manifest_entry(manifest_path, prim_slug, {
            "prompt_hash": current_hash,
            "content_digest": digest,
            "elicited_at": now_iso(),
            "raw_path": str(output_path),
        })

        append_event(log_path, event="elicit_completed", phase=3, type=primitive_type,
                     primitive=prim_slug, output=str(output_path.relative_to(output_base)),
                     content_digest=digest)
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_primitive.py -v`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/narrative_data/primitive/ tests/test_primitive.py
git commit -m "feat: add primitive commands for Phase 3 Layer 0 elicitation"
```

---

### Task 6: Genre Elaborate + Elicit-Native (Phase 4)

**Files:**
- Modify: `src/narrative_data/genre/commands.py`
- Modify: `tests/test_commands.py`

Add `elaborate_genre()` for Phase 4a (genre × primitive) and `elicit_native()` for Phase 4b (genre-native tropes/shapes). Also restrict `elicit_genre()` to region-only and remove `_load_descriptor_context()`.

- [ ] **Step 1: Write tests for elaborate and elicit-native**

```python
# Append to tests/test_commands.py

from narrative_data.genre.commands import elaborate_genre, elicit_native


class TestElaborateGenre:
    def test_elaborates_genre_primitive_pair(self, tmp_output_dir):
        client = MagicMock()
        client.generate.return_value = "# The Mentor in Folk Horror\n\nElaboration..."
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        # Create genre region and primitive files
        genre_dir = output_base / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True)
        (genre_dir / "region.raw.md").write_text("Folk horror description...")
        prim_dir = output_base / "archetypes" / "mentor"
        prim_dir.mkdir(parents=True)
        (prim_dir / "raw.md").write_text("Standalone mentor description...")

        elaborate_genre(
            client=client,
            output_base=output_base,
            log_path=log_path,
            primitive_type="archetypes",
            genres=["folk-horror"],
            primitives=["mentor"],
        )

        out_file = genre_dir / "elaborations" / "archetypes" / "mentor.raw.md"
        assert out_file.exists()

        from narrative_data.pipeline.events import read_events
        events = read_events(log_path)
        assert any(e["event"] == "elaborate_completed" for e in events)


class TestElicitNative:
    def test_elicits_tropes_for_genre(self, tmp_output_dir):
        client = MagicMock()
        client.generate.return_value = "# Folk Horror Tropes\n\nTrope analysis..."
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        genre_dir = output_base / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True)
        (genre_dir / "region.raw.md").write_text("Folk horror description...")

        elicit_native(
            client=client,
            output_base=output_base,
            log_path=log_path,
            native_type="tropes",
            genres=["folk-horror"],
        )

        out_file = genre_dir / "tropes.raw.md"
        assert out_file.exists()

        from narrative_data.pipeline.events import read_events
        events = read_events(log_path)
        assert any(e["event"] == "elicit_native_completed" for e in events)


class TestElicitGenreRestriction:
    def test_rejects_non_region_categories(self, tmp_output_dir):
        import pytest
        from narrative_data.genre.commands import elicit_genre

        client = MagicMock()
        output_base = tmp_output_dir
        manifest_path = output_base / "genres" / "manifest.json"

        with pytest.raises(ValueError, match="Use 'narrative-data discover'"):
            elicit_genre(
                client=client,
                output_base=output_base,
                manifest_path=manifest_path,
                categories=["archetypes"],
            )

    def test_allows_region_category(self, tmp_output_dir):
        from narrative_data.genre.commands import elicit_genre

        client = MagicMock()
        output_base = tmp_output_dir
        manifest_path = output_base / "genres" / "manifest.json"

        # Should not raise — region is still allowed
        elicit_genre(
            client=client,
            output_base=output_base,
            manifest_path=manifest_path,
            categories=["region"],
        )
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_commands.py::TestElaborateGenre tests/test_commands.py::TestElicitNative tests/test_commands.py::TestElicitGenreRestriction -v`
Expected: FAIL

- [ ] **Step 3: Implement elaborate_genre and elicit_native**

Add to `src/narrative_data/genre/commands.py`:

```python
def elaborate_genre(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    primitive_type: str,
    genres: list[str],
    primitives: list[str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
) -> None:
    """Phase 4a: Elaborate genre × primitive pairs."""
    builder = PromptBuilder()

    for genre_slug in genres:
        region_path = output_base / "genres" / genre_slug / "region.raw.md"
        if not region_path.exists():
            console.print(f"[dim]  Skipping {genre_slug} — no region.raw.md[/dim]")
            continue
        genre_content = region_path.read_text()

        for prim_slug in primitives:
            prim_path = output_base / primitive_type / prim_slug / "raw.md"
            if not prim_path.exists():
                console.print(f"[dim]  Skipping {prim_slug} — no Layer 0 raw.md[/dim]")
                continue
            prim_content = prim_path.read_text()

            elab_dir = output_base / "genres" / genre_slug / "elaborations" / primitive_type
            elab_dir.mkdir(parents=True, exist_ok=True)
            output_path = elab_dir / f"{prim_slug}.raw.md"

            context = {"genre_description": genre_content, "primitive_description": prim_content}
            try:
                prompt = builder.build_stage1(
                    domain="genre",
                    category=f"elaborate-{primitive_type}",
                    target_name=f"{slug_to_name(prim_slug)} in {slug_to_name(genre_slug)}",
                    context=context,
                )
            except FileNotFoundError:
                console.print(f"[dim]  Skipping elaborate-{primitive_type} — prompt template missing[/dim]")
                return

            current_hash = compute_prompt_hash(prompt)
            manifest_path = output_base / "genres" / "manifest.json"
            manifest_key = f"{genre_slug}/elaborations/{primitive_type}/{prim_slug}"
            if not force:
                entry = load_manifest(manifest_path).get("entries", {}).get(manifest_key)
                if entry and entry.get("prompt_hash") == current_hash and output_path.exists():
                    console.print(f"[dim]  {genre_slug}/{primitive_type}/{prim_slug} up to date[/dim]")
                    continue

            append_event(log_path, event="elaborate_started", phase=4, type=primitive_type,
                         genre=genre_slug, primitive=prim_slug)
            console.print(f"[cyan]  Elaborating {primitive_type}/{prim_slug} for {genre_slug}…[/cyan]")

            result_text = client.generate(model=model, prompt=prompt)
            archive_existing(output_path)
            output_path.write_text(result_text)

            digest = compute_content_digest(result_text)
            update_manifest_entry(manifest_path, manifest_key, {
                "prompt_hash": current_hash,
                "content_digest": digest,
                "elicited_at": now_iso(),
                "raw_path": str(output_path),
            })

            append_event(log_path, event="elaborate_completed", phase=4, type=primitive_type,
                         genre=genre_slug, primitive=prim_slug,
                         output=str(output_path.relative_to(output_base)), content_digest=digest)


def elicit_native(
    client: OllamaClient,
    output_base: Path,
    log_path: Path,
    native_type: str,
    genres: list[str],
    model: str = ELICITATION_MODEL,
    force: bool = False,
) -> None:
    """Phase 4b: Elicit genre-native tropes or narrative-shapes."""
    builder = PromptBuilder()

    for genre_slug in genres:
        region_path = output_base / "genres" / genre_slug / "region.raw.md"
        if not region_path.exists():
            console.print(f"[dim]  Skipping {genre_slug} — no region.raw.md[/dim]")
            continue

        genre_content = region_path.read_text()
        genre_dir = output_base / "genres" / genre_slug
        output_path = genre_dir / f"{native_type}.raw.md"

        context = {"genre_description": genre_content}
        try:
            prompt = builder.build_stage1(
                domain="genre",
                category=native_type,
                target_name=slug_to_name(genre_slug),
                context=context,
            )
        except FileNotFoundError:
            console.print(f"[dim]  Skipping {native_type} — prompt template missing[/dim]")
            return

        current_hash = compute_prompt_hash(prompt)
        manifest_path = output_base / "genres" / "manifest.json"
        manifest_key = f"{genre_slug}/{native_type}"
        if not force:
            entry = load_manifest(manifest_path).get("entries", {}).get(manifest_key)
            if entry and entry.get("prompt_hash") == current_hash and output_path.exists():
                console.print(f"[dim]  {genre_slug}/{native_type} up to date[/dim]")
                continue

        append_event(log_path, event="elicit_native_started", phase=4, type=native_type, genre=genre_slug,
                     native_type=native_type)
        console.print(f"[cyan]  Eliciting {native_type} for {genre_slug}…[/cyan]")

        result_text = client.generate(model=model, prompt=prompt)
        archive_existing(output_path)
        output_path.write_text(result_text)

        digest = compute_content_digest(result_text)
        update_manifest_entry(manifest_path, manifest_key, {
            "prompt_hash": current_hash,
            "content_digest": digest,
            "elicited_at": now_iso(),
            "raw_path": str(output_path),
        })

        append_event(log_path, event="elicit_native_completed", phase=4, type=native_type,
                     genre=genre_slug, native_type=native_type,
                     output=str(output_path.relative_to(output_base)), content_digest=digest)
```

- [ ] **Step 4: Restrict elicit_genre to region-only and remove _load_descriptor_context**

Modify `elicit_genre()` in `genre/commands.py`:

1. Add at the start of `elicit_genre()`, before any processing:
```python
non_region = [c for c in (categories or GENRE_CATEGORIES) if c != "region"]
if non_region:
    raise ValueError(
        f"Categories {non_region} are no longer supported via 'genre elicit'. "
        "Use 'narrative-data discover' for primitive extraction, "
        "'narrative-data primitive' for standalone elicitation, "
        "or 'narrative-data genre elaborate' for genre × primitive elaboration."
    )
```

2. Delete the `_load_descriptor_context()` function entirely.
3. Remove the `descriptor_context = _load_descriptor_context()` line and the `context.update(descriptor_context)` line from `elicit_genre()`.
4. Remove the `resolve_descriptor_dir` import from the imports section.
5. Add new imports needed by `elaborate_genre` and `elicit_native`:
```python
from narrative_data.pipeline.events import append_event
from narrative_data.pipeline.invalidation import archive_existing, compute_content_digest
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_commands.py -v`
Expected: PASS (some existing tests may need adjustment for the region-only restriction)

- [ ] **Step 6: Run full test suite**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/narrative_data/genre/commands.py tests/test_commands.py
git commit -m "feat: add elaborate_genre, elicit_native; restrict genre elicit to region-only"
```

---

### Task 7: CLI Wiring

**Files:**
- Modify: `src/narrative_data/cli.py`
- Modify: `tests/test_cli.py`

Wire new commands into the Click CLI: `discover` subgroup, `primitive` subgroup, `pipeline` subgroup, `genre elaborate`, `genre elicit-native`.

- [ ] **Step 1: Write CLI help tests for new commands**

Follow the existing test pattern in `test_cli.py` which uses `CliRunner` inside a `TestCLI` class:

```python
# Append to tests/test_cli.py TestCLI class

from click.testing import CliRunner
from narrative_data.cli import cli


class TestNewCLICommands:
    def test_discover_extract_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["discover", "extract", "--help"])
        assert result.exit_code == 0
        assert "--type" in result.output

    def test_discover_synthesize_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["discover", "synthesize", "--help"])
        assert result.exit_code == 0
        assert "--type" in result.output
        assert "--cluster" in result.output

    def test_primitive_elicit_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["primitive", "elicit", "--help"])
        assert result.exit_code == 0
        assert "--type" in result.output

    def test_genre_elaborate_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["genre", "elaborate", "--help"])
        assert result.exit_code == 0
        assert "--type" in result.output

    def test_genre_elicit_native_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["genre", "elicit-native", "--help"])
        assert result.exit_code == 0
        assert "--type" in result.output

    def test_pipeline_status_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["pipeline", "status", "--help"])
        assert result.exit_code == 0

    def test_pipeline_approve_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["pipeline", "approve", "--help"])
        assert result.exit_code == 0
        assert "--type" in result.output
        assert "--phase" in result.output
        assert "--primitives" in result.output
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd tools/narrative-data && uv run pytest tests/test_cli.py -v`
Expected: FAIL — new commands don't exist yet

- [ ] **Step 3: Wire CLI commands**

Add to `cli.py`:

1. `discover` subgroup with `extract` and `synthesize` commands
2. `primitive` subgroup with `elicit` and `structure` commands
3. `pipeline` subgroup with `status`, `resume`, and `approve` commands
4. `genre elaborate` command
5. `genre elicit-native` command

Each command follows the existing pattern: parse CLI args, resolve paths, instantiate OllamaClient, delegate to command module. The `pipeline status` command calls `derive_state()` and prints a Rich table. The `pipeline approve` command calls `append_event()` with the review gate data.

- [ ] **Step 4: Run CLI tests to verify they pass**

Run: `cd tools/narrative-data && uv run pytest tests/test_cli.py -v`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All tests PASS

- [ ] **Step 6: Run ruff to check formatting/linting**

Run: `cd tools/narrative-data && uv run ruff check . && uv run ruff format --check .`
Expected: Clean

- [ ] **Step 7: Commit**

```bash
git add src/narrative_data/cli.py tests/test_cli.py
git commit -m "feat: wire discover, primitive, pipeline CLI subgroups and commands"
```

---

### Task 8: Phase 1 Prompt Templates (Archetypes First)

**Files:**
- Create: `prompts/discovery/extract-archetypes.md`
- Create: `prompts/discovery/synthesize-archetypes.md`
- Create: `prompts/primitive/archetypes.md`
- Create: `prompts/genre/elaborate-archetypes.md`

Write the archetype-specific prompt templates for all four phases. Archetypes are first because they're our pipeline validation type.

- [ ] **Step 1: Write Phase 1 extraction prompt**

Create `prompts/discovery/extract-archetypes.md`:

The prompt should ask qwen3.5 to identify 5-8 character archetypes essential or distinctive to the given genre. For each: name, why this genre gives rise to it (grounded in axes/affordances/constraints), distinguishing tension, and overlap signal with other genres. Instruct the model to look beyond conventional lists — examine the genre's exclusions, locus of power, temporal orientation, and state variables for distinct axis intersections. Include `{target_name}` and `{genre_content}` placeholders.

- [ ] **Step 2: Write Phase 2 synthesis prompt**

Create `prompts/discovery/synthesize-archetypes.md`:

The prompt should ask qwen3.5 to synthesize per-genre archetype extractions into ~8-12 distinct archetypes. For each: canonical name, core identity (genre-agnostic), genre variants (axis shifts per genre), uniqueness assessment. Instructions for merging vs. keeping separate. Include `{primitive_type}`, `{cluster_name}`, `{genre_count}`, and `{extractions}` placeholders.

- [ ] **Step 3: Write Phase 3 standalone elicitation prompt**

Create `prompts/primitive/archetypes.md`:

Rich dimensional analysis prompt. Cover: personality axes (warmth, openness, agency, morality, stability, authority, interiority), behavioral patterns, narrative roles and functions, relationship tendencies, narrative promise (what the audience expects when this archetype appears), genre-agnostic invariants, what flexes. Include `{target_name}` placeholder. The `_commentary.md` directive is appended automatically.

- [ ] **Step 4: Write Phase 4a elaboration prompt**

Create `prompts/genre/elaborate-archetypes.md`:

Focus on delta. Given genre description and standalone archetype description, analyze: axis shifts under this genre's constraints, affordance effects (locus of power, temporal orientation, state variables), excluded expressions, genre-specific narrative function. Do not repeat the standalone description. Include `{target_name}` placeholder; genre and primitive descriptions are injected via context.

- [ ] **Step 5: Verify prompts load correctly**

Run: `cd tools/narrative-data && uv run python3 -c "from narrative_data.prompts import PromptBuilder; b = PromptBuilder(); print(b.build_discovery('archetypes', 'Folk Horror', 'test content')[:200])"`
Expected: First 200 chars of the composed prompt

- [ ] **Step 6: Commit**

```bash
git add prompts/discovery/ prompts/primitive/ prompts/genre/elaborate-archetypes.md
git commit -m "feat: add archetype prompt templates for all four pipeline phases"
```

---

### Task 9: Remaining Prompt Templates

**Files:**
- Create: `prompts/discovery/extract-{dynamics,goals,profiles,settings}.md`
- Create: `prompts/discovery/synthesize-{dynamics,goals,profiles,settings}.md`
- Create: `prompts/primitive/{dynamics,goals,profiles,settings}.md`
- Create: `prompts/genre/elaborate-{dynamics,goals,profiles,settings}.md`

Write prompt templates for the remaining four primitive types. Each follows the same structure as archetypes but with type-specific analytical dimensions.

- [ ] **Step 1: Write dynamics prompts (all 4 phases)**

Dynamics focus on: role definitions (role A, role B), power asymmetry, evolution patterns (how the dynamic changes over a story), scene affordances (what scenes this dynamic enables), relational texture. Phase 1 extraction asks for 5-8 interpersonal dynamics. Phase 3 covers axes like symmetry, volatility, dependency, visibility.

- [ ] **Step 2: Write goals prompts (all 4 phases)**

Goals focus on: pursuit expression (how characters pursue this goal), success shape (what achievement looks like), failure shape (what failure looks like), resource interaction (what state variables this goal affects), temporal scope (immediate vs. generational). Phase 1 extraction asks for 5-8 character motivations or pursuits.

- [ ] **Step 3: Write profiles prompts (all 4 phases)**

Profiles (scene types) focus on: scene shape (opening, escalation, resolution pattern), tension signature, characteristic moments, pacing expectations, cast requirements. Phase 1 extraction asks for 5-8 scene types essential to this genre.

- [ ] **Step 4: Write settings prompts (all 4 phases)**

Settings focus on: atmospheric signature, sensory palette, temporal character, narrative function (what this setting enables dramatically), communicability dimensions. Phase 1 extraction asks for 5-8 settings that are essential or distinctive to this genre.

- [ ] **Step 5: Verify all prompts load**

Run: `cd tools/narrative-data && uv run python3 -c "
from narrative_data.prompts import PromptBuilder
b = PromptBuilder()
for t in ['dynamics', 'goals', 'profiles', 'settings']:
    b.build_discovery(t, 'Test Genre', 'content')
    print(f'{t}: OK')
"`
Expected: All four print OK

- [ ] **Step 6: Commit**

```bash
git add prompts/
git commit -m "feat: add prompt templates for dynamics, goals, profiles, and settings"
```

---

### Task 10: Refactor Genre Tropes/Shapes for Phase 4b

**Files:**
- Modify: `prompts/genre/tropes.md`
- Modify: `prompts/genre/narrative-shapes.md`

Remove any references to descriptor context injection from these templates. They now receive only the genre region description as context via `elicit_native()`.

- [ ] **Step 1: Review current tropes.md for descriptor references**

Read `prompts/genre/tropes.md` and identify any text that assumes descriptor JSON is injected.

- [ ] **Step 2: Refactor tropes.md**

Update the template to work with genre region description only. The Genre Integration section questions remain (state variables, locus of power, temporal, exclusions, modifier function) — these are grounded in the genre region description, not descriptors.

- [ ] **Step 3: Refactor narrative-shapes.md**

Same treatment as tropes.md.

- [ ] **Step 4: Run existing tests**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add prompts/genre/tropes.md prompts/genre/narrative-shapes.md
git commit -m "refactor: remove descriptor injection assumptions from tropes and narrative-shapes prompts"
```

---

### Task 11: Pipeline Status and Resume Commands

**Files:**
- Modify: `src/narrative_data/pipeline/events.py` (add format_status)
- Test: `tests/test_events.py` (add status formatting tests)

Implement the status display logic that computes progress per type and phase, and the resume logic that determines next actions.

- [ ] **Step 1: Write tests for status formatting**

```python
# Append to tests/test_events.py
from narrative_data.pipeline.events import format_status
from narrative_data.config import GENRE_CLUSTERS
from narrative_data.genre.commands import GENRE_REGIONS


class TestFormatStatus:
    def test_empty_status(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        status = format_status(log_path, "archetypes")
        assert "Phase 1" in status
        assert "0/30" in status or "0/" in status

    def test_partial_phase1(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        for genre in ["folk-horror", "cosmic-horror"]:
            append_event(log_path, event="extract_completed", phase=1, type="archetypes",
                         genre=genre, output=f"x/{genre}.raw.md", content_digest="sha256:a")
        status = format_status(log_path, "archetypes")
        assert "2/" in status

    def test_blocked_phase3(self, tmp_path):
        log_path = tmp_path / "pipeline.jsonl"
        # Phase 2 complete but no review gate
        for cluster in GENRE_CLUSTERS:
            append_event(log_path, event="synthesize_completed", phase=2, type="archetypes",
                         cluster=cluster, output=f"x/cluster-{cluster}.raw.md",
                         content_digest="sha256:a", primitives_found=8)
        status = format_status(log_path, "archetypes")
        assert "blocked" in status.lower() or "awaiting" in status.lower()
```

- [ ] **Step 2: Implement format_status**

Add `format_status(log_path, primitive_type) -> str` to `events.py`. It calls `derive_state()`, computes totals against known genre/cluster counts, and returns a formatted string showing progress per phase with blocked/complete indicators.

- [ ] **Step 3: Run tests**

Run: `cd tools/narrative-data && uv run pytest tests/test_events.py -v`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/narrative_data/pipeline/events.py tests/test_events.py
git commit -m "feat: add pipeline status formatting and blocked-phase detection"
```

---

### Task 12: End-to-End Validation and Cleanup

**Files:**
- All modified files
- Modify: `tests/test_commands.py` — update any broken tests from genre elicit restriction

Final pass: ensure all tests pass, ruff is clean, and the CLI works end-to-end.

- [ ] **Step 1: Run full test suite**

Run: `cd tools/narrative-data && uv run pytest -v`
Expected: All tests PASS. If any fail from the genre elicit restriction, update them to use `categories=["region"]` or test the new error behavior.

- [ ] **Step 2: Run ruff**

Run: `cd tools/narrative-data && uv run ruff check . && uv run ruff format --check .`
Expected: Clean

- [ ] **Step 3: Verify CLI help for all commands**

Run: `cd tools/narrative-data && uv run narrative-data --help && uv run narrative-data discover --help && uv run narrative-data primitive --help && uv run narrative-data pipeline --help`
Expected: All help text shows correctly with subcommands listed

- [ ] **Step 4: Clean up the test folk-horror/archetypes.raw.md from the aborted test run**

```bash
rm /Users/petetaylor/projects/tasker-systems/storyteller-data/narrative-data/genres/folk-horror/archetypes.raw.md
```

- [ ] **Step 5: Commit any remaining fixes**

```bash
git add -A
git commit -m "chore: test fixes and cleanup for primitive-first pipeline"
```

- [ ] **Step 6: Run `narrative-data pipeline status` to verify it shows clean empty state**

Run: `cd tools/narrative-data && STORYTELLER_DATA_PATH=/Users/petetaylor/projects/tasker-systems/storyteller-data uv run narrative-data pipeline status`
Expected: Shows all phases at 0 progress, Phase 1 ready to start

---

## Execution Notes

- **Tasks 1-7** are the infrastructure — pipeline events, config, PromptBuilder, command modules, CLI wiring. These must be done in order.
- **Tasks 8-9** are prompt authoring — independent of each other but depend on Task 3 (PromptBuilder) and Task 7 (CLI). Task 8 (archetypes) should be done first to validate the template pattern.
- **Task 10** is a small refactor of existing prompts — can run in parallel with Tasks 8-9.
- **Task 11** adds the status/resume display — depends on Task 1 (events) and Task 2 (config).
- **Task 12** is integration validation — must be last.

After this plan completes, the tooling is ready. The next step is to run Phase 1 extraction for archetypes and review the output quality at the first human review gate.

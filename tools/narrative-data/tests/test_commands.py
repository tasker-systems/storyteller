"""Tests for genre, spatial, and cross-pollination command orchestration."""

from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from narrative_data.ollama import OllamaClient  # noqa: F401 (used in fixture)
from narrative_data.utils import now_iso, slug_to_name

# ─────────────────────────────────────────────
# utils tests
# ─────────────────────────────────────────────


class TestSlugToName:
    def test_single_word(self):
        assert slug_to_name("horror") == "Horror"

    def test_hyphenated(self):
        assert slug_to_name("folk-horror") == "Folk Horror"

    def test_three_parts(self):
        assert slug_to_name("high-epic-fantasy") == "High Epic Fantasy"

    def test_already_spaced(self):
        assert slug_to_name("folk horror") == "Folk Horror"


class TestNowIso:
    def test_returns_string(self):
        result = now_iso()
        assert isinstance(result, str)

    def test_contains_utc_offset(self):
        result = now_iso()
        # ISO 8601 with timezone: ends in +00:00 or similar
        assert "+" in result or result.endswith("Z")


# ─────────────────────────────────────────────
# genre command tests
# ─────────────────────────────────────────────


@pytest.fixture
def mock_ollama() -> OllamaClient:
    return MagicMock(spec=OllamaClient)


@pytest.fixture
def genre_output_dir(tmp_path: Path) -> Path:
    d = tmp_path / "genres"
    d.mkdir()
    return d


@pytest.fixture
def manifest_path(tmp_path: Path) -> Path:
    return tmp_path / "genre_manifest.json"


class TestElicitGenre:
    def test_skips_up_to_date_cell(self, mock_ollama, genre_output_dir, manifest_path):
        """A cell with a matching prompt hash is not re-elicited."""
        from narrative_data.genre.commands import elicit_genre
        from narrative_data.pipeline.invalidation import save_manifest

        region_dir = genre_output_dir / "folk-horror"
        region_dir.mkdir()
        raw_path = region_dir / "region.raw.md"
        raw_path.write_text("existing content")

        # Pre-populate manifest with a matching entry.
        manifest = {
            "entries": {
                "folk-horror/region": {
                    "prompt_hash": "matching_hash",
                    "content_digest": "sha256:abc",
                    "elicited_at": "2026-01-01T00:00:00+00:00",
                }
            }
        }
        save_manifest(manifest_path, manifest)

        call_count_before = mock_ollama.generate.call_count

        with patch(
            "narrative_data.genre.commands.compute_prompt_hash",
            return_value="matching_hash",
        ):
            elicit_genre(
                client=mock_ollama,
                output_base=genre_output_dir.parent,
                manifest_path=manifest_path,
                regions=["folk-horror"],
                categories=["region"],
                force=False,
            )

        # Ollama should not have been called since cell is up-to-date
        assert mock_ollama.generate.call_count == call_count_before

    def test_force_re_elicits(self, mock_ollama, genre_output_dir, manifest_path):
        """Force flag causes re-elicitation even when manifest is current."""
        from narrative_data.genre.commands import elicit_genre
        from narrative_data.pipeline.invalidation import save_manifest

        manifest = {
            "entries": {
                "folk-horror/region": {
                    "prompt_hash": "matching_hash",
                    "content_digest": "sha256:abc",
                    "elicited_at": "2026-01-01T00:00:00+00:00",
                }
            }
        }
        save_manifest(manifest_path, manifest)

        mock_ollama.generate.return_value = "# Folk Horror\n\nFresh content."

        with (
            patch(
                "narrative_data.genre.commands.compute_prompt_hash",
                return_value="matching_hash",
            ),
            patch(
                "narrative_data.prompts.PromptBuilder.load_core_prompt",
                return_value="Describe: {target_name}",
            ),
        ):
            elicit_genre(
                client=mock_ollama,
                output_base=genre_output_dir.parent,
                manifest_path=manifest_path,
                regions=["folk-horror"],
                categories=["region"],
                force=True,
            )

        assert mock_ollama.generate.call_count >= 1

    def test_region_ordered_first(self, mock_ollama, genre_output_dir, manifest_path):
        """'region' category is always elicited before dependent categories."""
        from narrative_data.genre.commands import _order_categories

        categories = ["tropes", "region", "archetypes"]
        ordered = _order_categories(categories)
        assert ordered[0] == "region"

    def test_missing_region_elicited(self, mock_ollama, genre_output_dir, manifest_path):
        """A cell with no manifest entry is treated as stale and gets elicited."""
        from narrative_data.genre.commands import elicit_genre

        mock_ollama.generate.return_value = "# Folk Horror\n\nContent."

        with patch(
            "narrative_data.prompts.PromptBuilder.load_core_prompt",
            return_value="Describe: {target_name}",
        ):
            elicit_genre(
                client=mock_ollama,
                output_base=genre_output_dir.parent,
                manifest_path=manifest_path,
                regions=["folk-horror"],
                categories=["region"],
                force=False,
            )

        assert mock_ollama.generate.call_count >= 1
        region_dir = genre_output_dir.parent / "genres" / "folk-horror"
        assert (region_dir / "region.raw.md").exists()


class TestStructureGenre:
    def test_skips_already_structured(self, mock_ollama, genre_output_dir, manifest_path):
        """A cell with a structured .json already present is skipped."""
        from narrative_data.genre.commands import structure_genre
        from narrative_data.pipeline.invalidation import save_manifest

        region_dir = genre_output_dir / "folk-horror"
        region_dir.mkdir()
        (region_dir / "region.raw.md").write_text("content")
        (region_dir / "region.json").write_text('{"structured": true}')

        manifest = {
            "entries": {
                "folk-horror/region": {
                    "prompt_hash": "h",
                    "content_digest": "sha256:abc",
                    "elicited_at": "2026-01-01T00:00:00+00:00",
                    "structured_at": "2026-01-01T00:00:00+00:00",
                }
            }
        }
        save_manifest(manifest_path, manifest)

        call_count_before = mock_ollama.generate_structured.call_count

        structure_genre(
            client=mock_ollama,
            output_base=genre_output_dir.parent,
            manifest_path=manifest_path,
            regions=["folk-horror"],
            categories=["region"],
            force=False,
        )

        assert mock_ollama.generate_structured.call_count == call_count_before

    def test_force_re_structures(self, mock_ollama, genre_output_dir, manifest_path):
        """Force flag causes re-structuring even when .json exists."""
        from narrative_data.genre.commands import structure_genre
        from narrative_data.pipeline.invalidation import save_manifest

        region_dir = genre_output_dir / "folk-horror"
        region_dir.mkdir()
        (region_dir / "region.raw.md").write_text("# Folk Horror\n\nContent.")
        (region_dir / "region.json").write_text('{"old": true}')

        manifest = {
            "entries": {
                "folk-horror/region": {
                    "prompt_hash": "h",
                    "content_digest": "sha256:abc",
                    "elicited_at": "2026-01-01T00:00:00+00:00",
                    "structured_at": "2026-01-01T00:00:00+00:00",
                }
            }
        }
        save_manifest(manifest_path, manifest)

        valid_region = {
            "entity_id": "019d0000-0000-7000-8000-000000000010",
            "name": "Folk Horror",
            "description": "Rural dread",
            "provenance": {
                "prompt_hash": "abc",
                "model": "test",
                "generated_at": "2026-03-17T00:00:00Z",
            },
            "aesthetic": [{"dimension": "spare_ornate", "value": -0.3}],
            "tonal": [{"dimension": "dread_wonder", "value": -0.8}],
            "thematic": [],
            "structural": [],
            "world_affordances": {
                "magic": "subtle",
                "technology": "historical",
                "violence": "consequence-laden",
                "death": "permanent",
                "supernatural": "ambiguous",
            },
        }
        mock_ollama.generate_structured.return_value = valid_region

        structure_genre(
            client=mock_ollama,
            output_base=genre_output_dir.parent,
            manifest_path=manifest_path,
            regions=["folk-horror"],
            categories=["region"],
            force=True,
        )

        assert mock_ollama.generate_structured.call_count >= 1


# ─────────────────────────────────────────────
# spatial command tests
# ─────────────────────────────────────────────


@pytest.fixture
def spatial_output_dir(tmp_path: Path) -> Path:
    d = tmp_path / "spatial"
    d.mkdir()
    return d


class TestElicitSpatial:
    def test_skips_up_to_date_cell(self, mock_ollama, spatial_output_dir, manifest_path):
        """A cell with matching prompt hash is not re-elicited."""
        from narrative_data.pipeline.invalidation import save_manifest
        from narrative_data.spatial.commands import elicit_spatial

        setting_dir = spatial_output_dir / "family-home"
        setting_dir.mkdir()
        (setting_dir / "setting-type.raw.md").write_text("existing")

        manifest = {
            "entries": {
                "family-home/setting-type": {
                    "prompt_hash": "matching_hash",
                    "content_digest": "sha256:abc",
                    "elicited_at": "2026-01-01T00:00:00+00:00",
                }
            }
        }
        save_manifest(manifest_path, manifest)

        call_count_before = mock_ollama.generate.call_count

        with patch(
            "narrative_data.spatial.commands.compute_prompt_hash",
            return_value="matching_hash",
        ):
            elicit_spatial(
                client=mock_ollama,
                output_base=spatial_output_dir.parent,
                manifest_path=manifest_path,
                settings=["family-home"],
                categories=["setting-type"],
                force=False,
            )

        assert mock_ollama.generate.call_count == call_count_before

    def test_dependency_ordering(self):
        """setting-type must come before dependent categories."""
        from narrative_data.spatial.commands import _order_spatial_categories

        categories = ["tonal-inheritance", "setting-type", "topology"]
        ordered = _order_spatial_categories(categories)
        assert ordered[0] == "setting-type"
        assert ordered.index("topology") < ordered.index("tonal-inheritance")

    def test_missing_cell_elicited(self, mock_ollama, spatial_output_dir, manifest_path):
        """A cell with no manifest entry is elicited."""
        from narrative_data.spatial.commands import elicit_spatial

        mock_ollama.generate.return_value = "# Family Home\n\nCozy domestic space."

        with patch(
            "narrative_data.prompts.PromptBuilder.load_core_prompt",
            return_value="Describe: {target_name}",
        ):
            elicit_spatial(
                client=mock_ollama,
                output_base=spatial_output_dir.parent,
                manifest_path=manifest_path,
                settings=["family-home"],
                categories=["setting-type"],
                force=False,
            )

        assert mock_ollama.generate.call_count >= 1
        setting_dir = spatial_output_dir.parent / "spatial" / "family-home"
        assert (setting_dir / "setting-type.raw.md").exists()


class TestBuildSpatialContext:
    def test_no_context_for_setting_type(self, tmp_path: Path):
        """setting-type has no prior stage to inject."""
        from narrative_data.spatial.commands import _build_spatial_context

        setting_dir = tmp_path / "family-home"
        setting_dir.mkdir()
        context = _build_spatial_context(setting_dir, "setting-type")
        assert context == {}

    def test_setting_type_injected_for_place_entities(self, tmp_path: Path):
        """place-entities gets setting-type.json injected."""
        from narrative_data.spatial.commands import _build_spatial_context

        setting_dir = tmp_path / "family-home"
        setting_dir.mkdir()
        (setting_dir / "setting-type.json").write_text('{"name": "Family Home"}')

        context = _build_spatial_context(setting_dir, "place-entities")
        assert "setting-type" in context
        assert "Family Home" in context["setting-type"]

    def test_topology_gets_setting_type_and_place_entities(self, tmp_path: Path):
        """topology gets both setting-type.json and place-entities.json injected."""
        from narrative_data.spatial.commands import _build_spatial_context

        setting_dir = tmp_path / "family-home"
        setting_dir.mkdir()
        (setting_dir / "setting-type.json").write_text('{"name": "Family Home"}')
        (setting_dir / "place-entities.json").write_text('[{"name": "Kitchen"}]')

        context = _build_spatial_context(setting_dir, "topology")
        assert "setting-type" in context
        assert "place-entities" in context

    def test_tonal_inheritance_gets_all_three(self, tmp_path: Path):
        """tonal-inheritance gets setting-type, place-entities, and topology."""
        from narrative_data.spatial.commands import _build_spatial_context

        setting_dir = tmp_path / "family-home"
        setting_dir.mkdir()
        (setting_dir / "setting-type.json").write_text('{"name": "Family Home"}')
        (setting_dir / "place-entities.json").write_text('[{"name": "Kitchen"}]')
        (setting_dir / "topology.json").write_text('[{"edge_id": "e1"}]')

        context = _build_spatial_context(setting_dir, "tonal-inheritance")
        assert "setting-type" in context
        assert "place-entities" in context
        assert "topology" in context


# ─────────────────────────────────────────────
# cross_pollination stub test
# ─────────────────────────────────────────────


class TestCrossPollinationStub:
    def test_prints_readiness_message(self, tmp_path: Path, capsys):
        """Stub prints a readiness message and does not crash."""
        from narrative_data.cross_pollination.commands import run_cross_pollination

        # Should not raise
        run_cross_pollination(output_base=tmp_path)

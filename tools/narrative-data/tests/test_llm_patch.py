# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Tests for pipeline.llm_patch — LLM-assisted field fills for narrative data."""

import json
from pathlib import Path
from unittest.mock import MagicMock

from narrative_data.pipeline.llm_patch import (
    _VALID_VALENCE,
    extract_currencies,
    extract_scale_manifestations,
    extract_valence,
    fill_all_llm_patch,
)

FIXTURES_DIR = Path(__file__).parent / "fixtures"
FOLK_HORROR_MD = (FIXTURES_DIR / "dynamics_folk_horror.md").read_text()


# ---------------------------------------------------------------------------
# extract_valence
# ---------------------------------------------------------------------------


class TestExtractValence:
    def test_returns_valence_from_llm(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "hostile"
        entity = {"canonical_name": "The Debt", "edge_type": "debt-laden", "valence": None}
        result = extract_valence(entity, "source markdown", mock_client)
        assert result["valence"] == "hostile"

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {"canonical_name": "X", "valence": "hostile"}
        result = extract_valence(entity, "irrelevant", mock_client)
        mock_client.generate.assert_not_called()
        assert result["valence"] == "hostile"

    def test_skips_populated_non_none(self):
        """Any non-None, non-empty valence should be preserved."""
        mock_client = MagicMock()
        for val in list(_VALID_VALENCE):
            entity = {"valence": val}
            result = extract_valence(entity, "", mock_client)
            assert result["valence"] == val
        mock_client.generate.assert_not_called()

    def test_does_not_mutate_original(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "nurturing"
        entity = {"canonical_name": "X", "valence": None}
        original_id = id(entity)
        result = extract_valence(entity, "md", mock_client)
        assert id(result) != original_id or result is not entity
        # Original should still have None valence
        assert entity["valence"] is None

    def test_ignores_invalid_llm_response(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "utterly-fictional-value"
        entity = {"canonical_name": "X", "edge_type": "something", "valence": None}
        result = extract_valence(entity, "md", mock_client)
        # Invalid value — valence should remain None (not updated)
        assert result.get("valence") is None

    def test_valid_valence_values_accepted(self):
        """Every value in _VALID_VALENCE should be accepted as LLM output."""
        for val in _VALID_VALENCE:
            mock_client = MagicMock()
            mock_client.generate.return_value = val
            entity = {"canonical_name": "X", "valence": None}
            result = extract_valence(entity, "md", mock_client)
            assert result["valence"] == val, f"Expected {val} to be accepted"

    def test_strips_whitespace_and_lowercases(self):
        mock_client = MagicMock()
        mock_client.generate.return_value = "  Hostile  "
        entity = {"canonical_name": "X", "valence": None}
        result = extract_valence(entity, "md", mock_client)
        assert result["valence"] == "hostile"

    def test_skips_empty_string_valence_is_treated_as_null(self):
        """Empty string valence should trigger LLM (treated as not populated)."""
        mock_client = MagicMock()
        mock_client.generate.return_value = "protective"
        entity = {"canonical_name": "X", "valence": ""}
        result = extract_valence(entity, "md", mock_client)
        assert result["valence"] == "protective"

    def test_calls_llm_with_entity_context(self):
        """The LLM prompt should include edge_type and entity name."""
        mock_client = MagicMock()
        mock_client.generate.return_value = "sacred"
        entity = {
            "canonical_name": "The Pact",
            "edge_type": "binding obligation",
            "role_slots": [{"role": "Binder"}, {"role": "Bound"}],
            "valence": None,
        }
        extract_valence(entity, "some md", mock_client)
        call_args = mock_client.generate.call_args
        prompt = call_args.kwargs.get("prompt") or call_args.args[1]
        assert "The Pact" in prompt
        assert "binding obligation" in prompt


# ---------------------------------------------------------------------------
# extract_currencies
# ---------------------------------------------------------------------------


class TestExtractCurrencies:
    def test_returns_currencies_from_llm(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {"currencies": ["loyalty", "silence"]}
        entity = {
            "canonical_name": "The Debt",
            "edge_type": "debt",
            "currencies": [],
        }
        result = extract_currencies(entity, "source markdown", mock_client)
        assert result["currencies"] == ["loyalty", "silence"]

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {"canonical_name": "X", "currencies": ["existing"]}
        result = extract_currencies(entity, "irrelevant", mock_client)
        mock_client.generate_structured.assert_not_called()
        assert result["currencies"] == ["existing"]

    def test_does_not_mutate_original(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {"currencies": ["debt"]}
        entity = {"canonical_name": "X", "currencies": []}
        original_currencies = entity["currencies"]
        result = extract_currencies(entity, "md", mock_client)
        assert original_currencies == []  # not mutated
        assert result["currencies"] == ["debt"]

    def test_filters_empty_strings(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {"currencies": ["loyalty", "", "  "]}
        entity = {"canonical_name": "X", "currencies": []}
        result = extract_currencies(entity, "md", mock_client)
        assert result["currencies"] == ["loyalty"]

    def test_handles_llm_exception_gracefully(self):
        mock_client = MagicMock()
        mock_client.generate_structured.side_effect = ValueError("LLM parse error")
        entity = {"canonical_name": "X", "currencies": []}
        result = extract_currencies(entity, "md", mock_client)
        # Should return entity unchanged on error
        assert result["currencies"] == []

    def test_handles_empty_currencies_list(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {"currencies": []}
        entity = {"canonical_name": "X", "currencies": []}
        result = extract_currencies(entity, "md", mock_client)
        # Empty list from LLM — no change
        assert result["currencies"] == []

    def test_uses_structured_generate(self):
        """Should call generate_structured, not generate."""
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {"currencies": ["trust"]}
        entity = {"canonical_name": "X", "currencies": []}
        extract_currencies(entity, "md", mock_client)
        mock_client.generate_structured.assert_called_once()
        mock_client.generate.assert_not_called()

    def test_prompt_includes_entity_name(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {"currencies": ["debt"]}
        entity = {"canonical_name": "The Great Bargain", "edge_type": "transactional", "currencies": []}
        extract_currencies(entity, "md", mock_client)
        call_args = mock_client.generate_structured.call_args
        prompt = call_args.kwargs.get("prompt") or call_args.args[1]
        assert "The Great Bargain" in prompt


# ---------------------------------------------------------------------------
# extract_scale_manifestations
# ---------------------------------------------------------------------------


class TestExtractScaleManifestations:
    def test_fills_scale_manifestations(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {
            "orbital": "Long-term theme of power exchange.",
            "arc": "Tension builds over the act.",
            "scene": "Manifests as immediate threat.",
        }
        entity = {
            "canonical_name": "The Contract",
            "edge_type": "binding",
            "scale_manifestations": None,
        }
        result = extract_scale_manifestations(entity, "source md", mock_client)
        assert result["scale_manifestations"]["orbital"] == "Long-term theme of power exchange."
        assert result["scale_manifestations"]["arc"] == "Tension builds over the act."
        assert result["scale_manifestations"]["scene"] == "Manifests as immediate threat."

    def test_skips_populated(self):
        mock_client = MagicMock()
        entity = {
            "canonical_name": "X",
            "scale_manifestations": {"orbital": "already set", "arc": None, "scene": None},
        }
        result = extract_scale_manifestations(entity, "irrelevant", mock_client)
        mock_client.generate_structured.assert_not_called()
        assert result["scale_manifestations"]["orbital"] == "already set"

    def test_fills_when_all_sub_fields_null(self):
        """When scale_manifestations exists but all sub-fields are None, should fill."""
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {
            "orbital": "Some orbital text.",
            "arc": None,
            "scene": None,
        }
        entity = {
            "canonical_name": "X",
            "scale_manifestations": {"orbital": None, "arc": None, "scene": None},
        }
        result = extract_scale_manifestations(entity, "md", mock_client)
        mock_client.generate_structured.assert_called_once()
        assert result["scale_manifestations"]["orbital"] == "Some orbital text."

    def test_fills_when_scale_manifestations_is_none(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {
            "orbital": "Theme.",
            "arc": None,
            "scene": "Immediate.",
        }
        entity = {"canonical_name": "X", "scale_manifestations": None}
        result = extract_scale_manifestations(entity, "md", mock_client)
        assert result["scale_manifestations"]["orbital"] == "Theme."

    def test_does_not_update_when_all_returned_null(self):
        """If LLM returns all nulls, don't overwrite existing None entry."""
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {
            "orbital": None,
            "arc": None,
            "scene": None,
        }
        entity = {"canonical_name": "X", "scale_manifestations": None}
        result = extract_scale_manifestations(entity, "md", mock_client)
        # No non-null values returned — should leave scale_manifestations as None
        assert result.get("scale_manifestations") is None

    def test_does_not_mutate_original(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {
            "orbital": "Theme.",
            "arc": None,
            "scene": None,
        }
        entity = {"canonical_name": "X", "scale_manifestations": None}
        result = extract_scale_manifestations(entity, "md", mock_client)
        assert entity["scale_manifestations"] is None
        assert result["scale_manifestations"] is not None

    def test_handles_exception_gracefully(self):
        mock_client = MagicMock()
        mock_client.generate_structured.side_effect = ValueError("error")
        entity = {"canonical_name": "X", "scale_manifestations": None}
        result = extract_scale_manifestations(entity, "md", mock_client)
        assert result.get("scale_manifestations") is None

    def test_uses_structured_generate(self):
        mock_client = MagicMock()
        mock_client.generate_structured.return_value = {
            "orbital": "text",
            "arc": None,
            "scene": None,
        }
        entity = {"canonical_name": "X", "scale_manifestations": None}
        extract_scale_manifestations(entity, "md", mock_client)
        mock_client.generate_structured.assert_called_once()
        mock_client.generate.assert_not_called()


# ---------------------------------------------------------------------------
# fill_all_llm_patch — orchestration
# ---------------------------------------------------------------------------


class TestFillAllLlmPatch:
    def _make_corpus(self, tmp_path: Path) -> Path:
        """Build minimal corpus with dynamics JSON and markdown."""
        corpus = tmp_path / "narrative-data"
        dynamics_dir = corpus / "discovery" / "dynamics"
        dynamics_dir.mkdir(parents=True)

        # Source markdown (use real fixture for realistic section matching)
        (dynamics_dir / "folk-horror.md").write_text(FOLK_HORROR_MD)

        # JSON with sparse fields
        entities = [
            {
                "canonical_name": "Blood-Line Contract",
                "genre_slug": "folk-horror",
                "edge_type": "Information-symmetric within the kin, asymmetric to the outsider.",
                "directionality": "bidirectional_asymmetric",
                "scale": "orbital",
                "spans_scales": [],
                "currencies": [],
                "valence": None,
                "role_slots": [{"role": "The Lineage"}, {"role": "The Ancestors"}],
                "scale_manifestations": {"orbital": None, "arc": None, "scene": None},
            }
        ]
        (dynamics_dir / "folk-horror.json").write_text(json.dumps(entities))

        return corpus

    def _make_mock_client(self) -> MagicMock:
        """Create a mock client with sensible return values."""
        mock_client = MagicMock()
        mock_client.generate.return_value = "protective"
        mock_client.generate_structured.side_effect = [
            {"currencies": ["obligation", "ancestry"]},
            {"orbital": "Generational trap.", "arc": None, "scene": None},
        ]
        return mock_client

    def test_processes_dynamics_files(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        mock_client = self._make_mock_client()
        summary = fill_all_llm_patch(corpus, mock_client, types=["dynamics"], genres=None, dry_run=False)
        assert "dynamics" in summary
        assert summary["dynamics"]["files_processed"] == 1

    def test_entities_updated_reported(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        mock_client = self._make_mock_client()
        summary = fill_all_llm_patch(corpus, mock_client, types=["dynamics"], genres=None, dry_run=False)
        assert summary["dynamics"]["entities_updated"] >= 1

    def test_dry_run_does_not_write(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        mock_client = self._make_mock_client()
        json_path = corpus / "discovery" / "dynamics" / "folk-horror.json"
        original = json_path.read_text()

        fill_all_llm_patch(corpus, mock_client, types=["dynamics"], genres=None, dry_run=True)

        assert json_path.read_text() == original

    def test_dry_run_still_reports_changes(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        mock_client = self._make_mock_client()
        summary = fill_all_llm_patch(corpus, mock_client, types=["dynamics"], genres=None, dry_run=True)
        assert summary["dynamics"]["entities_updated"] >= 1

    def test_writes_updated_json(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        mock_client = MagicMock()
        mock_client.generate.return_value = "hostile"
        mock_client.generate_structured.side_effect = [
            {"currencies": ["fear"]},
            {"orbital": "Eternal.", "arc": None, "scene": None},
        ]
        json_path = corpus / "discovery" / "dynamics" / "folk-horror.json"

        fill_all_llm_patch(corpus, mock_client, types=["dynamics"], genres=None, dry_run=False)

        written = json.loads(json_path.read_text())
        assert len(written) == 1
        assert written[0]["valence"] == "hostile"

    def test_genre_filter_limits_files(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        # Add a second genre file that should be excluded
        extra = [{"canonical_name": "X", "valence": None, "currencies": [], "scale_manifestations": None}]
        (corpus / "discovery" / "dynamics" / "cosmic-horror.json").write_text(json.dumps(extra))

        mock_client = self._make_mock_client()
        summary = fill_all_llm_patch(
            corpus, mock_client, types=["dynamics"], genres=["folk-horror"], dry_run=False
        )
        assert summary["dynamics"]["files_processed"] == 1

    def test_unsupported_type_skipped(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        mock_client = MagicMock()
        summary = fill_all_llm_patch(
            corpus, mock_client, types=["spatial-topology"], genres=None, dry_run=False
        )
        # spatial-topology not supported in llm_patch — should not appear
        assert "spatial-topology" not in summary

    def test_missing_type_dir_returns_zero_summary(self, tmp_path: Path):
        corpus = tmp_path / "narrative-data"
        corpus.mkdir(parents=True)
        mock_client = MagicMock()
        summary = fill_all_llm_patch(corpus, mock_client, types=["dynamics"], genres=None, dry_run=False)
        assert summary["dynamics"]["files_processed"] == 0
        assert summary["dynamics"]["entities_updated"] == 0

    def test_summary_keys_present(self, tmp_path: Path):
        corpus = self._make_corpus(tmp_path)
        mock_client = self._make_mock_client()
        summary = fill_all_llm_patch(corpus, mock_client, types=["dynamics"], genres=None, dry_run=False)
        result = summary["dynamics"]
        assert "files_processed" in result
        assert "entities_updated" in result
        assert "entities_skipped" in result

    def test_skips_already_filled_entities(self, tmp_path: Path):
        """Entities with all fields already populated should not call LLM."""
        corpus = tmp_path / "narrative-data"
        dynamics_dir = corpus / "discovery" / "dynamics"
        dynamics_dir.mkdir(parents=True)
        (dynamics_dir / "folk-horror.md").write_text("")
        fully_filled = [
            {
                "canonical_name": "Full Entity",
                "valence": "hostile",
                "currencies": ["debt", "shame"],
                "scale_manifestations": {
                    "orbital": "Orbital text.",
                    "arc": "Arc text.",
                    "scene": "Scene text.",
                },
            }
        ]
        (dynamics_dir / "folk-horror.json").write_text(json.dumps(fully_filled))

        mock_client = MagicMock()
        fill_all_llm_patch(corpus, mock_client, types=["dynamics"], genres=None, dry_run=False)
        mock_client.generate.assert_not_called()
        mock_client.generate_structured.assert_not_called()

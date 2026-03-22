"""Tests for the structure CLI subcommand."""

from unittest.mock import patch

from click.testing import CliRunner

from narrative_data.cli import cli


class TestStructureCLI:
    def test_structure_group_exists(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["structure", "--help"])
        assert result.exit_code == 0
        assert "structure" in result.output.lower()

    def test_structure_run_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["structure", "run", "--help"])
        assert result.exit_code == 0
        assert "TYPE_SLUG" in result.output or "type_slug" in result.output.lower()
        assert "--genre" in result.output
        assert "--all" in result.output
        assert "--clusters" in result.output
        assert "--force" in result.output
        assert "--plan" in result.output

    def test_unknown_type_exits_with_error(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["structure", "run", "nonexistent", "--all"])
        assert result.exit_code != 0
        assert "Unknown type" in result.output or "Unknown type" in (result.stderr or "")

    def test_no_flags_exits_with_error(self):
        runner = CliRunner()
        with (
            patch("narrative_data.config.resolve_output_path"),
            patch("narrative_data.ollama.OllamaClient"),
        ):
            result = runner.invoke(cli, ["structure", "run", "archetypes"])
        assert result.exit_code != 0 or "Specify" in result.output

    @patch("narrative_data.pipeline.structure_commands.structure_type")
    @patch("narrative_data.ollama.OllamaClient")
    @patch("narrative_data.config.resolve_output_path")
    def test_all_genres_calls_structure_type(self, mock_path, mock_client, mock_st, tmp_path):
        mock_path.return_value = tmp_path
        runner = CliRunner()
        result = runner.invoke(cli, ["structure", "run", "archetypes", "--all"])
        # Should not fail with "Unknown type"
        assert "Unknown type" not in result.output

    @patch("narrative_data.pipeline.structure_commands.structure_type")
    @patch("narrative_data.ollama.OllamaClient")
    @patch("narrative_data.config.resolve_output_path")
    def test_single_genre_calls_structure_type(self, mock_path, mock_client, mock_st, tmp_path):
        mock_path.return_value = tmp_path
        runner = CliRunner()
        result = runner.invoke(cli, ["structure", "run", "archetypes", "--genre", "horror"])
        assert "Unknown type" not in result.output

    @patch("narrative_data.pipeline.structure_commands.structure_clusters")
    @patch("narrative_data.ollama.OllamaClient")
    @patch("narrative_data.config.resolve_output_path")
    def test_clusters_calls_structure_clusters(self, mock_path, mock_client, mock_sc, tmp_path):
        mock_path.return_value = tmp_path
        runner = CliRunner()
        result = runner.invoke(cli, ["structure", "run", "archetypes", "--clusters"])
        assert "Unknown type" not in result.output

    def test_all_valid_type_slugs_recognized(self):
        """Verify that all expected type slugs pass the TYPE_REGISTRY check."""
        from narrative_data.pipeline.structure_commands import TYPE_REGISTRY

        valid_types = [
            "genre-dimensions",
            "archetypes",
            "dynamics",
            "goals",
            "profiles",
            "settings",
            "ontological-posture",
            "archetype-dynamics",
            "spatial-topology",
            "place-entities",
            "tropes",
            "narrative-shapes",
        ]
        for slug in valid_types:
            assert slug in TYPE_REGISTRY, f"{slug} missing from TYPE_REGISTRY"

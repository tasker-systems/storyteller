"""Tests for CLI invocation via Click test runner."""

from click.testing import CliRunner

from narrative_data.cli import cli


class TestCLI:
    def test_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["--help"])
        assert result.exit_code == 0
        assert "genre" in result.output

    def test_genre_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["genre", "--help"])
        assert result.exit_code == 0
        assert "elicit" in result.output
        assert "structure" in result.output

    def test_spatial_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["spatial", "--help"])
        assert result.exit_code == 0
        assert "elicit" in result.output

    def test_status_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["status", "--help"])
        assert result.exit_code == 0

    def test_list_help(self):
        runner = CliRunner()
        result = runner.invoke(cli, ["list", "--help"])
        assert result.exit_code == 0

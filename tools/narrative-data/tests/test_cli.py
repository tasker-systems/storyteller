# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

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

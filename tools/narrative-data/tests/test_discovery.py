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
        client.generate.return_value = (
            "# Extracted archetypes for Folk Horror\n\n- The Outsider\n- The Land-Keeper"
        )
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        prompts_dir = _make_mock_prompts(output_base)
        genre_dir = output_base / "genres" / "folk-horror"
        genre_dir.mkdir(parents=True)
        (genre_dir / "region.md").write_text("Folk horror region description...")

        extract_primitives(
            client=client,
            output_base=output_base,
            log_path=log_path,
            primitive_type="archetypes",
            genres=["folk-horror"],
            prompts_dir=prompts_dir,
        )

        out_file = output_base / "discovery" / "archetypes" / "folk-horror.md"
        assert out_file.exists()
        assert "Outsider" in out_file.read_text()

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
        client.generate.return_value = (
            "# Cluster Synthesis: Horror Archetypes\n\n1. The Outsider..."
        )
        output_base = tmp_output_dir
        log_path = output_base / "pipeline.jsonl"
        prompts_dir = _make_mock_prompts(output_base)
        disc_dir = output_base / "discovery" / "archetypes"
        disc_dir.mkdir(parents=True)
        (disc_dir / "folk-horror.md").write_text("Folk horror archetypes...")
        (disc_dir / "cosmic-horror.md").write_text("Cosmic horror archetypes...")

        synthesize_cluster(
            client=client,
            output_base=output_base,
            log_path=log_path,
            primitive_type="archetypes",
            cluster_name="horror",
            genres=["folk-horror", "cosmic-horror"],
            prompts_dir=prompts_dir,
        )

        out_file = disc_dir / "cluster-horror.md"
        assert out_file.exists()

        events = read_events(log_path)
        assert len(events) == 2
        assert events[0]["event"] == "synthesize_started"
        assert events[1]["event"] == "synthesize_completed"
        assert events[1]["cluster"] == "horror"

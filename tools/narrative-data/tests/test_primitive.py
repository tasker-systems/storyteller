# tests/test_primitive.py
from unittest.mock import MagicMock

from narrative_data.pipeline.events import read_events
from narrative_data.primitive.commands import elicit_primitives


def _make_mock_prompts(tmp_path):
    """Create minimal prompt templates for testing."""
    prompts_dir = tmp_path / "prompts"
    (prompts_dir / "primitive").mkdir(parents=True)
    (prompts_dir / "primitive" / "archetypes.md").write_text("Analyze the archetype {target_name}.")
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
        """Primitives list comes from the review gate — only listed ones elicited."""
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

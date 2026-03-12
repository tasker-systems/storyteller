"""Tests for goal lexicon enrichment."""

from goal_lexicon.enrich import build_enrichment_prompt


def test_build_prompt_includes_goal_info():
    goal = {
        "id": "protect_secret",
        "description": "Keep information hidden",
        "category": "protection",
        "visibility": "Hidden",
        "valence": "tense",
    }
    prompt = build_enrichment_prompt(goal, [], [], [])
    assert "protect_secret" in prompt
    assert "protection" in prompt
    assert "behavioral lexicon" in prompt.lower()


def test_build_prompt_includes_relevant_descriptors():
    goal = {
        "id": "protect_secret",
        "description": "Keep information hidden",
        "category": "protection",
        "visibility": "Hidden",
        "valence": "tense",
    }
    profiles = [{"id": "quiet_reunion", "description": "A gentle meeting"}]
    archetypes = [{"id": "stoic_survivor", "description": "Endures without complaint"}]
    dynamics = [{"id": "strangers_in_shared_grief", "description": "Bound by loss"}]

    prompt = build_enrichment_prompt(goal, profiles, archetypes, dynamics)
    assert "quiet_reunion" in prompt
    assert "stoic_survivor" in prompt
    assert "strangers_in_shared_grief" in prompt

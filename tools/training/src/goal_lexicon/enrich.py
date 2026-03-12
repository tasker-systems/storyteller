"""Generate behavioral lexicons for goals via LLM (Ollama)."""

import json
from pathlib import Path
from typing import Any

import httpx


def load_descriptors(descriptor_dir: Path) -> dict[str, Any]:
    """Load all descriptor files from the given directory."""
    result = {}
    for name in ["goals", "profiles", "archetypes", "dynamics", "genres"]:
        path = descriptor_dir / f"{name}.json"
        if path.exists():
            with open(path) as f:
                result[name] = json.load(f)
    return result


def build_enrichment_prompt(
    goal: dict, profiles: list[dict], archetypes: list[dict], dynamics: list[dict]
) -> str:
    """Build the prompt for generating behavioral lexicon entries for a goal."""
    prompt = f"""Generate behavioral lexicon entries for this narrative goal.

Goal: {goal['id']}
Description: {goal['description']}
Category: {goal['category']}
Visibility: {goal['visibility']}
Valence: {goal['valence']}

Relevant profiles (scene types where this goal appears):
"""
    for p in profiles:
        prompt += f"- {p['id']}: {p['description']}\n"

    prompt += "\nRelevant archetypes (character types who can pursue this goal):\n"
    for a in archetypes:
        prompt += f"- {a['id']}: {a['description']}\n"

    prompt += "\nRelevant dynamics (relationships that enable this goal):\n"
    for d in dynamics:
        prompt += f"- {d['id']}: {d['description']}\n"

    prompt += """
Generate 15-25 behavioral lexicon entries. Each entry describes what pursuing this goal LOOKS LIKE —
observable behavior, speech patterns, physical tells, relational moves. NOT abstract atmosphere.

For each entry, specify:
- fragment: the behavioral description (1-2 sentences)
- register: "character_signal" (primary), "atmospheric", or "transitional"
- dimensional_context: which archetypes/profiles/dynamics this fragment fits best
  (use null for wildcard)

Respond with valid JSON:
{
  "entries": [
    {
      "fragment": "...",
      "register": "character_signal",
      "dimensional_context": {
        "archetypes": ["archetype_id"] or null,
        "profiles": ["profile_id"] or null,
        "dynamics": ["dynamic_id"] or null,
        "valence": ["heavy", "tense"]
      }
    }
  ]
}"""
    return prompt


def enrich_goal(
    goal: dict,
    descriptors: dict[str, Any],
    model: str = "qwen2.5:32b-instruct",
    base_url: str = "http://localhost:11434",
) -> list[dict]:
    """Generate lexicon entries for a single goal via Ollama."""
    profiles = descriptors.get("profiles", {}).get("profiles", [])
    archetypes = descriptors.get("archetypes", {}).get("archetypes", [])
    dynamics = descriptors.get("dynamics", {}).get("dynamics", [])

    relevant_profiles = [p for p in profiles if goal["id"] in p.get("scene_goals", [])]
    relevant_archetypes = [a for a in archetypes if goal["id"] in a.get("pursuable_goals", [])]
    relevant_dynamics = [d for d in dynamics if goal["id"] in d.get("enabled_goals", [])]

    prompt = build_enrichment_prompt(
        goal, relevant_profiles, relevant_archetypes, relevant_dynamics
    )

    for attempt in range(3):
        try:
            response = httpx.post(
                f"{base_url}/api/generate",
                json={
                    "model": model,
                    "prompt": prompt,
                    "stream": False,
                    "options": {"temperature": 0.8, "num_predict": 2000},
                },
                timeout=600.0,
            )
            response.raise_for_status()
            break
        except httpx.ReadTimeout:
            if attempt < 2:
                print(f"  Timeout (attempt {attempt + 1}/3), retrying...")
                continue
            print(f"  Warning: timed out after 3 attempts for {goal['id']}")
            return []

    text = response.json()["response"]

    try:
        start = text.index("{")
        end = text.rindex("}") + 1
        data = json.loads(text[start:end])
        return data.get("entries", [])
    except (ValueError, json.JSONDecodeError):
        print(f"  Warning: failed to parse LLM output for {goal['id']}")
        return []


def enrich_all_goals(
    descriptor_dir: Path,
    model: str = "qwen2.5:32b-instruct",
    base_url: str = "http://localhost:11434",
) -> None:
    """Enrich all goals in goals.json with behavioral lexicons."""
    descriptors = load_descriptors(descriptor_dir)
    goals_path = descriptor_dir / "goals.json"

    with open(goals_path) as f:
        goals_data = json.load(f)

    for i, goal in enumerate(goals_data["goals"]):
        if goal.get("lexicon"):
            print(f"Skipping: {goal['id']} (already has {len(goal['lexicon'])} entries)")
            continue
        print(f"Enriching: {goal['id']} ({i + 1}/{len(goals_data['goals'])})...")
        entries = enrich_goal(goal, descriptors, model, base_url)
        goal["lexicon"] = entries
        print(f"  Generated {len(entries)} entries")

        # Save after each goal so progress survives interruption
        with open(goals_path, "w") as f:
            json.dump(goals_data, f, indent=2)

    print(f"\nDone. Enriched {len(goals_data['goals'])} goals.")

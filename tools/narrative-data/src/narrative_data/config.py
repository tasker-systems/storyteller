"""Configuration and data path resolution."""

import os
from pathlib import Path

GENRE_CATEGORIES: list[str] = [
    "region", "archetypes", "tropes", "narrative-shapes",
    "dynamics", "profiles", "goals", "settings",
]

SPATIAL_CATEGORIES: list[str] = [
    "setting-type", "place-entities", "topology", "tonal-inheritance",
]

ELICITATION_MODEL = "qwen3.5:35b"
STRUCTURING_MODEL = "qwen2.5:3b-instruct"
OLLAMA_BASE_URL = "http://localhost:11434"
ELICITATION_TIMEOUT = 600.0
STRUCTURING_TIMEOUT = 120.0


def resolve_data_path() -> Path:
    path = os.environ.get("STORYTELLER_DATA_PATH")
    if not path:
        raise RuntimeError(
            "STORYTELLER_DATA_PATH environment variable is not set. "
            "Set it to the path of the storyteller-data repository."
        )
    return Path(path)


def resolve_output_path() -> Path:
    return resolve_data_path() / "narrative-data"


def resolve_descriptor_dir() -> Path:
    return resolve_data_path() / "training-data" / "descriptors"

# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Model registry and fan-out data types for the Tome elicitation pipeline."""

from dataclasses import dataclass
from typing import Any

from narrative_data.config import CREATIVE_MODEL, ELICITATION_MODEL, STRUCTURING_MODEL

MODEL_REGISTRY: dict[str, str] = {
    "fan_out_structured": STRUCTURING_MODEL,
    "fan_out_creative": CREATIVE_MODEL,
    "coherence": ELICITATION_MODEL,
}


def get_model(role: str) -> str:
    """Return the model name for the given role.

    Raises KeyError if the role is not registered.
    """
    return MODEL_REGISTRY[role]


@dataclass
class FanOutSpec:
    """Specification for a single fan-out elicitation instance."""

    stage: str
    index: int
    template_name: str
    model_role: str
    context: dict[str, Any]

    @property
    def output_filename(self) -> str:
        """Return the output filename, 1-indexed and zero-padded to 3 digits."""
        return f"instance-{self.index + 1:03d}.json"

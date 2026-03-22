# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.

"""Prompt loader and compositional builder for the two-stage pipeline."""

import json
from pathlib import Path

_PACKAGE_DIR = Path(__file__).parent.parent.parent
_DEFAULT_PROMPTS_DIR = _PACKAGE_DIR / "prompts"


class PromptBuilder:
    """Loads markdown prompt templates and composes them with dynamic context."""

    def __init__(self, prompts_dir: Path = _DEFAULT_PROMPTS_DIR):
        self.prompts_dir = prompts_dir

    def load_core_prompt(self, domain: str, category: str) -> str:
        path = self.prompts_dir / domain / f"{category}.md"
        if not path.exists():
            raise FileNotFoundError(f"Prompt template not found: {path}")
        return path.read_text()

    def _load_commentary_directive(self) -> str:
        path = self.prompts_dir / "_commentary.md"
        if path.exists():
            return path.read_text()
        return ""

    def build_stage1(
        self,
        domain: str,
        category: str,
        target_name: str,
        context: dict[str, str] | None = None,
    ) -> str:
        core = self.load_core_prompt(domain, category)
        prompt = core.replace("{target_name}", target_name)

        if context:
            prompt += "\n\n---\n\n## Additional Context\n\n"
            for label, content in context.items():
                prompt += f"### {label}\n\n{content}\n\n"

        prompt += self._load_commentary_directive()
        return prompt

    def build_discovery(
        self,
        primitive_type: str,
        target_name: str,
        genre_content: str,
    ) -> str:
        """Build a Phase 1 discovery extraction prompt."""
        template_path = self.prompts_dir / "discovery" / f"extract-{primitive_type}.md"
        if not template_path.exists():
            raise FileNotFoundError(f"Discovery prompt template not found: {template_path}")
        prompt = template_path.read_text()
        prompt = prompt.replace("{target_name}", target_name)
        prompt = prompt.replace("{genre_content}", genre_content)
        prompt += "\n\n" + self._load_commentary_directive()
        return prompt

    def build_synthesis(
        self,
        primitive_type: str,
        cluster_name: str,
        extractions: dict[str, str],
    ) -> str:
        """Build a Phase 2 cluster synthesis prompt."""
        template_path = self.prompts_dir / "discovery" / f"synthesize-{primitive_type}.md"
        if not template_path.exists():
            raise FileNotFoundError(f"Synthesis prompt template not found: {template_path}")
        prompt = template_path.read_text()
        prompt = prompt.replace("{primitive_type}", primitive_type)
        prompt = prompt.replace("{cluster_name}", cluster_name)
        prompt = prompt.replace("{genre_count}", str(len(extractions)))
        extraction_text = ""
        for genre_slug, content in extractions.items():
            extraction_text += f"### {genre_slug}\n\n{content}\n\n"
        prompt = prompt.replace("{extractions}", extraction_text)
        prompt += "\n\n" + self._load_commentary_directive()
        return prompt

    def build_structure(
        self,
        structure_type: str,
        raw_content: str,
        schema: dict,
    ) -> str:
        """Build a type-specific structuring prompt for the 7b model."""
        template_path = self.prompts_dir / "structure" / f"{structure_type}.md"
        if not template_path.exists():
            raise FileNotFoundError(f"Structure prompt template not found: {template_path}")
        prompt = template_path.read_text()
        prompt = prompt.replace("{raw_content}", raw_content)
        prompt = prompt.replace("{schema}", json.dumps(schema, indent=2))
        return prompt

    def build_segment_structure(
        self,
        segment_type: str,
        raw_content: str,
        schema: dict,
    ) -> str:
        """Build a segment-level structuring prompt."""
        template_path = self.prompts_dir / "structure" / "segments" / f"{segment_type}.md"
        if not template_path.exists():
            raise FileNotFoundError(f"Segment prompt template not found: {template_path}")
        prompt = template_path.read_text()
        prompt = prompt.replace("{raw_content}", raw_content)
        prompt = prompt.replace("{schema}", json.dumps(schema, indent=2))
        return prompt

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

    @staticmethod
    def build_stage2(raw_content: str, schema: dict) -> str:
        schema_str = json.dumps(schema, indent=2)
        return f"""Given the following content:
---
{raw_content}
---

Produce JSON matching this schema:
{schema_str}

Rules:
- Preserve all substantive information from the source
- Map evaluative notes to the commentary and suggestions fields
- Do not invent information not present in the source
- If a field cannot be populated from the source, use null"""

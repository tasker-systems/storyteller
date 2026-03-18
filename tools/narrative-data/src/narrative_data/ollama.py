"""Thin httpx client for Ollama API."""

import json
from typing import Any

import httpx

from narrative_data.config import ELICITATION_TIMEOUT, OLLAMA_BASE_URL, STRUCTURING_TIMEOUT


class OllamaClient:
    """Client for Ollama's /api/generate endpoint."""

    def __init__(self, base_url: str = OLLAMA_BASE_URL):
        self.base_url = base_url

    def generate(
        self,
        model: str,
        prompt: str,
        timeout: float = ELICITATION_TIMEOUT,
        temperature: float = 0.8,
        max_retries: int = 3,
    ) -> str:
        """Stage 1: Generate raw text. Returns the response string."""
        assert max_retries >= 1, "max_retries must be >= 1"
        for attempt in range(max_retries):
            try:
                response = httpx.post(
                    f"{self.base_url}/api/generate",
                    json={
                        "model": model,
                        "prompt": prompt,
                        "stream": False,
                        "options": {"temperature": temperature},
                    },
                    timeout=timeout,
                )
                response.raise_for_status()
                return response.json()["response"]
            except httpx.ReadTimeout:
                if attempt < max_retries - 1:
                    continue
                raise
        raise RuntimeError("unreachable")

    def generate_structured(
        self,
        model: str,
        prompt: str,
        schema: dict[str, Any],
        timeout: float = STRUCTURING_TIMEOUT,
        temperature: float = 0.1,
        max_retries: int = 3,
    ) -> dict[str, Any]:
        """Stage 2: Generate structured JSON. Returns parsed dict."""
        assert max_retries >= 1, "max_retries must be >= 1"
        for attempt in range(max_retries):
            try:
                response = httpx.post(
                    f"{self.base_url}/api/generate",
                    json={
                        "model": model,
                        "prompt": prompt,
                        "stream": False,
                        "format": schema,
                        "options": {"temperature": temperature},
                    },
                    timeout=timeout,
                )
                response.raise_for_status()
                text = response.json()["response"]
                return json.loads(text)
            except httpx.ReadTimeout:
                if attempt < max_retries - 1:
                    continue
                raise
        raise RuntimeError("unreachable")

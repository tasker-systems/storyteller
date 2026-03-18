"""Tests for Ollama client (httpx mocked)."""

from unittest.mock import patch

from narrative_data.ollama import OllamaClient


class TestOllamaClient:
    def test_default_config(self):
        client = OllamaClient()
        assert client.base_url == "http://localhost:11434"

    def test_custom_config(self):
        client = OllamaClient(base_url="http://gpu-box:11434")
        assert client.base_url == "http://gpu-box:11434"

    def test_generate(self):
        client = OllamaClient()
        mock_response = {"response": "# Folk Horror\n\nA genre rooted in rural dread..."}
        with patch("httpx.post") as mock_post:
            mock_post.return_value.json.return_value = mock_response
            mock_post.return_value.raise_for_status = lambda: None
            result = client.generate(model="qwen3.5:35b", prompt="Describe folk horror")
        assert "Folk Horror" in result
        call_json = mock_post.call_args[1]["json"]
        assert call_json["model"] == "qwen3.5:35b"
        assert call_json["stream"] is False

    def test_generate_structured(self):
        client = OllamaClient()
        mock_response = {"response": '{"name": "Folk Horror", "description": "Rural dread"}'}
        with patch("httpx.post") as mock_post:
            mock_post.return_value.json.return_value = mock_response
            mock_post.return_value.raise_for_status = lambda: None
            result = client.generate_structured(
                model="qwen2.5:3b-instruct",
                prompt="Structure this content",
                schema={"type": "object", "properties": {"name": {"type": "string"}}},
            )
        assert result["name"] == "Folk Horror"
        call_json = mock_post.call_args[1]["json"]
        assert "format" in call_json

    def test_generate_timeout_retry(self):
        client = OllamaClient()
        from unittest.mock import MagicMock

        import httpx as httpx_mod
        success_response = MagicMock()
        success_response.json.return_value = {"response": "ok"}
        success_response.raise_for_status = MagicMock()
        with patch("httpx.post") as mock_post:
            mock_post.side_effect = [httpx_mod.ReadTimeout("timeout"), success_response]
            result = client.generate(model="test", prompt="test")
        assert result == "ok"
        assert mock_post.call_count == 2

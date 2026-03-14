//! Simulated player for automated playtest runs.
//!
//! Uses a direct Ollama HTTP call (`/api/chat`) to generate player responses
//! to narrator output. The CLI must NOT depend on `storyteller-engine`
//! (which pulls in bevy, ort, candle, etc.), so this is a standalone minimal
//! client rather than a re-use of `ExternalServerProvider`.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Simulates a player character responding to narrator output.
#[derive(Debug)]
pub struct PlayerSimulation {
    client: reqwest::Client,
    ollama_url: String,
    model: String,
    system_prompt: String,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

impl PlayerSimulation {
    pub fn new(ollama_url: &str, model: &str, protagonist_context: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");

        let system_prompt = format!(
            "You are playing a character in an interactive story. Stay in character and respond \
             naturally to what the narrator describes. Keep responses to 1-3 sentences — you are \
             a player giving input, not writing prose.\n\n\
             Your character:\n{protagonist_context}"
        );

        Self {
            client,
            ollama_url: ollama_url.to_string(),
            model: model.to_string(),
            system_prompt,
        }
    }

    /// Extract protagonist context from the `composition_json` in the `SceneComposed` event.
    ///
    /// Parses the JSON to find the named character and returns a short context
    /// string for the simulation system prompt.
    pub fn build_protagonist_context(protagonist_name: &str, composition_json: &str) -> String {
        if let Ok(composition) = serde_json::from_str::<serde_json::Value>(composition_json) {
            if let Some(characters) = composition.get("characters").and_then(|c| c.as_array()) {
                for character in characters {
                    let name = character.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name == protagonist_name {
                        let performance_notes = character
                            .get("performance_notes")
                            .and_then(|p| p.as_str())
                            .unwrap_or("");
                        let backstory = character
                            .get("backstory")
                            .and_then(|b| b.as_str())
                            .unwrap_or("");
                        return format!(
                            "Name: {name}\n{performance_notes}\n\nBackstory: {backstory}"
                        );
                    }
                }
            }
        }
        format!("Name: {protagonist_name}")
    }

    /// Generate player input in response to the latest narrator output.
    pub async fn generate_input(
        &self,
        narrator_output: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: self.system_prompt.clone(),
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: format!("The narrator says:\n\n{narrator_output}\n\nWhat do you do?"),
                },
            ],
            stream: false,
            options: OllamaOptions {
                temperature: 0.9,
                num_predict: 200,
            },
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.ollama_url))
            .json(&request)
            .send()
            .await?
            .json::<OllamaChatResponse>()
            .await?;

        Ok(response.message.content.trim().to_string())
    }
}

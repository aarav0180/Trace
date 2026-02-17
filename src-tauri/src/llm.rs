use crate::settings::Settings;
use reqwest::Client;
use serde_json::json;

/// Unified LLM client that dispatches to OpenAI, Anthropic, or Google APIs.
pub struct LlmClient {
    http: Client,
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            http: Client::new(),
        }
    }

    /// Send a prompt to the active cloud provider and return the response text.
    pub async fn prompt(
        &self,
        settings: &Settings,
        system: &str,
        user_msg: &str,
    ) -> Result<String, String> {
        let api_key = settings
            .active_key()
            .ok_or("No API key configured for the active provider")?;

        match settings.active_provider.as_str() {
            "openai" => self.prompt_openai(api_key, &settings.active_model, system, user_msg).await,
            "anthropic" => self.prompt_anthropic(api_key, &settings.active_model, system, user_msg).await,
            "google" => self.prompt_google(api_key, &settings.active_model, system, user_msg).await,
            other => Err(format!("Unknown provider: {}", other)),
        }
    }

    async fn prompt_openai(
        &self,
        api_key: &str,
        model: &str,
        system: &str,
        user_msg: &str,
    ) -> Result<String, String> {
        let body = json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user_msg }
            ],
            "temperature": 0.2,
            "max_tokens": 4096
        });

        let resp = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {}", e))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

        data["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Unexpected OpenAI response format: {}", data))
    }

    async fn prompt_anthropic(
        &self,
        api_key: &str,
        model: &str,
        system: &str,
        user_msg: &str,
    ) -> Result<String, String> {
        let body = json!({
            "model": model,
            "max_tokens": 4096,
            "system": system,
            "messages": [
                { "role": "user", "content": user_msg }
            ]
        });

        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Anthropic request failed: {}", e))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Anthropic response: {}", e))?;

        data["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Unexpected Anthropic response format: {}", data))
    }

    async fn prompt_google(
        &self,
        api_key: &str,
        model: &str,
        system: &str,
        user_msg: &str,
    ) -> Result<String, String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let body = json!({
            "system_instruction": {
                "parts": [{ "text": system }]
            },
            "contents": [{
                "parts": [{ "text": user_msg }]
            }],
            "generationConfig": {
                "temperature": 0.2,
                "maxOutputTokens": 4096
            }
        });

        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Google AI request failed: {}", e))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Google AI response: {}", e))?;

        data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Unexpected Google AI response format: {}", data))
    }
}

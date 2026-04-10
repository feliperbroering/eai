use anyhow::{Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    llm::{LlmClient, parse_response, render_prompt},
    types::{CommandRequest, GeneratedCommand},
};

pub struct OpenAiCompatClient {
    label: String,
    http_client: Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl OpenAiCompatClient {
    pub fn new(
        label: &str,
        http_client: Client,
        base_url: String,
        model: String,
        api_key: String,
    ) -> Self {
        Self {
            label: label.to_string(),
            http_client,
            base_url,
            model,
            api_key,
        }
    }

    async fn post(&self, system: &str, user: &str) -> Result<String> {
        let endpoint = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let response = self
            .http_client
            .post(&endpoint)
            .bearer_auth(&self.api_key)
            .json(&ChatCompletionRequest {
                model: self.model.clone(),
                temperature: 0.0,
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: system.to_string(),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: user.to_string(),
                    },
                ],
            })
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 404 {
                bail!(
                    "model '{}' not found on {} — make sure -m matches a model available on this backend",
                    self.model, self.label
                );
            }
            if status.as_u16() == 401 {
                bail!(
                    "authentication failed for {} — check your API key",
                    self.label
                );
            }
            let body = response.text().await.unwrap_or_default();
            bail!("{} returned HTTP {} — {}", self.label, status, body);
        }

        let parsed = response.json::<ChatCompletionResponse>().await?;

        Ok(parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default())
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatClient {
    async fn call(&self, system: &str, user: &str) -> Result<String> {
        self.post(system, user).await
    }

    async fn generate_command(&self, request: &CommandRequest) -> Result<GeneratedCommand> {
        let (system, user) = render_prompt(request);
        let content = self.post(&system, &user).await?;
        let parsed = parse_response(&content);
        if parsed.command.is_empty() {
            bail!("{} returned an empty command", self.label);
        }
        Ok(parsed)
    }

    fn label(&self) -> String {
        format!("{}/{}", self.label, self.model)
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    temperature: f32,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

use anyhow::{Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    llm::{LlmClient, parse_response, render_prompt},
    types::{CommandRequest, GeneratedCommand},
};

pub struct OllamaClient {
    http_client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(http_client: Client, base_url: String, model: String) -> Self {
        Self {
            http_client,
            base_url,
            model,
        }
    }

    async fn post(&self, system: &str, user: &str) -> Result<String> {
        let endpoint = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let response = self
            .http_client
            .post(endpoint)
            .json(&OllamaGenerateRequest {
                model: self.model.clone(),
                prompt: user.to_string(),
                system: system.to_string(),
                stream: false,
                options: OllamaOptions { temperature: 0.0 },
            })
            .send()
            .await?
            .error_for_status()?
            .json::<OllamaGenerateResponse>()
            .await?;
        Ok(response.response)
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn call(&self, system: &str, user: &str) -> Result<String> {
        self.post(system, user).await
    }

    async fn generate_command(&self, request: &CommandRequest) -> Result<GeneratedCommand> {
        let (system, prompt) = render_prompt(request);
        let raw = self.post(&system, &prompt).await?;
        let parsed = parse_response(&raw);
        if parsed.command.is_empty() {
            bail!("ollama returned an empty command");
        }
        Ok(parsed)
    }

    fn label(&self) -> String {
        format!("ollama/{}", self.model)
    }
}

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

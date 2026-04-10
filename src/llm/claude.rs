use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use tokio::process::Command;

use crate::{
    llm::{LlmClient, parse_response, render_prompt},
    types::{CommandRequest, GeneratedCommand},
};

pub struct ClaudeCliClient {
    model: Option<String>,
}

impl ClaudeCliClient {
    pub fn new(model: Option<String>) -> Self {
        Self { model }
    }

    async fn run(&self, system: &str, user: &str) -> Result<String> {
        let mut command = Command::new("claude");
        command
            .arg("-p")
            .arg(user)
            .arg("--system-prompt")
            .arg(system);
        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }
        let output = command
            .output()
            .await
            .context("failed to execute `claude -p`")?;
        if !output.status.success() {
            bail!(
                "claude exited with {}",
                output.status.code().unwrap_or_default()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[async_trait]
impl LlmClient for ClaudeCliClient {
    async fn call(&self, system: &str, user: &str) -> Result<String> {
        self.run(system, user).await
    }

    async fn generate_command(&self, request: &CommandRequest) -> Result<GeneratedCommand> {
        let (system, user) = render_prompt(request);
        let stdout = self.run(&system, &user).await?;
        let parsed = parse_response(&stdout);
        if parsed.command.is_empty() {
            bail!("claude returned an empty command");
        }
        Ok(parsed)
    }

    fn label(&self) -> String {
        match &self.model {
            Some(model) => format!("claude-cli/{model}"),
            None => "claude-cli".to_string(),
        }
    }
}

mod claude;
mod ollama;
mod openai;

use std::{env, process::Stdio, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tokio::{process::Command, time::sleep};
use which::which;

use crate::{
    config::AppConfig,
    types::{BackendKind, CommandRequest, GeneratedCommand},
};

pub use claude::ClaudeCliClient;
pub use ollama::OllamaClient;
pub use openai::OpenAiCompatClient;

const OLLAMA_INSTALL_SCRIPT_URL: &str = "https://ollama.com/install.sh";
const MAX_GENERATIONS: usize = 5;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn call(&self, system: &str, user: &str) -> Result<String>;
    async fn generate_command(&self, request: &CommandRequest) -> Result<GeneratedCommand>;
    fn label(&self) -> String;
}

pub struct Backend {
    client: Box<dyn LlmClient>,
}

impl Backend {
    pub fn new(client: Box<dyn LlmClient>) -> Self {
        Self { client }
    }

    pub fn label(&self) -> String {
        self.client.label()
    }

    pub async fn call(&self, system: &str, user: &str) -> Result<String> {
        self.client.call(system, user).await
    }

    pub async fn generate_command(&self, request: &CommandRequest) -> Result<GeneratedCommand> {
        self.client.generate_command(request).await
    }
}

pub async fn resolve_backend(
    http_client: Client,
    config: &AppConfig,
    requested_backend: Option<BackendKind>,
    requested_model: Option<&str>,
) -> Result<Backend> {
    if let Some(backend) = requested_backend.or_else(|| config.default.backend.into_backend_kind())
    {
        return build_backend(http_client, config, backend, requested_model).await;
    }

    if env_var(&config.groq.api_key_env).is_some() {
        return build_backend(http_client, config, BackendKind::Groq, requested_model).await;
    }

    match build_ollama_backend(
        http_client.clone(),
        config,
        requested_model.unwrap_or(config.ollama.model.as_str()),
        true,
    )
    .await
    {
        Ok(backend) => return Ok(backend),
        Err(error) => {
            eprintln!("warning: could not prepare Ollama automatically: {error}");
        }
    }

    if env_var(&config.openai.api_key_env).is_some() && !config.openai.base_url.trim().is_empty() {
        return build_backend(http_client, config, BackendKind::Openai, requested_model).await;
    }

    if which("claude").is_ok() {
        return build_backend(http_client, config, BackendKind::ClaudeCli, requested_model).await;
    }

    bail!(
        "no backend available; start Ollama, set {}, configure {}, or install Claude CLI",
        config.groq.api_key_env,
        config.openai.api_key_env
    )
}

async fn build_backend(
    http_client: Client,
    config: &AppConfig,
    backend: BackendKind,
    requested_model: Option<&str>,
) -> Result<Backend> {
    let backend = match backend {
        BackendKind::Ollama => {
            build_ollama_backend(
                http_client,
                config,
                requested_model.unwrap_or(config.ollama.model.as_str()),
                true,
            )
            .await?
        }
        BackendKind::Groq => build_openai_compat(
            "groq",
            http_client,
            &config.groq.api_key_env,
            &config.groq.base_url,
            requested_model.unwrap_or(&config.groq.model),
        )?,
        BackendKind::Openai => build_openai_compat(
            "openai",
            http_client,
            &config.openai.api_key_env,
            &config.openai.base_url,
            requested_model.unwrap_or(&config.openai.model),
        )?,
        BackendKind::ClaudeCli => {
            let allow_mock = env::var("EAI_MOCK_CLAUDE").ok().as_deref() == Some("1");
            if !allow_mock {
                which("claude").map_err(|_| anyhow!("claude was not found in PATH"))?;
            }
            let client = ClaudeCliClient::new(requested_model.map(str::to_string));
            Backend::new(Box::new(client))
        }
    };

    Ok(backend)
}

fn build_openai_compat(
    label: &str,
    http_client: Client,
    api_key_env: &str,
    base_url: &str,
    model: &str,
) -> Result<Backend> {
    let api_key = env_var(api_key_env).ok_or_else(|| anyhow!("{api_key_env} is not set"))?;
    if base_url.trim().is_empty() {
        bail!("{label} base_url is empty in config");
    }
    let client = OpenAiCompatClient::new(
        label,
        http_client,
        base_url.to_string(),
        model.to_string(),
        api_key,
    );
    Ok(Backend::new(Box::new(client)))
}

async fn build_ollama_backend(
    http_client: Client,
    config: &AppConfig,
    model: &str,
    install_if_missing: bool,
) -> Result<Backend> {
    ensure_ollama_ready(&http_client, &config.ollama.url, model, install_if_missing).await?;
    let client = OllamaClient::new(http_client, config.ollama.url.clone(), model.to_string());
    Ok(Backend::new(Box::new(client)))
}

async fn ensure_ollama_ready(
    client: &Client,
    url: &str,
    model: &str,
    install_if_missing: bool,
) -> Result<()> {
    ensure_ollama_installed(install_if_missing).await?;
    ensure_ollama_reachable(client, url).await?;
    ensure_ollama_model(client, model, url).await
}

async fn ensure_ollama_installed(install_if_missing: bool) -> Result<()> {
    if which("ollama").is_ok() {
        return Ok(());
    }

    if !install_if_missing {
        bail!("ollama is not installed");
    }

    if !matches!(env::consts::OS, "macos" | "linux") {
        bail!(
            "automatic Ollama install is only supported on macOS and Linux, found {}",
            env::consts::OS
        );
    }

    eprintln!("▶ Ollama not found. Installing via official installer...");

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "OLLAMA_NO_START=1 curl -fsSL {OLLAMA_INSTALL_SCRIPT_URL} | sh"
        ))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .context("failed to launch the Ollama installer")?;

    if !status.success() {
        bail!(
            "automatic Ollama installation failed with exit {}",
            status.code().unwrap_or_default()
        );
    }

    which("ollama")
        .map_err(|_| anyhow!("Ollama installer finished but `ollama` is not in PATH"))?;
    Ok(())
}

async fn ensure_ollama_reachable(client: &Client, url: &str) -> Result<()> {
    if ollama_reachable(client, url).await? {
        return Ok(());
    }

    let _child = Command::new("ollama")
        .arg("serve")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    for _ in 0..10 {
        sleep(Duration::from_millis(500)).await;
        if ollama_reachable(client, url).await? {
            return Ok(());
        }
    }

    bail!(
        "ollama is not reachable at {}; start it with `ollama serve`",
        url
    )
}

async fn ensure_ollama_model(client: &Client, model: &str, url: &str) -> Result<()> {
    if ollama_has_model(client, url, model).await? {
        return Ok(());
    }

    eprintln!("▶ Pulling Ollama model {model}...");

    let status = Command::new("ollama")
        .arg("pull")
        .arg(model)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .with_context(|| format!("failed to pull Ollama model {model}"))?;

    if !status.success() {
        bail!(
            "ollama pull {} failed with exit {}",
            model,
            status.code().unwrap_or_default()
        );
    }

    if !ollama_has_model(client, url, model).await? {
        bail!("ollama finished pulling {model}, but the model is still unavailable");
    }

    Ok(())
}

async fn ollama_reachable(client: &Client, url: &str) -> Result<bool> {
    let endpoint = format!("{}/api/tags", url.trim_end_matches('/'));
    let response = client.get(endpoint).send().await;

    match response {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

async fn ollama_has_model(client: &Client, url: &str, model: &str) -> Result<bool> {
    let endpoint = format!("{}/api/tags", url.trim_end_matches('/'));
    let response = client
        .get(endpoint)
        .send()
        .await?
        .error_for_status()?
        .json::<OllamaTagsResponse>()
        .await?;

    Ok(response.models.into_iter().any(|entry| entry.name == model))
}

pub fn env_var(key: &str) -> Option<String> {
    read_key_from_shell_profile(key).or_else(|| env::var(key).ok().filter(|v| !v.trim().is_empty()))
}

/// Fallback: parse the export line from the user's shell profile
/// so `eai` works immediately after `eai setup` without sourcing.
fn read_key_from_shell_profile(key: &str) -> Option<String> {
    let home = dirs::home_dir()?;
    if cfg!(windows) {
        for profile in windows_powershell_profiles(&home) {
            let Ok(contents) = std::fs::read_to_string(&profile) else {
                continue;
            };
            for line in contents.lines().rev() {
                if let Some(val) = parse_powershell_env_line(line.trim(), key)
                    && !val.is_empty()
                {
                    return Some(val);
                }
            }
        }
        return None;
    }

    let shell = env::var("SHELL").unwrap_or_default();
    let shell_name = shell.rsplit('/').next().unwrap_or("zsh");

    let profiles: Vec<std::path::PathBuf> = match shell_name {
        "fish" => vec![home.join(".config/fish/config.fish")],
        "bash" => vec![home.join(".bash_profile"), home.join(".bashrc")],
        _ => vec![home.join(".zshrc")],
    };

    let export_marker = format!("{key}=");
    let fish_marker = format!("set -gx {key} ");

    for profile in profiles {
        let Ok(contents) = std::fs::read_to_string(&profile) else {
            continue;
        };
        for line in contents.lines().rev() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("export ") {
                if let Some(val) = rest.strip_prefix(&export_marker) {
                    let val = val.trim_matches('"').trim_matches('\'');
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            } else if trimmed.starts_with(&fish_marker) {
                let val = trimmed
                    .strip_prefix(&fish_marker)?
                    .trim_matches('"')
                    .trim_matches('\'');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }

    None
}

fn windows_powershell_profiles(home: &std::path::Path) -> Vec<std::path::PathBuf> {
    let documents = home.join("Documents");
    vec![
        documents
            .join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
        documents
            .join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
    ]
}

fn parse_powershell_env_line(line: &str, key: &str) -> Option<String> {
    let marker = format!("$env:{key}");
    if !line.starts_with(&marker) {
        return None;
    }

    let (_, value_part) = line.split_once('=')?;
    let value = value_part
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();
    Some(value)
}

pub fn render_prompt(request: &CommandRequest) -> (String, String) {
    let system = build_system_prompt(request);
    let user = build_user_prompt(request);
    (system, user)
}

pub fn generation_limit() -> usize {
    MAX_GENERATIONS
}

fn build_system_prompt(request: &CommandRequest) -> String {
    let mut lines = vec![
        format!(
            "You are a shell command generator for {} ({}).",
            request.os, request.shell
        ),
        "Respond with exactly two lines:".to_string(),
        "Line 1: the shell command (raw, no backticks, no markdown fences)".to_string(),
        "Line 2: a brief casual explanation starting with // (like a code comment)".to_string(),
        String::new(),
        "Example:".to_string(),
        r#"docker ps --format "table {{.Names}}\t{{.Status}}""#.to_string(),
        "// lists containers showing name and status columns".to_string(),
        String::new(),
        "Rules:".to_string(),
        "- First line is ONLY the raw command. Second line is ONLY the // explanation.".to_string(),
        "- Keep the explanation short, casual, and helpful — like a senior dev commenting their code".to_string(),
        "- If multiple commands are needed, chain with && or ; on the first line".to_string(),
        format!(
            "- Use tools and flags available on {} ({}) only.",
            request.os, request.shell
        ),
        "- Assume tools mentioned by name are installed.".to_string(),
        "- Prefer safe inspection commands over destructive ones unless explicitly asked.".to_string(),
    ];

    if request.tool_docs.is_some() {
        lines.push(
            "- ONLY use flags that appear in the tool documentation below. NEVER invent or guess flags."
                .to_string(),
        );
    }

    if request.context.is_some() {
        lines.push(
            "- If a prior attempt failed, use the error output and user feedback to fix the command."
                .to_string(),
        );
    }

    if request.search_results.is_some() {
        lines.push(
            "- Use the web search notes when they clarify syntax or platform-specific behavior."
                .to_string(),
        );
    }

    if request.stdin_data.is_some() {
        lines.push(
            "- Piped data from stdin is provided. Use it to understand the data format and structure."
                .to_string(),
        );
        lines.push(
            "- Generate a command that works on the ORIGINAL source, not one that reads from stdin."
                .to_string(),
        );
    }

    lines.join("\n")
}

fn build_user_prompt(request: &CommandRequest) -> String {
    let mut sections = vec![];

    if let Some(tool_docs) = &request.tool_docs {
        sections.push(format!(
            "Tool documentation (use ONLY these flags):\n{tool_docs}"
        ));
    }

    sections.push(format!("Request: {}", request.prompt));

    if !request.history.is_empty() {
        let history = request
            .history
            .iter()
            .map(|entry| {
                format!(
                    "- {} => {} ({})",
                    entry.prompt, entry.command, entry.exit_code
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!("Recent history:\n{history}"));
    }

    if let Some(search_results) = &request.search_results {
        sections.push(format!("Web search notes:\n{search_results}"));
    }

    if let Some(stdin_data) = &request.stdin_data {
        sections.push(format!("Piped input data (stdin):\n```\n{stdin_data}\n```"));
    }

    if let Some(context) = &request.context {
        sections.push(format!("Feedback context:\n{context}"));
    }

    sections.join("\n\n")
}

pub fn parse_response(raw: &str) -> GeneratedCommand {
    let text = raw.trim();

    let text = text
        .strip_prefix("```bash\n")
        .or_else(|| text.strip_prefix("```sh\n"))
        .or_else(|| text.strip_prefix("```\n"))
        .unwrap_or(text);
    let text = text
        .strip_suffix("\n```")
        .or_else(|| text.strip_suffix("```"))
        .unwrap_or(text);
    let text = text.trim();

    let mut command = String::new();
    let mut explanation = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("//") && !command.is_empty() {
            let explain = trimmed.trim_start_matches('/').trim();
            if !explain.is_empty() {
                explanation = Some(explain.to_string());
            }
        } else if command.is_empty() {
            command = trimmed
                .strip_prefix("$ ")
                .unwrap_or(trimmed)
                .trim_matches('`')
                .trim()
                .to_string();
        }
    }

    GeneratedCommand {
        command,
        explanation,
    }
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModelTag>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelTag {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::parse_response;

    #[test]
    fn strips_code_fences() {
        let parsed = parse_response("```bash\nrg TODO .\n```");
        assert_eq!(parsed.command, "rg TODO .");
        assert!(parsed.explanation.is_none());
    }

    #[test]
    fn parses_command_with_explanation() {
        let parsed =
            parse_response("docker ps --all\n// lists all containers including stopped ones");
        assert_eq!(parsed.command, "docker ps --all");
        assert_eq!(
            parsed.explanation.as_deref(),
            Some("lists all containers including stopped ones")
        );
    }

    #[test]
    fn handles_raw_command_only() {
        let parsed = parse_response("head -n 20 readme.md");
        assert_eq!(parsed.command, "head -n 20 readme.md");
        assert!(parsed.explanation.is_none());
    }
}

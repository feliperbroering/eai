use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::types::{BackendKind, ShellKind};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub default: DefaultConfig,

    #[serde(default)]
    pub ollama: OllamaConfig,

    #[serde(default)]
    pub groq: GroqConfig,

    #[serde(default)]
    pub openai: OpenAiConfig,

    #[serde(default)]
    pub gemini: GeminiConfig,

    #[serde(default, rename = "claude-cli")]
    pub claude_cli: ClaudeCliConfig,

    #[serde(default)]
    pub search: SearchConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        let config = toml::from_str::<Self>(&contents)
            .with_context(|| format!("failed to parse config at {}", path.display()))?;
        Ok(config)
    }

    pub fn ensure_config_file() -> Result<PathBuf> {
        let path = config_path()?;
        if path.exists() {
            return Ok(path);
        }

        ensure_parent(&path)?;
        let contents = toml::to_string_pretty(&Self::default())?;
        fs::write(&path, contents)
            .with_context(|| format!("failed to write config at {}", path.display()))?;
        Ok(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultConfig {
    #[serde(default)]
    pub backend: BackendPreference,

    #[serde(default)]
    pub shell: Option<ShellKind>,

    #[serde(default = "default_confirm")]
    pub confirm: bool,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            backend: BackendPreference::Auto,
            shell: None,
            confirm: true,
        }
    }
}

fn default_confirm() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendPreference {
    #[default]
    Auto,
    Ollama,
    Groq,
    Openai,
    Gemini,
    ClaudeCli,
}

impl BackendPreference {
    pub fn into_backend_kind(self) -> Option<BackendKind> {
        match self {
            Self::Auto => None,
            Self::Ollama => Some(BackendKind::Ollama),
            Self::Groq => Some(BackendKind::Groq),
            Self::Openai => Some(BackendKind::Openai),
            Self::Gemini => Some(BackendKind::Gemini),
            Self::ClaudeCli => Some(BackendKind::ClaudeCli),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_model")]
    pub model: String,

    #[serde(default = "default_ollama_url")]
    pub url: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            model: default_ollama_model(),
            url: default_ollama_url(),
        }
    }
}

fn default_ollama_model() -> String {
    "qwen3:4b".to_string()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqConfig {
    #[serde(default = "default_groq_api_key_env")]
    pub api_key_env: String,

    #[serde(default = "default_groq_model")]
    pub model: String,

    #[serde(default = "default_groq_base_url")]
    pub base_url: String,
}

impl Default for GroqConfig {
    fn default() -> Self {
        Self {
            api_key_env: default_groq_api_key_env(),
            model: default_groq_model(),
            base_url: default_groq_base_url(),
        }
    }
}

fn default_groq_api_key_env() -> String {
    "GROQ_API_KEY".to_string()
}

fn default_groq_model() -> String {
    "llama-3.3-70b-versatile".to_string()
}

fn default_groq_base_url() -> String {
    "https://api.groq.com/openai/v1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    #[serde(default = "default_openai_api_key_env")]
    pub api_key_env: String,

    #[serde(default = "default_openai_base_url")]
    pub base_url: String,

    #[serde(default = "default_openai_model")]
    pub model: String,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_key_env: default_openai_api_key_env(),
            base_url: default_openai_base_url(),
            model: default_openai_model(),
        }
    }
}

fn default_openai_api_key_env() -> String {
    "OPENAI_API_KEY".to_string()
}

fn default_openai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_openai_model() -> String {
    "gpt-4o-mini".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    #[serde(default = "default_gemini_api_key_env")]
    pub api_key_env: String,

    #[serde(default = "default_gemini_base_url")]
    pub base_url: String,

    #[serde(default = "default_gemini_model")]
    pub model: String,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key_env: default_gemini_api_key_env(),
            base_url: default_gemini_base_url(),
            model: default_gemini_model(),
        }
    }
}

fn default_gemini_api_key_env() -> String {
    "GEMINI_API_KEY".to_string()
}

fn default_gemini_base_url() -> String {
    "https://generativelanguage.googleapis.com/v1beta/openai".to_string()
}

fn default_gemini_model() -> String {
    "gemini-2.5-flash-lite".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeCliConfig {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_search_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub engine: SearchEngine,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            engine: SearchEngine::default(),
        }
    }
}

fn default_search_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngine {
    #[default]
    Ddg,
    Tavily,
}

pub fn config_path() -> Result<PathBuf> {
    if let Ok(override_dir) = env::var("EAI_CONFIG_DIR")
        && !override_dir.trim().is_empty()
    {
        return Ok(PathBuf::from(override_dir).join("eai").join("config.toml"));
    }
    let base = dirs::config_dir().ok_or_else(|| anyhow!("failed to resolve config dir"))?;
    Ok(base.join("eai").join("config.toml"))
}

pub fn history_path() -> Result<PathBuf> {
    if let Ok(override_dir) = env::var("EAI_DATA_DIR")
        && !override_dir.trim().is_empty()
    {
        return Ok(PathBuf::from(override_dir)
            .join("eai")
            .join("history.jsonl"));
    }
    let base = dirs::data_local_dir().ok_or_else(|| anyhow!("failed to resolve data dir"))?;
    Ok(base.join("eai").join("history.jsonl"))
}

pub fn ensure_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    Ok(())
}

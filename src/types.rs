use std::{borrow::Cow, env, fmt};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    Ollama,
    Groq,
    Openai,
    ClaudeCli,
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Ollama => "ollama",
            Self::Groq => "groq",
            Self::Openai => "openai",
            Self::ClaudeCli => "claude-cli",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ShellKind {
    Zsh,
    Bash,
    Fish,
    Sh,
}

impl ShellKind {
    pub fn detect() -> Self {
        let shell = env::var("SHELL").unwrap_or_default();
        let shell = shell.rsplit('/').next().unwrap_or_default();

        match shell {
            "bash" => Self::Bash,
            "fish" => Self::Fish,
            "sh" => Self::Sh,
            _ => Self::Zsh,
        }
    }

    pub fn program(self) -> &'static str {
        match self {
            Self::Zsh => "zsh",
            Self::Bash => "bash",
            Self::Fish => "fish",
            Self::Sh => "sh",
        }
    }

    pub fn command_flag(self) -> &'static str {
        match self {
            Self::Fish => "-c",
            Self::Zsh | Self::Bash | Self::Sh => "-lc",
        }
    }
}

impl fmt::Display for ShellKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.program())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsKind {
    Macos,
    Linux,
    Windows,
    Unknown,
}

impl OsKind {
    pub fn detect() -> Self {
        match env::consts::OS {
            "macos" => Self::Macos,
            "linux" => Self::Linux,
            "windows" => Self::Windows,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for OsKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Windows => "windows",
            Self::Unknown => "unknown",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub ts: String,
    pub prompt: String,
    pub command: String,
    pub exit_code: i32,
    pub backend: String,
    pub iterations: usize,
}

#[derive(Debug, Clone)]
pub struct CommandRequest {
    pub prompt: String,
    pub shell: ShellKind,
    pub os: OsKind,
    pub context: Option<String>,
    pub search_results: Option<String>,
    pub tool_docs: Option<String>,
    pub history: Vec<HistoryEntry>,
    pub stdin_data: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GeneratedCommand {
    pub command: String,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    pub fn is_empty(&self) -> bool {
        self.stdout.trim().is_empty() && self.stderr.trim().is_empty()
    }

    pub fn stdout_head(&self) -> Cow<'_, str> {
        truncate_for_context(&self.stdout, 700)
    }

    pub fn stderr_head(&self) -> Cow<'_, str> {
        truncate_for_context(&self.stderr, 700)
    }
}

fn truncate_for_context(input: &str, max_len: usize) -> Cow<'_, str> {
    if input.chars().count() <= max_len {
        return Cow::Borrowed(input);
    }

    let truncated: String = input.chars().take(max_len).collect();
    Cow::Owned(format!("{truncated}..."))
}

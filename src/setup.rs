use std::{env, fs, io::Write};

use anyhow::{Context, Result, bail};
use console::style;
use dialoguer::{Input, Select};
use reqwest::Client;

use crate::{
    config::{self, AppConfig, BackendPreference},
    ui,
};

// ── provider registry ──────────────────────────────────────────────────────

#[allow(dead_code)]
struct Provider {
    name: &'static str,
    tag: &'static str,
    base_url: &'static str,
    default_model: &'static str,
    api_key_env: &'static str,
    signup_url: &'static str,
    steps: &'static [&'static str],
    target: ConfigTarget,
}

#[derive(Clone, Copy)]
enum ConfigTarget {
    Groq,
    Openai,
    Ollama,
    Custom,
}

static PROVIDERS: &[Provider] = &[
    Provider {
        name: "Groq",
        tag: "★ free · fast · recommended",
        base_url: "https://api.groq.com/openai/v1",
        default_model: "llama-3.3-70b-versatile",
        api_key_env: "GROQ_API_KEY",
        signup_url: "https://console.groq.com",
        steps: &[
            "Open  https://console.groq.com",
            "Sign up with GitHub or Google (takes 10 seconds)",
            "Go to  API Keys → Create API Key",
            "Copy the key and paste it below",
        ],
        target: ConfigTarget::Groq,
    },
    Provider {
        name: "OpenRouter",
        tag: "1 key → GPT-4o, Claude, Llama, Gemini...",
        base_url: "https://openrouter.ai/api/v1",
        default_model: "meta-llama/llama-3.3-70b-instruct",
        api_key_env: "OPENROUTER_API_KEY",
        signup_url: "https://openrouter.ai/keys",
        steps: &[
            "Open  https://openrouter.ai/keys",
            "Sign up → Create Key",
            "Copy the key and paste it below",
        ],
        target: ConfigTarget::Openai,
    },
    Provider {
        name: "OpenAI",
        tag: "gpt-4o-mini",
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-4o-mini",
        api_key_env: "OPENAI_API_KEY",
        signup_url: "https://platform.openai.com/api-keys",
        steps: &[
            "Open  https://platform.openai.com/api-keys",
            "Create a new secret key",
            "Copy and paste it below",
        ],
        target: ConfigTarget::Openai,
    },
    Provider {
        name: "Ollama",
        tag: "local · free · no API key",
        base_url: "",
        default_model: "qwen2.5:3b",
        api_key_env: "",
        signup_url: "https://ollama.com",
        steps: &[
            "Install from  https://ollama.com  (or  brew install ollama )",
            "eai will auto-download the model on first run",
        ],
        target: ConfigTarget::Ollama,
    },
    Provider {
        name: "Custom API",
        tag: "GLM, Kimi, Together, Fireworks...",
        base_url: "",
        default_model: "",
        api_key_env: "",
        signup_url: "",
        steps: &[],
        target: ConfigTarget::Custom,
    },
];

// ── first-run detection ────────────────────────────────────────────────────

pub fn needs_setup() -> bool {
    if let Ok(path) = config::config_path()
        && path.exists()
    {
        return false;
    }

    let has = |k: &str| env::var(k).ok().filter(|v| !v.trim().is_empty()).is_some();

    if has("GROQ_API_KEY") || has("OPENAI_API_KEY") || has("OPENROUTER_API_KEY") {
        return false;
    }
    if which::which("ollama").is_ok() || which::which("claude").is_ok() {
        return false;
    }

    true
}

// ── main entry ─────────────────────────────────────────────────────────────

pub async fn run_setup() -> Result<()> {
    eprintln!();
    eprintln!(
        "  {} {}",
        style("⚡").cyan().bold(),
        style("Let's connect your AI in 30 seconds.").white().bold()
    );
    eprintln!();

    let items: Vec<String> = PROVIDERS
        .iter()
        .map(|p| format!("{:<16}{}", p.name, style(p.tag).dim()))
        .collect();

    let idx = Select::new()
        .with_prompt(format!("  {} Pick a provider", style("›").cyan()))
        .items(&items)
        .default(0)
        .interact()?;

    let provider = &PROVIDERS[idx];

    eprintln!();
    eprintln!(
        "  {} {}",
        style("━━━").cyan(),
        style(format!("Setting up {}", provider.name)).bold()
    );
    eprintln!();

    match provider.target {
        ConfigTarget::Custom => setup_custom().await,
        ConfigTarget::Ollama => setup_ollama(provider),
        _ => setup_api_provider(provider).await,
    }
}

// ── provider-specific flows ────────────────────────────────────────────────

async fn setup_api_provider(provider: &Provider) -> Result<()> {
    for (i, step) in provider.steps.iter().enumerate() {
        eprintln!("  {}  {}", style(format!("{}.", i + 1)).cyan().bold(), step);
    }
    eprintln!();

    let key: String = Input::new()
        .with_prompt(format!("  {} API Key", style("›").cyan()))
        .interact_text()?;

    let key = key.trim().to_string();
    if key.is_empty() {
        bail!("API key cannot be empty");
    }

    let sp = ui::spinner("Validating...");
    let valid = validate_key(&key, provider.base_url, provider.default_model).await;
    sp.finish_and_clear();

    match valid {
        Ok(()) => {
            eprintln!(
                "  {} {}",
                style("✓").green().bold(),
                style(format!("Key works! {} is ready.", provider.name)).green()
            );
        }
        Err(e) => {
            eprintln!(
                "  {} {}",
                style("⚠").yellow().bold(),
                style(format!("Could not validate: {e}")).yellow()
            );
            eprintln!(
                "  {} Saving anyway — test with {}",
                style("·").dim(),
                style("eai hello").cyan()
            );
        }
    }

    eprintln!();
    write_shell_env(provider.api_key_env, &key)?;
    unsafe { env::set_var(provider.api_key_env, &key) };

    let mut config = AppConfig::load().unwrap_or_default();
    apply_provider_config(&mut config, provider);
    write_config(&config)?;

    eprintln!();
    print_done();
    Ok(())
}

fn setup_ollama(provider: &Provider) -> Result<()> {
    for (i, step) in provider.steps.iter().enumerate() {
        eprintln!("  {}  {}", style(format!("{}.", i + 1)).cyan().bold(), step);
    }
    eprintln!();

    let mut config = AppConfig::load().unwrap_or_default();
    config.default.backend = BackendPreference::Ollama;
    write_config(&config)?;

    eprintln!();
    print_done();
    Ok(())
}

async fn setup_custom() -> Result<()> {
    eprintln!("  {} Any OpenAI-compatible API works.", style("·").dim());
    eprintln!(
        "  {} GLM (Zhipu), Kimi (Moonshot), Together, Fireworks, Cerebras, DeepSeek...",
        style("·").dim()
    );
    eprintln!();

    let base_url: String = Input::new()
        .with_prompt(format!("  {} Base URL", style("›").cyan()))
        .with_initial_text("https://")
        .interact_text()?;

    let model: String = Input::new()
        .with_prompt(format!("  {} Model name", style("›").cyan()))
        .interact_text()?;

    let env_name: String = Input::new()
        .with_prompt(format!("  {} Env var name for the key", style("›").cyan()))
        .with_initial_text("CUSTOM_API_KEY")
        .interact_text()?;

    let key: String = Input::new()
        .with_prompt(format!("  {} API Key", style("›").cyan()))
        .interact_text()?;

    let key = key.trim().to_string();
    if key.is_empty() {
        bail!("API key cannot be empty");
    }

    write_shell_env(&env_name, &key)?;
    unsafe { env::set_var(&env_name, &key) };

    let mut config = AppConfig::load().unwrap_or_default();
    config.default.backend = BackendPreference::Openai;
    config.openai.api_key_env = env_name;
    config.openai.model = model;
    config.openai.base_url = base_url;
    write_config(&config)?;

    eprintln!();
    print_done();
    Ok(())
}

// ── config helpers ─────────────────────────────────────────────────────────

fn apply_provider_config(config: &mut AppConfig, provider: &Provider) {
    match provider.target {
        ConfigTarget::Groq => {
            config.default.backend = BackendPreference::Groq;
            config.groq.api_key_env = provider.api_key_env.to_string();
            config.groq.model = provider.default_model.to_string();
            config.groq.base_url = provider.base_url.to_string();
        }
        ConfigTarget::Openai => {
            config.default.backend = BackendPreference::Openai;
            config.openai.api_key_env = provider.api_key_env.to_string();
            config.openai.model = provider.default_model.to_string();
            config.openai.base_url = provider.base_url.to_string();
        }
        ConfigTarget::Ollama => {
            config.default.backend = BackendPreference::Ollama;
        }
        ConfigTarget::Custom => {}
    }
}

fn write_config(config: &AppConfig) -> Result<()> {
    let path = config::config_path()?;
    config::ensure_parent(&path)?;
    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, &contents)?;
    eprintln!(
        "  {} Saved config to {}",
        style("✓").green(),
        style(path.display()).dim()
    );
    Ok(())
}

// ── API key validation ─────────────────────────────────────────────────────

async fn validate_key(key: &str, base_url: &str, model: &str) -> Result<()> {
    let client = Client::builder()
        .user_agent("eai/0.1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let resp = client
        .post(&endpoint)
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 1
        }))
        .send()
        .await
        .context("network error")?;

    if resp.status().is_success() {
        Ok(())
    } else if resp.status().as_u16() == 401 {
        bail!("invalid API key")
    } else {
        bail!("API returned {}", resp.status())
    }
}

// ── shell profile ──────────────────────────────────────────────────────────

fn write_shell_env(name: &str, value: &str) -> Result<()> {
    let shell = env::var("SHELL").unwrap_or_default();
    let shell_name = shell.rsplit('/').next().unwrap_or("zsh");

    let home = dirs::home_dir().context("could not find home directory")?;

    let (profile, export_line) = match shell_name {
        "fish" => (
            home.join(".config/fish/config.fish"),
            format!("set -gx {name} \"{value}\""),
        ),
        "bash" => {
            let p = if home.join(".bash_profile").exists() {
                home.join(".bash_profile")
            } else {
                home.join(".bashrc")
            };
            (p, format!("export {name}=\"{value}\""))
        }
        _ => (home.join(".zshrc"), format!("export {name}=\"{value}\"")),
    };

    if let Ok(contents) = fs::read_to_string(&profile) {
        let marker = format!("{name}=");
        let fish_marker = format!("set -gx {name}");
        if contents.contains(&marker) || contents.contains(&fish_marker) {
            let updated: Vec<String> = contents
                .lines()
                .map(|l| {
                    if l.contains(&marker) || l.contains(&fish_marker) {
                        export_line.clone()
                    } else {
                        l.to_string()
                    }
                })
                .collect();
            fs::write(&profile, updated.join("\n") + "\n")?;
            eprintln!(
                "  {} Updated {} in {}",
                style("✓").green(),
                style(name).cyan(),
                style(profile.display()).dim()
            );
            return Ok(());
        }
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&profile)?;
    writeln!(file)?;
    writeln!(file, "# eai — AI shell command generator")?;
    writeln!(file, "{export_line}")?;

    eprintln!(
        "  {} Added {} to {}",
        style("✓").green(),
        style(name).cyan(),
        style(profile.display()).dim()
    );
    eprintln!(
        "  {} Run {} or restart your terminal",
        style("·").dim(),
        style(format!("source {}", profile.display())).cyan()
    );

    Ok(())
}

// ── UX ─────────────────────────────────────────────────────────────────────

fn print_done() {
    eprintln!(
        "  {} {} Try: {}",
        style("⚡").cyan().bold(),
        style("You're all set!").green().bold(),
        style("eai list files modified today").white().bold()
    );
    eprintln!();
}

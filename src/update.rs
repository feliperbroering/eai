use std::fs;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

use crate::{config, ui};

const GITHUB_REPO: &str = "feliperbroering/eai";
const CHECK_INTERVAL: Duration = Duration::from_secs(86400); // 24h
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

fn cache_path() -> Result<std::path::PathBuf> {
    let base = dirs::cache_dir().ok_or_else(|| anyhow::anyhow!("no cache dir"))?;
    Ok(base.join("eai").join("last_update_check"))
}

fn should_check() -> bool {
    let Ok(path) = cache_path() else {
        return true;
    };
    let Ok(meta) = fs::metadata(&path) else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    SystemTime::now()
        .duration_since(modified)
        .unwrap_or(CHECK_INTERVAL)
        >= CHECK_INTERVAL
}

fn touch_cache() {
    if let Ok(path) = cache_path() {
        let _ = config::ensure_parent(&path);
        let _ = fs::write(&path, "");
    }
}

fn parse_version(tag: &str) -> Option<(u32, u32, u32)> {
    let v = tag.strip_prefix('v').unwrap_or(tag);
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

fn is_newer(latest: &str) -> bool {
    let Some(current) = parse_version(CURRENT_VERSION) else {
        return false;
    };
    let Some(latest) = parse_version(latest) else {
        return false;
    };
    latest > current
}

/// Check for updates in the background. Returns the latest version if newer.
pub async fn check(http_client: &Client) -> Option<String> {
    if !should_check() {
        return None;
    }

    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let resp = http_client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let release: GithubRelease = resp.json().await.ok()?;

    // Only touch cache after a successful API response
    touch_cache();

    let tag = release.tag_name;

    if is_newer(&tag) {
        let version = tag.strip_prefix('v').unwrap_or(&tag).to_string();
        Some(version)
    } else {
        None
    }
}

/// Show update banner and prompt for install.
pub fn prompt_update(latest: &str) -> Result<bool> {
    ui::print_update_available(CURRENT_VERSION, latest);

    let term = console::Term::stdout();
    loop {
        match term.read_key()? {
            console::Key::Char('y' | 'Y') | console::Key::Enter => return Ok(true),
            console::Key::Char('n' | 'N') | console::Key::CtrlC => return Ok(false),
            _ => {}
        }
    }
}

pub fn install_command() -> Option<(&'static str, Vec<&'static str>)> {
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        Some((
            "bash",
            vec![
                "-c",
                "curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash",
            ],
        ))
    } else {
        // Windows: no reliable auto-update — guide user to GitHub releases
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("v99.0.0"));
        assert!(!is_newer("v0.0.1"));
        assert!(!is_newer(CURRENT_VERSION));
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("invalid"), None);
    }
}

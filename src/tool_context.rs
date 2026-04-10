use std::{env, time::Duration};

use anyhow::Result;
use reqwest::Client;
use tokio::{process::Command, time::timeout};
use which::which;

use crate::{config::SearchEngine, llm::Backend, search, ui};

pub struct ToolContext {
    pub tool_docs: Option<String>,
}

pub async fn gather(
    backend: &Backend,
    prompt: &str,
    http_client: &Client,
    search_engine: SearchEngine,
) -> Result<ToolContext> {
    let sp = ui::spinner("Analyzing prompt...");
    let tools = match extract_tool_names(backend, prompt).await {
        Ok(t) => t,
        Err(_) => {
            sp.finish_and_clear();
            return Ok(ToolContext { tool_docs: None });
        }
    };
    sp.finish_and_clear();

    if tools.is_empty() {
        return Ok(ToolContext { tool_docs: None });
    }

    let mut sections = vec![];

    for tool in &tools {
        if which(tool).is_ok() {
            let sp = ui::spinner(&format!("Loading {tool} docs..."));
            let (version, docs) = tokio::join!(get_tool_version(tool), get_tool_docs(tool));
            sp.finish_and_clear();

            let label = match &version {
                Some(v) => format!("{tool} {v}"),
                None => tool.to_string(),
            };

            match docs {
                Some((source, doc_text)) => {
                    ui::status_ok(&format!("Found {label} — loaded docs from {source}"));
                    sections.push(format!("### {tool} (installed)\n{doc_text}"));
                }
                None => {
                    ui::status_ok(&format!("Found {label}"));
                    ui::status_warn(&format!("No docs found for {tool}"));
                }
            }
        } else {
            ui::status_warn(&format!("'{tool}' not found"));
            let sp = ui::spinner(&format!("Searching docs for {tool}..."));
            let usage_query = format!("{tool} cli flags usage reference");
            let install_query = format!("how to install {tool}");
            let mut combined = vec![];
            if let Ok(r) = search::search(http_client, search_engine, &usage_query).await
                && let Some(ctx) = r.as_prompt_context()
            {
                combined.push(ctx);
            }
            if let Ok(r) = search::search(http_client, search_engine, &install_query).await
                && let Some(ctx) = r.as_prompt_context()
            {
                combined.push(ctx);
            }
            sp.finish_and_clear();
            if combined.is_empty() {
                ui::status_warn(&format!("No docs found for {tool}"));
                sections.push(format!("### {tool} (not installed)"));
            } else {
                ui::status_ok("Found install instructions");
                sections.push(format!(
                    "### {tool} (not installed)\n{}",
                    combined.join("\n\n")
                ));
            }
        }
    }

    let tool_docs = if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    };

    Ok(ToolContext { tool_docs })
}

async fn extract_tool_names(backend: &Backend, prompt: &str) -> Result<Vec<String>> {
    let system = r#"You extract CLI tool names from user prompts. Strict rules:
- ONLY extract names of specific external CLI programs (ffmpeg, docker, kubectl, terraform, etc.)
- Do NOT extract: filenames, file paths, common words, descriptions, or concepts
- Do NOT extract shell builtins or coreutils: cat, head, tail, ls, cp, mv, rm, mkdir, grep, find, sort, sed, awk, echo, cd, pwd, chmod, chown, tar, gzip, curl, wget, ssh, scp, less, more, touch, wc, cut, tr, diff, man, which, kill, ps, top, env, xargs, tee, basename, dirname, stat, file, ln, df, du, dd, mount
- Output one tool per line, lowercase, ASCII only
- Output NOTHING (completely empty) if the request can be done with standard shell commands
- Maximum 5 tools

Input: use ffmpeg to convert video to mp4
Output: ffmpeg

Input: show me the beginning of readme.md
Output:

Input: me mostre o começo do stm32wb55cc.md pra eu ter nocao do conteudo dele
Output:

Input: list all docker containers running
Output: docker

Input: list files sorted by date
Output:

Input: compress with imagemagick then convert with ffmpeg
Output: imagemagick
ffmpeg

Input: deploy with terraform and check with kubectl
Output: terraform
kubectl

Input: como usar o comando docling pra converter pdf
Output: docling

Input: create a git branch and push
Output:

Input: tar the src folder
Output:"#;

    let raw = backend.call(system, prompt).await?;

    let mut seen = std::collections::HashSet::new();
    let tools = raw
        .lines()
        .filter_map(|line| {
            let word = line.split_whitespace().next()?.trim().to_lowercase();
            if word.len() <= 1 || word.len() > 40 {
                return None;
            }
            if !word
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return None;
            }
            if is_noise_word(&word) {
                return None;
            }
            if seen.insert(word.clone()) {
                Some(word)
            } else {
                None
            }
        })
        .take(5)
        .collect();

    Ok(tools)
}

fn is_noise_word(w: &str) -> bool {
    matches!(
        w,
        // English stop words
        "output" | "input" | "the" | "a" | "an" | "and" | "or" | "to" | "for"
            | "in" | "of" | "with" | "from" | "by" | "on" | "at" | "is" | "it"
            | "as" | "no" | "not" | "if" | "my" | "me" | "do" | "all" | "use"
            | "run" | "set" | "get" | "new" | "add" | "how" | "file" | "files"
            | "show" | "list" | "make" | "help" | "like" | "want" | "need"
            | "just" | "this" | "that" | "please" | "also" | "using" | "into"
            | "about" | "then" | "only" | "here" | "there" | "can" | "will"
            | "none" | "noop" | "nothing" | "empty" | "yes" | "true" | "false"
            | "null" | "command" | "commands" | "shell" | "terminal" | "console"
            // Shell builtins / coreutils — LLM already knows these, docs just add noise
            | "cat" | "head" | "tail" | "ls" | "cp" | "mv" | "rm" | "mkdir"
            | "rmdir" | "pwd" | "grep" | "find" | "sort" | "sed" | "awk"
            | "wc" | "chmod" | "chown" | "tar" | "gzip" | "zip" | "unzip"
            | "less" | "more" | "touch" | "ln" | "df" | "du" | "ps" | "kill"
            | "top" | "env" | "export" | "source" | "alias" | "which" | "man"
            | "date" | "cal" | "test" | "read" | "tee"
            | "xargs" | "diff" | "patch" | "ssh" | "scp" | "curl" | "wget"
            | "ping" | "nc" | "tr" | "cut" | "paste" | "basename" | "dirname"
            | "realpath" | "stat" | "dd" | "mount" | "echo" | "cd" | "git"
    )
}

// ── version ─────────────────────────────────────────────────────────────────

async fn get_tool_version(tool: &str) -> Option<String> {
    let output = timeout(
        Duration::from_secs(15),
        Command::new(tool).arg("--version").output(),
    )
    .await
    .ok()?
    .ok()?;

    let text = if !output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    // Prefer lines containing "version", then fall back to any line
    for line in text.lines() {
        if line.to_lowercase().contains("version")
            && let Some(v) = find_version_number(line)
        {
            return Some(format!("v{v}"));
        }
    }
    for line in text.lines() {
        if let Some(v) = find_version_number(line) {
            return Some(format!("v{v}"));
        }
    }
    None
}

/// Find first semver-like pattern (X.Y or X.Y.Z) in a string.
fn find_version_number(s: &str) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                if chars[i] == '.' {
                    dots += 1;
                }
                i += 1;
            }
            // Trim trailing dot
            if i > start && chars[i - 1] == '.' {
                i -= 1;
                dots -= 1;
            }
            if dots >= 1 {
                return Some(chars[start..i].iter().collect());
            }
        } else {
            i += 1;
        }
    }
    None
}

// ── tldr ────────────────────────────────────────────────────────────────────

async fn ensure_tldr() {
    if which("tldr").is_ok() {
        return;
    }

    let sp = ui::spinner("Installing tldr...");

    let status = match env::consts::OS {
        "macos" => Command::new("brew")
            .args(["install", "tldr"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .ok(),
        _ => Command::new("cargo")
            .args(["install", "tealdeer"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .ok(),
    };

    sp.finish_and_clear();

    if status.map(|s| s.success()).unwrap_or(false) {
        let _ = Command::new("tldr").arg("--update").status().await;
        ui::status_ok("Installed tldr");
    }
}

// ── doc resolution: tldr → --help ───────────────────────────────────────────

async fn get_tool_docs(tool: &str) -> Option<(String, String)> {
    ensure_tldr().await;

    // 1. tldr (concise, community-curated)
    if let Ok(Ok(output)) = timeout(
        Duration::from_secs(15),
        Command::new("tldr")
            .args(["--color", "never", tool])
            .output(),
    )
    .await
        && output.status.success()
    {
        let clean = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        if !clean.trim().is_empty() {
            return Some(("tldr".into(), truncate(clean, 3000)));
        }
    }

    // 2. --help (local, always up to date)
    let output = timeout(
        Duration::from_secs(15),
        Command::new(tool)
            .arg("--help")
            .env("COLUMNS", "200")
            .env("TERM", "dumb")
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    let raw = if !output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    let clean = clean_help_text(&raw);
    if clean.trim().is_empty() {
        return None;
    }

    Some(("--help".into(), truncate(clean, 3000)))
}

// ── help text cleanup ───────────────────────────────────────────────────────

fn clean_help_text(raw: &str) -> String {
    let stripped = strip_ansi(raw);
    let mut lines: Vec<String> = Vec::new();

    for line in stripped.lines() {
        let cleaned: String = line
            .chars()
            .map(|c| if is_box_drawing(c) { ' ' } else { c })
            .collect();

        let collapsed: String = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

        if !collapsed.is_empty() {
            lines.push(collapsed);
        }
    }

    lines.join("\n")
}

fn is_box_drawing(c: char) -> bool {
    matches!(c, '\u{2500}'..='\u{257F}' | '▶' | '•' | '★' | '●')
}

// ── string helpers ──────────────────────────────────────────────────────────

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for ch in chars.by_ref() {
                if ch.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn truncate(s: String, max: usize) -> String {
    if s.chars().count() <= max {
        s
    } else {
        format!("{}...", s.chars().take(max).collect::<String>())
    }
}

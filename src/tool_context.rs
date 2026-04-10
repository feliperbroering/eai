use std::{env, process::Stdio, time::Duration};

use anyhow::{Result, bail};
use reqwest::Client;
use serde::Deserialize;
use tokio::{process::Command, time::timeout};
use which::which;

use crate::{config::SearchEngine, llm::Backend, search, ui};

pub struct ToolContext {
    pub tool_docs: Option<String>,
}

// ── tool discovery ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ToolSuggestion {
    pub name: String,
    pub description: String,
    pub repo_url: String,
    pub install_cmd: String,
    #[allow(dead_code)]
    pub confidence: u8,
    #[serde(skip)]
    pub version: Option<String>,
    #[serde(skip)]
    pub verified: bool,
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

    let all_missing = tools.iter().all(|t| which(t).is_err());

    if all_missing {
        let missing_names: Vec<String> = tools.iter().map(|s| s.to_string()).collect();

        match try_discover_and_install(backend, prompt, &missing_names, http_client, search_engine)
            .await
        {
            DiscoverResult::Installed(tool_name) => {
                return gather_installed_tool(&tool_name).await;
            }
            DiscoverResult::Skipped => {
                return Ok(ToolContext { tool_docs: None });
            }
            DiscoverResult::Cancelled => bail!("cancelled"),
        }
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
        }
    }

    let tool_docs = if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    };

    Ok(ToolContext { tool_docs })
}

enum DiscoverResult {
    Installed(String),
    Skipped,
    Cancelled,
}

async fn try_discover_and_install(
    backend: &Backend,
    prompt: &str,
    missing_tools: &[String],
    http_client: &Client,
    search_engine: SearchEngine,
) -> DiscoverResult {
    let suggestions =
        match discover_alternatives(backend, prompt, missing_tools, http_client, search_engine)
            .await
        {
            Ok(s) if !s.is_empty() => s,
            _ => return DiscoverResult::Skipped,
        };

    ui::print_tool_suggestions(&suggestions);

    loop {
        match ui::prompt_tool_install(suggestions.len()) {
            Ok(ui::InstallAction::Install(idx)) => {
                let chosen = &suggestions[idx];
                eprintln!();

                match install_tool(chosen).await {
                    Ok(()) => return DiscoverResult::Installed(chosen.name.clone()),
                    Err(e) => {
                        ui::status_warn(&format!("Install failed: {e}"));
                        eprintln!();
                        ui::print_tool_suggestions(&suggestions);
                    }
                }
            }
            Ok(ui::InstallAction::Skip) => return DiscoverResult::Skipped,
            Ok(ui::InstallAction::Cancel) | Err(_) => return DiscoverResult::Cancelled,
        }
    }
}

async fn gather_installed_tool(tool: &str) -> Result<ToolContext> {
    let sp = ui::spinner(&format!("Loading {tool} docs..."));
    let (version, docs) = tokio::join!(get_tool_version(tool), get_tool_docs(tool));
    sp.finish_and_clear();

    let label = match &version {
        Some(v) => format!("{tool} {v}"),
        None => tool.to_string(),
    };

    let doc_section = match docs {
        Some((source, doc_text)) => {
            ui::status_ok(&format!("Found {label} — loaded docs from {source}"));
            format!("### {tool} (installed)\n{doc_text}")
        }
        None => {
            ui::status_ok(&format!("Found {label}"));
            format!("### {tool} (installed)")
        }
    };

    Ok(ToolContext {
        tool_docs: Some(doc_section),
    })
}

// ── discovery + install ─────────────────────────────────────────────────────

async fn discover_alternatives(
    backend: &Backend,
    prompt: &str,
    missing_tools: &[String],
    http_client: &Client,
    search_engine: SearchEngine,
) -> Result<Vec<ToolSuggestion>> {
    let sp = ui::spinner("Searching for tools...");

    let queries: Vec<String> = missing_tools
        .iter()
        .map(|t| format!("{t} CLI tool install"))
        .chain(std::iter::once(format!(
            "best CLI tool for {prompt} terminal"
        )))
        .collect();

    let mut all_snippets = Vec::new();
    for q in &queries {
        if let Ok(r) = search::search(http_client, search_engine, q).await {
            if let Some(ctx) = r.as_prompt_context() {
                all_snippets.push(ctx);
            }
        }
    }

    sp.finish_and_clear();

    if all_snippets.is_empty() {
        return Ok(vec![]);
    }

    let snippets = all_snippets.join("\n\n");

    let pm = detect_package_manager();
    let os = std::env::consts::OS;
    let system = format!(
        r#"You suggest CLI tools for a user's task. Return ONLY a JSON array (no markdown fences).
Each element: {{"name":"...","description":"...","repo_url":"https://github.com/OWNER/REPO","install_cmd":"...","confidence":0-100}}
Rules:
- Suggest up to 3 real CLI tools that can accomplish the user's task, sorted by relevance
- repo_url should be the tool's GitHub URL if you know it; use your best guess for owner/repo
- install_cmd must work on {os} — prefer {pm}, fallback to pip/cargo/npm
- confidence: 90+ = popular well-known tool, 70-89 = established, 50-69 = niche or uncertain
- Include tools mentioned in the search results even if you're not 100% sure about the repo URL"#
    );

    let user_msg = format!(
        "User wants: {prompt}\nTools mentioned but not installed: {}\nOS: {os}\nSearch results:\n{snippets}",
        missing_tools.join(", ")
    );

    let sp = ui::spinner("Evaluating tools...");
    let raw = backend.call(&system, &user_msg).await?;
    sp.finish_and_clear();

    let json_str = extract_json_array(&raw);

    match serde_json::from_str::<Vec<ToolSuggestion>>(&json_str) {
        Ok(mut suggestions) => {
            suggestions.truncate(3);
            verify_suggestions(http_client, &mut suggestions).await;
            suggestions.sort_by(|a, b| b.verified.cmp(&a.verified));
            Ok(suggestions)
        }
        Err(_) => Ok(vec![]),
    }
}

// ── package registry verification ───────────────────────────────────────────

/// Detect which package manager the install_cmd uses.
fn detect_registry(install_cmd: &str) -> Option<&'static str> {
    let cmd = install_cmd.trim();
    if cmd.starts_with("brew ") {
        Some("brew")
    } else if cmd.starts_with("pip ") || cmd.starts_with("pip3 ") || cmd.starts_with("pipx ") {
        Some("pip")
    } else if cmd.starts_with("npm ") || cmd.starts_with("npx ") {
        Some("npm")
    } else if cmd.starts_with("cargo ") {
        Some("cargo")
    } else {
        None
    }
}

/// Extract the package name from an install command.
fn extract_pkg_name(install_cmd: &str) -> Option<String> {
    let parts: Vec<&str> = install_cmd.split_whitespace().collect();
    parts
        .iter()
        .rev()
        .find(|p| !p.starts_with('-'))
        .map(|s| s.to_string())
}

struct PkgInfo {
    description: Option<String>,
    homepage: Option<String>,
    version: Option<String>,
}

async fn check_brew(client: &Client, name: &str) -> Option<PkgInfo> {
    #[derive(Deserialize)]
    struct BrewFormula {
        desc: Option<String>,
        homepage: Option<String>,
        versions: Option<BrewVersions>,
    }
    #[derive(Deserialize)]
    struct BrewVersions {
        stable: Option<String>,
    }

    let url = format!("https://formulae.brew.sh/api/formula/{name}.json");
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let f = resp.json::<BrewFormula>().await.ok()?;
    Some(PkgInfo {
        description: f.desc,
        homepage: f.homepage,
        version: f.versions.and_then(|v| v.stable),
    })
}

async fn check_pypi(client: &Client, name: &str) -> Option<PkgInfo> {
    #[derive(Deserialize)]
    struct PyPiResponse {
        info: PyPiInfo,
    }
    #[derive(Deserialize)]
    struct PyPiInfo {
        summary: Option<String>,
        home_page: Option<String>,
        project_url: Option<String>,
        version: Option<String>,
    }

    let url = format!("https://pypi.org/pypi/{name}/json");
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let p = resp.json::<PyPiResponse>().await.ok()?;
    let homepage = p
        .info
        .home_page
        .or(p.info.project_url)
        .filter(|u| !u.is_empty());
    Some(PkgInfo {
        description: p.info.summary,
        homepage,
        version: p.info.version,
    })
}

async fn check_npm(client: &Client, name: &str) -> Option<PkgInfo> {
    #[derive(Deserialize)]
    struct NpmPkg {
        description: Option<String>,
        homepage: Option<String>,
        #[serde(rename = "dist-tags")]
        dist_tags: Option<NpmDistTags>,
    }
    #[derive(Deserialize)]
    struct NpmDistTags {
        latest: Option<String>,
    }

    let url = format!("https://registry.npmjs.org/{name}");
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let n = resp.json::<NpmPkg>().await.ok()?;
    Some(PkgInfo {
        description: n.description,
        homepage: n.homepage,
        version: n.dist_tags.and_then(|d| d.latest),
    })
}

async fn check_crates(client: &Client, name: &str) -> Option<PkgInfo> {
    #[derive(Deserialize)]
    struct CratesResponse {
        #[serde(rename = "crate")]
        krate: CrateInfo,
    }
    #[derive(Deserialize)]
    struct CrateInfo {
        description: Option<String>,
        homepage: Option<String>,
        repository: Option<String>,
        max_stable_version: Option<String>,
    }

    let url = format!("https://crates.io/api/v1/crates/{name}");
    let resp = client
        .get(&url)
        .header("User-Agent", "eai/0.1.0")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let c = resp.json::<CratesResponse>().await.ok()?;
    let homepage = c
        .krate
        .homepage
        .or(c.krate.repository)
        .filter(|u| !u.is_empty());
    Some(PkgInfo {
        description: c.krate.description,
        homepage,
        version: c.krate.max_stable_version,
    })
}

fn is_valid_pkg_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | '/'))
        && !name.contains("..")
}

async fn check_registry(client: &Client, registry: &str, pkg_name: &str) -> Option<PkgInfo> {
    if !is_valid_pkg_name(pkg_name) {
        return None;
    }
    match registry {
        "brew" => check_brew(client, pkg_name).await,
        "pip" => check_pypi(client, pkg_name).await,
        "npm" => check_npm(client, pkg_name).await,
        "cargo" => check_crates(client, pkg_name).await,
        _ => None,
    }
}

fn apply_pkg_info(s: &mut ToolSuggestion, info: &PkgInfo) {
    s.verified = true;
    if let Some(ref desc) = info.description {
        s.description = desc.clone();
    }
    if let Some(ref url) = info.homepage {
        s.repo_url = url.clone();
    }
    if let Some(ref ver) = info.version {
        s.version = Some(format!("v{ver}"));
    }
}

async fn verify_suggestions(http_client: &Client, suggestions: &mut [ToolSuggestion]) {
    let sp = ui::spinner("Verifying packages...");

    for s in suggestions.iter_mut() {
        let registry = detect_registry(&s.install_cmd);
        let pkg_name = extract_pkg_name(&s.install_cmd);

        if let (Some(reg), Some(name)) = (registry, &pkg_name) {
            if let Some(info) = check_registry(http_client, reg, name).await {
                apply_pkg_info(s, &info);
                continue;
            }
        }

        let registries = ["brew", "pip", "npm", "cargo"];
        for reg in &registries {
            if registry == Some(*reg) {
                continue;
            }
            if let Some(info) = check_registry(http_client, reg, &s.name).await {
                apply_pkg_info(s, &info);
                s.install_cmd = match *reg {
                    "brew" => format!("brew install {}", s.name),
                    "pip" => format!("pip install {}", s.name),
                    "npm" => format!("npm install -g {}", s.name),
                    "cargo" => format!("cargo install {}", s.name),
                    _ => continue,
                };
                break;
            }
        }
    }

    sp.finish_and_clear();
}

fn detect_package_manager() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew"
    } else if which("apt").is_ok() {
        "apt"
    } else if which("pacman").is_ok() {
        "pacman"
    } else if which("dnf").is_ok() {
        "dnf"
    } else {
        "manual"
    }
}

const ALLOWED_INSTALLERS: &[&str] = &["brew", "pip", "pip3", "pipx", "npm", "npx", "cargo"];

async fn install_tool(suggestion: &ToolSuggestion) -> Result<()> {
    let cmd = &suggestion.install_cmd;

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        bail!("empty install command");
    }

    if !ALLOWED_INSTALLERS.contains(&parts[0]) {
        eprintln!();
        ui::status_warn("Unrecognized installer. Run manually:");
        eprintln!("  {}", console::style(cmd).cyan().bold());
        bail!("manual install required");
    }

    eprintln!(
        "  {} {}",
        console::style("⟩").cyan().bold(),
        console::style(cmd).dim()
    );
    eprintln!();

    let status = Command::new(parts[0])
        .args(&parts[1..])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;

    eprintln!();

    match status {
        Ok(s) if s.success() => {
            ui::status_ok(&format!("{} installed", suggestion.name));
            Ok(())
        }
        Ok(s) => bail!(
            "install failed (exit {}). Try another option.",
            s.code().unwrap_or(-1)
        ),
        Err(e) => bail!("could not run '{}': {e}", parts[0]),
    }
}

// ── tool extraction ─────────────────────────────────────────────────────────

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

fn extract_json_array(raw: &str) -> String {
    let s = raw.trim();
    if let Some(start) = s.find('[') {
        if let Some(end) = s.rfind(']') {
            return s[start..=end].to_string();
        }
    }
    s.trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string()
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

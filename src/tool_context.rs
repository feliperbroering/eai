use std::{process::Stdio, time::Duration};

use anyhow::{Result, bail};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tokio::{process::Command, time::timeout};
use which::which;

use crate::{config::SearchEngine, llm::Backend, search, tldr, ui};

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
    #[serde(skip)]
    pub stars: Option<u64>,
    #[serde(skip)]
    pub recent_commits: Option<u64>,
    #[serde(skip)]
    pub contributors: Option<u64>,
    #[serde(skip)]
    pub open_issues: Option<u64>,
    #[serde(skip)]
    pub last_push: Option<String>,
    #[serde(skip)]
    pub heat_score: f64,
    #[serde(skip)]
    pub review: Option<String>,
}

fn is_coreutils_only_prompt(prompt: &str) -> bool {
    let words: Vec<&str> = prompt
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_'))
        .filter(|w| !w.is_empty())
        .collect();

    words.iter().all(|w| {
        let w = w.to_lowercase();
        is_noise_word(&w)
            || w.len() <= 2
            || w.chars()
                .any(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            || COMMON_WORDS.contains(&w.as_str())
    })
}

const COMMON_WORDS: &[&str] = &[
    "all",
    "files",
    "folder",
    "directory",
    "directories",
    "current",
    "home",
    "recursive",
    "recursively",
    "show",
    "display",
    "print",
    "count",
    "number",
    "size",
    "sizes",
    "large",
    "largest",
    "small",
    "smallest",
    "old",
    "oldest",
    "new",
    "newest",
    "recent",
    "modified",
    "created",
    "deleted",
    "hidden",
    "empty",
    "name",
    "names",
    "path",
    "paths",
    "type",
    "types",
    "extension",
    "permissions",
    "owner",
    "group",
    "contents",
    "content",
    "text",
    "lines",
    "words",
    "bytes",
    "characters",
    "pattern",
    "match",
    "matches",
    "replace",
    "rename",
    "copy",
    "move",
    "delete",
    "remove",
    "create",
    "make",
    "write",
    "read",
    "append",
    "prepend",
    "sort",
    "sorted",
    "unique",
    "duplicate",
    "duplicates",
    "compress",
    "extract",
    "archive",
    "zip",
    "unzip",
    "backup",
    "restore",
    "search",
    "find",
    "locate",
    "list",
    "tree",
    "compare",
    "diff",
    "merge",
    "split",
    "join",
    "concat",
    "head",
    "tail",
    "first",
    "last",
    "top",
    "bottom",
    "reverse",
    "shuffle",
    "random",
    "sample",
    "filter",
    "exclude",
    "include",
    "only",
    "except",
    "from",
    "into",
    "with",
    "without",
    "using",
    "the",
    "and",
    "but",
    "not",
    "this",
    "that",
    "those",
    "these",
    "here",
    "there",
    "where",
    "when",
    "how",
    "what",
    "which",
    "who",
    "why",
    "each",
    "every",
    "any",
    "some",
    "many",
    "few",
    "more",
    "less",
    "than",
    "between",
    "before",
    "after",
    "above",
    "below",
    "over",
    "under",
    "por",
    "para",
    "com",
    "sem",
    "todos",
    "todas",
    "cada",
    "entre",
    "maior",
    "menor",
    "mais",
    "menos",
    "como",
    "onde",
    "quando",
    "mostrar",
    "listar",
    "buscar",
    "procurar",
    "encontrar",
    "ordenar",
    "pasta",
    "pastas",
    "arquivo",
    "arquivos",
    "diretorio",
];

pub async fn gather(
    backend: &Backend,
    prompt: &str,
    http_client: &Client,
    search_engine: SearchEngine,
    interactive: bool,
) -> Result<ToolContext> {
    if is_coreutils_only_prompt(prompt) {
        return Ok(ToolContext { tool_docs: None });
    }

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
    let mut missing = vec![];

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
                }
            }
        } else if let Some(tldr_doc) = tldr::lookup(tool) {
            ui::status_ok(&format!("Loaded {tool} docs from embedded tldr"));
            sections.push(format!(
                "### {tool} (not installed — reference only)\n{tldr_doc}"
            ));
            missing.push(tool.to_string());
        } else {
            missing.push(tool.to_string());
        }
    }

    if !missing.is_empty() && interactive {
        match try_discover_and_install(backend, prompt, &missing, http_client, search_engine).await
        {
            DiscoverResult::Installed(tool_name) => {
                if let Ok(ctx) = gather_installed_tool(&tool_name).await {
                    if let Some(doc) = ctx.tool_docs {
                        sections.push(doc);
                    }
                }
            }
            DiscoverResult::Skipped | DiscoverResult::Cancelled => {}
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
- Suggest up to 7 real, well-known CLI tools that can accomplish the user's task
- Only suggest tools you are confident actually exist as real open-source projects
- repo_url MUST be the exact GitHub URL — do NOT guess or fabricate owner/repo
- install_cmd must work on {os} — prefer {pm}, fallback to pip/cargo/npm
- confidence: 90+ = popular well-known tool, 70-89 = established, 50-69 = niche
- Do NOT suggest tools just because they match a keyword — they must solve the user's actual task"#
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
            suggestions.truncate(7);
            verify_suggestions(http_client, &mut suggestions).await;
            suggestions.retain(|s| s.verified);

            let sp = ui::spinner("Fetching GitHub stats...");
            enrich_with_github(http_client, &mut suggestions).await;
            sp.finish_and_clear();

            for s in &mut suggestions {
                s.heat_score = compute_heat_score(s);
            }
            suggestions.sort_by(|a, b| {
                b.heat_score
                    .partial_cmp(&a.heat_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            suggestions.truncate(5);

            if !suggestions.is_empty() {
                let sp = ui::spinner("Writing editorial review...");
                write_editorial_reviews(backend, prompt, &mut suggestions).await;
                sp.finish_and_clear();
            }

            Ok(suggestions)
        }
        Err(_) => Ok(vec![]),
    }
}

// ── GitHub enrichment ─────────────────────────────────────────────────────

fn extract_github_owner_repo(url: &str) -> Option<(String, String)> {
    let url = url.trim().trim_end_matches('/');
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() >= 5 && parts[2].contains("github.com") {
        let owner = parts[3].to_string();
        let repo = parts[4].to_string();
        if !owner.is_empty() && !repo.is_empty() {
            return Some((owner, repo));
        }
    }
    None
}

#[derive(Deserialize)]
struct GitHubRepo {
    stargazers_count: Option<u64>,
    open_issues_count: Option<u64>,
    pushed_at: Option<String>,
}

async fn fetch_github_stats(
    client: &Client,
    owner: &str,
    repo: &str,
) -> Option<(u64, u64, u64, u64, String)> {
    let repo_url = format!("https://api.github.com/repos/{owner}/{repo}");
    let contributors_url =
        format!("https://api.github.com/repos/{owner}/{repo}/contributors?per_page=1&anon=true");
    let commits_url = format!("https://api.github.com/repos/{owner}/{repo}/commits?per_page=1");

    let headers = |req: reqwest::RequestBuilder| -> reqwest::RequestBuilder {
        req.header("User-Agent", "eai/0.2")
            .header("Accept", "application/vnd.github.v3+json")
    };

    let (repo_resp, contrib_resp, commits_resp) = tokio::join!(
        timeout(
            Duration::from_secs(5),
            headers(client.get(&repo_url)).send()
        ),
        timeout(
            Duration::from_secs(5),
            headers(client.get(&contributors_url)).send()
        ),
        timeout(
            Duration::from_secs(5),
            headers(client.get(&commits_url)).send()
        ),
    );

    let repo_data = repo_resp.ok()?.ok()?;
    if !repo_data.status().is_success() {
        return None;
    }
    let repo_info = repo_data.json::<GitHubRepo>().await.ok()?;

    let stars = repo_info.stargazers_count.unwrap_or(0);
    let open_issues = repo_info.open_issues_count.unwrap_or(0);
    let last_push = repo_info.pushed_at.unwrap_or_default();

    let contributors = if let Ok(Ok(resp)) = contrib_resp {
        parse_link_total(&resp).unwrap_or(1)
    } else {
        0
    };

    let recent_commits = if let Ok(Ok(resp)) = commits_resp {
        parse_link_total(&resp).unwrap_or(0)
    } else {
        0
    };

    Some((stars, recent_commits, contributors, open_issues, last_push))
}

fn parse_link_total(resp: &reqwest::Response) -> Option<u64> {
    let link = resp.headers().get("link")?.to_str().ok()?;
    for part in link.split(',') {
        if part.contains("rel=\"last\"") {
            let page_str = part.rsplit("page=").next()?.split('>').next()?;
            return page_str.parse().ok();
        }
    }
    None
}

async fn enrich_with_github(client: &Client, suggestions: &mut [ToolSuggestion]) {
    let futures: Vec<_> = suggestions
        .iter()
        .map(|s| {
            let client = client.clone();
            let url = s.repo_url.clone();
            async move {
                if let Some((owner, repo)) = extract_github_owner_repo(&url) {
                    fetch_github_stats(&client, &owner, &repo).await
                } else {
                    None
                }
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    for (s, data) in suggestions.iter_mut().zip(results) {
        if let Some((stars, recent_commits, contributors, open_issues, last_push)) = data {
            s.stars = Some(stars);
            s.recent_commits = Some(recent_commits);
            s.contributors = Some(contributors);
            s.open_issues = Some(open_issues);
            s.last_push = Some(last_push);
        }
    }
}

fn compute_heat_score(s: &ToolSuggestion) -> f64 {
    let stars = s.stars.unwrap_or(0) as f64;
    let contributors = s.contributors.unwrap_or(0) as f64;
    let open_issues = s.open_issues.unwrap_or(0) as f64;

    // Stars: log scale, max ~50 points (10k+ stars → ~46)
    let star_score = (stars + 1.0).ln() * 5.0;

    // Contributors: log scale, max ~20 points
    let contrib_score = (contributors + 1.0).ln() * 4.0;

    // Activity: how many days since last push, max ~20 points
    let activity_score = if let Some(ref pushed) = s.last_push {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(pushed) {
            let days_ago = (Utc::now() - dt.with_timezone(&Utc)).num_days().max(0) as f64;
            // Recent = high score: 0 days → 20, 30 days → 15, 365 days → 5, 2y+ → ~0
            20.0 * (-days_ago / 200.0).exp()
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Community engagement: issues indicate active use, log scale, max ~10
    let issue_score = (open_issues + 1.0).ln() * 2.0;

    star_score + contrib_score + activity_score + issue_score
}

// ── editorial review via LLM ──────────────────────────────────────────────

async fn write_editorial_reviews(
    backend: &Backend,
    prompt: &str,
    suggestions: &mut [ToolSuggestion],
) {
    let mut tool_summaries = Vec::new();
    for (i, s) in suggestions.iter().enumerate() {
        let stars = s.stars.map(|v| format!("{v}")).unwrap_or("?".into());
        let contribs = s.contributors.map(|v| format!("{v}")).unwrap_or("?".into());
        let activity = s.last_push.as_deref().unwrap_or("unknown");
        tool_summaries.push(format!(
            "{}. {} — {}\n   GitHub: {} | ★ {} | contributors: {} | last push: {}",
            i + 1,
            s.name,
            s.description,
            s.repo_url,
            stars,
            contribs,
            activity
        ));
    }

    let system = r#"You are a senior developer writing a brief tool recommendation. For each tool, write 1-2 sentences explaining why it ranks where it does — mention concrete strengths (speed, accuracy, ecosystem, maintenance) and any weaknesses. Be direct and opinionated like a dev blog benchmark post.

Return ONLY a JSON array of strings, one review per tool, in the same order as input. No markdown fences."#;

    let user_msg = format!(
        "User's task: {prompt}\n\nTools (ranked by popularity + activity):\n{}",
        tool_summaries.join("\n")
    );

    let Ok(raw) = backend.call(system, &user_msg).await else {
        return;
    };

    let json_str = extract_json_array(&raw);
    if let Ok(reviews) = serde_json::from_str::<Vec<String>>(&json_str) {
        for (s, review) in suggestions.iter_mut().zip(reviews) {
            s.review = Some(review);
        }
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
    let result = match registry {
        "brew" => timeout(Duration::from_secs(5), check_brew(client, pkg_name)).await,
        "pip" => timeout(Duration::from_secs(5), check_pypi(client, pkg_name)).await,
        "npm" => timeout(Duration::from_secs(5), check_npm(client, pkg_name)).await,
        "cargo" => timeout(Duration::from_secs(5), check_crates(client, pkg_name)).await,
        _ => return None,
    };
    result.ok().flatten()
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
Output:

Input: procure alguma tool que estime o consumo de tokens do stm32wb55cc.md
Output: tiktoken

Input: estimate token count of a markdown file for summarization
Output: tiktoken"#;

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

// ── doc resolution: embedded tldr → --help ──────────────────────────────────

async fn get_tool_docs(tool: &str) -> Option<(String, String)> {
    let tldr_docs = tldr::lookup(tool);
    let help_docs = fetch_help_output(tool).await;

    match (tldr_docs, help_docs) {
        (Some(t), Some(h)) => {
            let combined = format!("{t}\n---\n{h}");
            Some(("tldr+help".into(), truncate(combined, 4000)))
        }
        (Some(t), None) => Some(("tldr".into(), truncate(t, 3000))),
        (None, Some(h)) => Some(("--help".into(), truncate(h, 3000))),
        (None, None) => None,
    }
}

async fn fetch_help_output(tool: &str) -> Option<String> {
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

    Some(clean)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_github_owner_repo_valid() {
        let cases = vec![
            (
                "https://github.com/BurntSushi/ripgrep",
                Some(("BurntSushi".into(), "ripgrep".into())),
            ),
            (
                "https://github.com/sharkdp/fd/",
                Some(("sharkdp".into(), "fd".into())),
            ),
            (
                "https://github.com/cli/cli",
                Some(("cli".into(), "cli".into())),
            ),
        ];
        for (url, expected) in cases {
            assert_eq!(extract_github_owner_repo(url), expected, "url: {url}");
        }
    }

    #[test]
    fn extract_github_owner_repo_invalid() {
        assert_eq!(
            extract_github_owner_repo("https://pypi.org/project/foo"),
            None
        );
        assert_eq!(extract_github_owner_repo("not a url"), None);
        assert_eq!(extract_github_owner_repo("https://github.com/lonely"), None);
    }

    #[test]
    fn heat_score_prefers_popular_and_active() {
        let popular = ToolSuggestion {
            name: "ripgrep".into(),
            description: String::new(),
            repo_url: String::new(),
            install_cmd: String::new(),
            confidence: 95,
            version: None,
            verified: true,
            stars: Some(40000),
            recent_commits: Some(500),
            contributors: Some(200),
            open_issues: Some(100),
            last_push: Some("2026-04-01T00:00:00Z".into()),
            heat_score: 0.0,
            review: None,
        };

        let obscure = ToolSuggestion {
            name: "obscure-tool".into(),
            description: String::new(),
            repo_url: String::new(),
            install_cmd: String::new(),
            confidence: 50,
            version: None,
            verified: true,
            stars: Some(10),
            recent_commits: Some(2),
            contributors: Some(1),
            open_issues: Some(0),
            last_push: Some("2023-01-01T00:00:00Z".into()),
            heat_score: 0.0,
            review: None,
        };

        let stale = ToolSuggestion {
            name: "stale-tool".into(),
            description: String::new(),
            repo_url: String::new(),
            install_cmd: String::new(),
            confidence: 70,
            version: None,
            verified: true,
            stars: Some(5000),
            recent_commits: Some(100),
            contributors: Some(50),
            open_issues: Some(10),
            last_push: Some("2020-01-01T00:00:00Z".into()),
            heat_score: 0.0,
            review: None,
        };

        let score_popular = compute_heat_score(&popular);
        let score_obscure = compute_heat_score(&obscure);
        let score_stale = compute_heat_score(&stale);

        assert!(
            score_popular > score_stale,
            "popular ({score_popular}) should beat stale ({score_stale})"
        );
        assert!(
            score_stale > score_obscure,
            "stale ({score_stale}) should beat obscure ({score_obscure})"
        );
        assert!(
            score_popular > score_obscure,
            "popular ({score_popular}) should beat obscure ({score_obscure})"
        );
    }

    #[test]
    fn heat_score_handles_missing_data() {
        let minimal = ToolSuggestion {
            name: "tool".into(),
            description: String::new(),
            repo_url: String::new(),
            install_cmd: String::new(),
            confidence: 50,
            version: None,
            verified: true,
            stars: None,
            recent_commits: None,
            contributors: None,
            open_issues: None,
            last_push: None,
            heat_score: 0.0,
            review: None,
        };
        let score = compute_heat_score(&minimal);
        assert!(score >= 0.0, "score should be non-negative: {score}");
    }

    #[test]
    fn noise_word_filters_shell_builtins() {
        assert!(is_noise_word("cat"));
        assert!(is_noise_word("grep"));
        assert!(is_noise_word("git"));
        assert!(is_noise_word("file"));
        assert!(!is_noise_word("ffmpeg"));
        assert!(!is_noise_word("docker"));
        assert!(!is_noise_word("tiktoken"));
    }

    #[test]
    fn extract_json_array_handles_fences() {
        assert_eq!(extract_json_array("[1,2]"), "[1,2]");
        assert_eq!(extract_json_array("```json\n[1]\n```"), "[1]");
        assert_eq!(extract_json_array("text [1,2] more"), "[1,2]");
    }

    #[test]
    fn detect_registry_identifies_package_managers() {
        assert_eq!(detect_registry("brew install foo"), Some("brew"));
        assert_eq!(detect_registry("pip install bar"), Some("pip"));
        assert_eq!(detect_registry("pip3 install bar"), Some("pip"));
        assert_eq!(detect_registry("npm install -g baz"), Some("npm"));
        assert_eq!(detect_registry("cargo install qux"), Some("cargo"));
        assert_eq!(detect_registry("git clone https://github.com/x/y"), None);
    }

    #[test]
    fn extract_pkg_name_gets_last_non_flag() {
        assert_eq!(
            extract_pkg_name("brew install ripgrep"),
            Some("ripgrep".into())
        );
        assert_eq!(
            extract_pkg_name("npm install -g tiktoken"),
            Some("tiktoken".into())
        );
        assert_eq!(
            extract_pkg_name("pip install --user foo"),
            Some("foo".into())
        );
    }

    #[test]
    fn valid_pkg_name_rejects_bad_names() {
        assert!(is_valid_pkg_name("ripgrep"));
        assert!(is_valid_pkg_name("@scope/pkg"));
        assert!(!is_valid_pkg_name(""));
        assert!(!is_valid_pkg_name("a..b"));
        assert!(!is_valid_pkg_name(&"x".repeat(200)));
    }

    #[test]
    fn coreutils_only_skips_extraction() {
        assert!(is_coreutils_only_prompt("list all files sorted by size"));
        assert!(is_coreutils_only_prompt("show largest directories"));
        assert!(is_coreutils_only_prompt(
            "find empty files in current directory"
        ));
        assert!(is_coreutils_only_prompt("mostrar arquivos na pasta"));
        assert!(!is_coreutils_only_prompt("convert video with ffmpeg"));
        assert!(!is_coreutils_only_prompt("deploy with terraform"));
        assert!(!is_coreutils_only_prompt("run docker containers"));
    }
}

use std::io::{IsTerminal, Read};
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use reqwest::Client;
use tokio::process::Command;

use console::style;

use clap::CommandFactory;

use crate::{
    aliases, cache,
    cli::{self, Cli, Commands},
    config::{AppConfig, SearchEngine},
    history,
    llm::{self, Backend},
    search::{self, SearchResults},
    setup, tool_context,
    types::{CommandRequest, ExecutionResult, GeneratedCommand, HistoryEntry, OsKind, ShellKind},
    ui, update,
};

const MAX_ITERATIONS: usize = 5;

pub async fn run(cli: Cli) -> Result<()> {
    let config = AppConfig::load()?;

    match cli.command {
        Some(Commands::Setup) => setup::run_setup().await,
        Some(Commands::Config) => open_config().await,
        Some(Commands::History { search, limit }) => show_history(search.as_deref(), limit),
        Some(Commands::Completions { shell }) => {
            let mut cmd = cli::Cli::command();
            clap_complete::generate(shell, &mut cmd, "eai", &mut std::io::stdout());
            Ok(())
        }
        Some(Commands::Init { shell }) => {
            print_shell_integration(shell);
            Ok(())
        }
        Some(Commands::Save {
            name,
            command,
            desc,
        }) => {
            aliases::save(&name, &command, desc.as_deref())?;
            ui::status_ok(&format!("Saved alias @{name}"));
            Ok(())
        }
        Some(Commands::Aliases) => {
            let entries = aliases::list()?;
            if entries.is_empty() {
                eprintln!("  No saved aliases. Use `eai save <name> <command>` to create one.");
                return Ok(());
            }
            for (name, entry) in &entries {
                eprintln!(
                    "  {} {}",
                    style(format!("@{name}")).cyan().bold(),
                    style(&entry.command).bold()
                );
                if let Some(desc) = &entry.description {
                    eprintln!("    {}", style(desc).dim());
                }
            }
            Ok(())
        }
        Some(Commands::Unsave { name }) => {
            if aliases::remove(&name)? {
                ui::status_ok(&format!("Removed alias @{name}"));
            } else {
                ui::status_warn(&format!("Alias @{name} not found"));
            }
            Ok(())
        }
        Some(Commands::ClearCache) => {
            if cache::clear() {
                ui::status_ok("Cache cleared");
            } else {
                ui::status_warn("No cache to clear");
            }
            Ok(())
        }
        None => {
            if cli.demo {
                run_demo();
                return Ok(());
            }

            let has_prompt = cli.prompt.iter().any(|s| !s.trim().is_empty());
            if !has_prompt {
                cli::Cli::print_help();
                return Ok(());
            }

            let config = if setup::needs_setup() {
                ui::banner();
                setup::run_setup().await?;
                eprintln!();
                AppConfig::load()?
            } else {
                config
            };

            run_prompt(cli, config).await
        }
    }
}

async fn run_prompt(cli: Cli, config: AppConfig) -> Result<()> {
    let stdin_data = read_stdin_if_piped().await.map(|data| {
        let nature = detect_stdin_nature(&data);
        if nature != "text" {
            format!("[Detected: {nature}]\n{data}")
        } else {
            data
        }
    });
    let http_client = Client::builder()
        .user_agent(format!("eai/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    let update_check = tokio::spawn({
        let client = http_client.clone();
        async move { update::check(&client).await }
    });

    ui::banner();

    let prompt = cli.prompt.join(" ");

    if prompt.starts_with('@') {
        let alias_name = prompt.trim_start_matches('@');
        if let Some(entry) = aliases::get(alias_name)? {
            ui::status_ok(&format!("Alias @{alias_name}"));
            let generated = GeneratedCommand {
                command: entry.command,
                explanation: entry.description,
            };
            ui::print_command(&generated.command, generated.explanation.as_deref());
            if cli.dry {
                return Ok(());
            }
            let shell = cli
                .shell
                .or(config.default.shell)
                .unwrap_or_else(ShellKind::detect);
            eprintln!();
            let execution = execute_command(shell, &generated.command).await?;
            if !execution.is_success() {
                ui::print_exit_status(execution.exit_code, !execution.is_empty());
            }
            return Ok(());
        }
    }

    if let Some(ref data) = stdin_data {
        ui::print_stdin_badge(data.len());
    }

    let shell = cli
        .shell
        .or(config.default.shell)
        .unwrap_or_else(ShellKind::detect);
    let os = OsKind::detect();
    let backend = llm::resolve_backend(
        http_client.clone(),
        &config,
        cli.backend,
        cli.model.as_deref(),
    )
    .await?;

    if cli.explain {
        return run_explain(&backend, &prompt).await;
    }

    if cli.script {
        let request = CommandRequest {
            prompt: prompt.clone(),
            shell,
            os,
            context: None,
            search_results: None,
            tool_docs: None,
            history: vec![],
            stdin_data: stdin_data.clone(),
            project_context: detect_project_context(),
        };
        return run_script(&backend, &request, cli.verbose).await;
    }

    if cli.recipe {
        let request = CommandRequest {
            prompt: prompt.clone(),
            shell,
            os,
            context: None,
            search_results: None,
            tool_docs: None,
            history: vec![],
            stdin_data: stdin_data.clone(),
            project_context: detect_project_context(),
        };
        return run_recipe(&backend, &request, cli.verbose).await;
    }

    let recent_history = history::load_recent(5)?;
    let should_confirm = !cli.no_confirm && config.default.confirm;

    let interactive = !cli.dry && !cli.no_confirm;
    let tool_ctx = tool_context::gather(
        &backend,
        &prompt,
        &http_client,
        config.search.engine,
        interactive,
    )
    .await?;

    let project_ctx = detect_project_context();
    let mut request = CommandRequest {
        prompt: prompt.clone(),
        shell,
        os,
        context: None,
        search_results: None,
        tool_docs: tool_ctx.tool_docs,
        history: recent_history,
        stdin_data,
        project_context: project_ctx,
    };

    if cli.force_search {
        let search_results = perform_search(&http_client, &config.search.engine, &prompt).await?;
        request.search_results = search_results.as_prompt_context();
    }

    let mut generation_count = 0;
    let mut generated =
        generate_command(&backend, &request, cli.verbose, &mut generation_count).await?;

    if cli.dry {
        ui::print_command_animated(&generated.command, generated.explanation.as_deref());
        check_and_prompt_update(update_check).await;
        return Ok(());
    }

    let mut first_display = true;
    loop {
        if first_display {
            ui::print_command_animated(&generated.command, generated.explanation.as_deref());
            first_display = false;
        } else {
            ui::print_command(&generated.command, generated.explanation.as_deref());
        }

        if should_confirm {
            loop {
                match ui::prompt_before_execution(
                    &generated.command,
                    generated.explanation.as_deref(),
                )? {
                    ui::PreAction::Execute => break,
                    ui::PreAction::Edit => {
                        generated.command = ui::prompt_text("Edit command", &generated.command)?;
                        generated.explanation = None;
                        ui::flush_stdin();
                        ui::print_command(&generated.command, None);
                    }
                    ui::PreAction::Refine => {
                        let feedback = ui::prompt_text("Refine", "")?;
                        if feedback.trim().is_empty() {
                            continue;
                        }

                        request.context =
                            Some(format!("User feedback before execution: {feedback}"));
                        generated = generate_command(
                            &backend,
                            &request,
                            cli.verbose,
                            &mut generation_count,
                        )
                        .await?;
                        ui::print_command(&generated.command, generated.explanation.as_deref());
                    }
                    ui::PreAction::Search => {
                        let query = search_query(&prompt, Some(&generated.command), None)?;
                        let search_results =
                            perform_search(&http_client, &config.search.engine, &query).await?;
                        request.search_results = search_results.as_prompt_context();
                        generated = generate_command(
                            &backend,
                            &request,
                            cli.verbose,
                            &mut generation_count,
                        )
                        .await?;
                        ui::print_command(&generated.command, generated.explanation.as_deref());
                    }
                    ui::PreAction::Cancel => return Ok(()),
                }
            }
        }

        eprintln!();
        let execution = execute_command(shell, &generated.command).await?;
        if !execution.is_success() {
            ui::print_exit_status(execution.exit_code, !execution.is_empty());
        } else if execution.is_empty() {
            ui::print_empty_output();
        }

        history::append(&HistoryEntry {
            ts: Utc::now().to_rfc3339(),
            prompt: prompt.clone(),
            command: generated.command.clone(),
            exit_code: execution.exit_code,
            backend: backend.label(),
            iterations: generation_count,
        })?;

        if execution.is_success() && !execution.is_empty() {
            check_and_prompt_update(update_check).await;
            return Ok(());
        }

        if !should_confirm {
            if execution.is_success() {
                return Ok(());
            }
            bail!("command failed with exit {}", execution.exit_code);
        }

        if generation_count >= MAX_ITERATIONS {
            bail!("reached max iterations ({MAX_ITERATIONS})");
        }

        match ui::prompt_after_execution()? {
            ui::PostAction::Refine => {
                let feedback = ui::prompt_text("Refine", "")?;
                let context = build_feedback_context(&generated.command, &execution, &feedback);
                request.context = Some(context);
                generated =
                    generate_command(&backend, &request, cli.verbose, &mut generation_count)
                        .await?;
            }
            ui::PostAction::Search => {
                let query = search_query(&prompt, Some(&generated.command), Some(&execution))?;
                let search_results =
                    perform_search(&http_client, &config.search.engine, &query).await?;
                request.search_results = search_results.as_prompt_context();
                request.context = Some(build_feedback_context(
                    &generated.command,
                    &execution,
                    "User asked for a syntax lookup before retrying.",
                ));
                generated =
                    generate_command(&backend, &request, cli.verbose, &mut generation_count)
                        .await?;
            }
            ui::PostAction::Quit => return Ok(()),
        }
    }
}

async fn open_config() -> Result<()> {
    let path = AppConfig::ensure_config_file()?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        if cfg!(windows) {
            "notepad".to_string()
        } else {
            "vi".to_string()
        }
    });
    let mut parts = shlex::split(&editor).unwrap_or_else(|| vec![editor.clone()]);

    let program = parts
        .first()
        .cloned()
        .context("failed to resolve $EDITOR command")?;
    parts.remove(0);

    let status = Command::new(program)
        .args(parts)
        .arg(&path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .with_context(|| format!("failed to open editor for {}", path.display()))?;

    if !status.success() {
        bail!("editor exited with {}", status.code().unwrap_or_default());
    }

    Ok(())
}

fn show_history(query: Option<&str>, limit: usize) -> Result<()> {
    let entries = history::search(query, limit)?;
    if entries.is_empty() {
        eprintln!("  No history entries found.");
        return Ok(());
    }

    for entry in &entries {
        let status = if entry.exit_code == 0 {
            style("✓").green()
        } else {
            style("✗").red()
        };
        eprintln!(
            "  {} {} {}",
            status,
            style(&entry.command).bold(),
            style(format!("({})", entry.backend)).dim()
        );
        eprintln!("    {} {}", style("↳").dim(), style(&entry.prompt).dim());
    }

    Ok(())
}

async fn generate_command(
    backend: &Backend,
    request: &CommandRequest,
    verbose: bool,
    generation_count: &mut usize,
) -> Result<GeneratedCommand> {
    if *generation_count == 0 && request.context.is_none() {
        if let Some((cmd, explanation)) = cache::lookup(
            &request.prompt,
            &request.os.to_string(),
            &request.shell.to_string(),
        ) {
            ui::status_ok("cached");
            *generation_count += 1;
            return Ok(GeneratedCommand {
                command: cmd,
                explanation,
            });
        }
    }

    if *generation_count >= llm::generation_limit() {
        bail!("reached max generations ({})", llm::generation_limit());
    }

    *generation_count += 1;

    if verbose {
        let (system, user) = llm::render_prompt(request);
        eprintln!("{}", style(format!("backend: {}", backend.label())).dim());
        eprintln!("{}", style(format!("system prompt:\n{system}\n")).dim());
        eprintln!("{}", style(format!("user prompt:\n{user}\n")).dim());
    }

    let sp = ui::generation_spinner(&backend.label());
    let result = backend.generate_command(request).await;
    sp.finish_and_clear();

    let result = match &result {
        Err(e) if e.to_string().contains("429") => {
            ui::status_warn(&format!(
                "Rate limited on {} — waiting 3s...",
                backend.label()
            ));
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let sp = ui::generation_spinner(&backend.label());
            let retry = backend.generate_command(request).await;
            sp.finish_and_clear();
            retry
        }
        _ => result,
    };

    if let Ok(ref cmd) = result {
        if request.context.is_none() {
            cache::store(
                &request.prompt,
                &request.os.to_string(),
                &request.shell.to_string(),
                &cmd.command,
                cmd.explanation.as_deref(),
            );
        }
    }

    result.map(|cmd| validate_command(&cmd))
}

fn validate_command(cmd: &GeneratedCommand) -> GeneratedCommand {
    let first_word = cmd
        .command
        .split(|c: char| c.is_whitespace() || c == '(' || c == ';')
        .find(|w| !w.is_empty())
        .unwrap_or("");

    const BUILTINS: &[&str] = &[
        "if", "for", "while", "do", "done", "then", "fi", "case", "esac", "echo", "cd", "export",
        "source", "eval", "exec", "set", "unset", "true", "false", "test",
    ];

    if first_word.is_empty() || BUILTINS.contains(&first_word) || first_word.contains('/') {
        return cmd.clone();
    }

    if which::which(first_word).is_err() {
        let note = format!("\u{26a0} `{}` not found in PATH", first_word);
        let mut warning = cmd.clone();
        warning.explanation = Some(match &cmd.explanation {
            Some(e) => format!("{e} — {note}"),
            None => note,
        });
        return warning;
    }

    cmd.clone()
}

async fn perform_search(
    http_client: &Client,
    engine: &SearchEngine,
    query: &str,
) -> Result<SearchResults> {
    let sp = ui::spinner(&format!("Searching: {query}"));
    let result = search::search(http_client, *engine, query).await;
    sp.finish_and_clear();
    result
}

async fn execute_command(shell: ShellKind, command: &str) -> Result<ExecutionResult> {
    let mut child = Command::new(shell.program())
        .args(shell.command_args(command))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to execute command with {}", shell.program()))?;

    let child_stdout = child.stdout.take().expect("piped stdout");
    let child_stderr = child.stderr.take().expect("piped stderr");

    let stdout_task = tokio::spawn(forward_and_capture(child_stdout, tokio::io::stdout(), None));
    let stderr_task = tokio::spawn(forward_and_capture(
        child_stderr,
        tokio::io::stderr(),
        Some("\x1b[2m"),
    ));

    let status = child.wait().await?;
    let stdout = stdout_task.await.unwrap_or_else(|_| Ok(String::new()))?;
    let stderr = stderr_task.await.unwrap_or_else(|_| Ok(String::new()))?;

    Ok(ExecutionResult {
        exit_code: status.code().unwrap_or(1),
        stdout,
        stderr,
    })
}

const OUTPUT_INDENT: &[u8] = b"  ";

async fn forward_and_capture<R, W>(
    mut reader: R,
    mut writer: W,
    ansi_prefix: Option<&str>,
) -> Result<String>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut captured = Vec::new();
    let mut buf = [0u8; 4096];
    let mut at_line_start = true;
    let prefix_bytes = ansi_prefix.unwrap_or("").as_bytes();
    let reset = if ansi_prefix.is_some() {
        b"\x1b[0m".as_slice()
    } else {
        b"".as_slice()
    };

    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        captured.extend_from_slice(&buf[..n]);

        let display = String::from_utf8_lossy(&buf[..n]);
        for &byte in display.as_bytes() {
            if at_line_start {
                writer.write_all(OUTPUT_INDENT).await?;
                writer.write_all(prefix_bytes).await?;
                at_line_start = false;
            }
            if byte == b'\n' {
                writer.write_all(reset).await?;
                writer.write_all(&[byte]).await?;
                at_line_start = true;
            } else {
                writer.write_all(&[byte]).await?;
            }
        }
        writer.flush().await?;
    }

    if !at_line_start {
        writer.write_all(reset).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(String::from_utf8_lossy(&captured).to_string())
}

fn search_query(
    prompt: &str,
    command: Option<&str>,
    execution: Option<&ExecutionResult>,
) -> Result<String> {
    let mut initial = prompt.to_string();

    if let Some(command) = command {
        initial.push(' ');
        initial.push_str(command);
    }

    if let Some(execution) = execution
        && let Some(line) = execution
            .stderr
            .lines()
            .find(|line| !line.trim().is_empty())
    {
        initial.push(' ');
        initial.push_str(line.trim());
    }

    ui::prompt_text("Search query", &initial)
}

fn build_feedback_context(
    command: &str,
    execution: &ExecutionResult,
    user_feedback: &str,
) -> String {
    format!(
        "Previous command: {command}\nExit code: {}\nStderr: {}\nStdout (truncated): {}\nUser feedback: {}",
        execution.exit_code,
        execution.stderr_head(),
        execution.stdout_head(),
        if user_feedback.trim().is_empty() {
            "none"
        } else {
            user_feedback.trim()
        }
    )
}

fn detect_project_context() -> Option<String> {
    let mut parts = Vec::new();
    let cwd = std::env::current_dir().ok()?;

    if cwd.join("Cargo.toml").exists() {
        parts.push("Rust project (Cargo.toml)");
    }
    if cwd.join("package.json").exists() {
        parts.push("Node.js project (package.json)");
    }
    if cwd.join("pyproject.toml").exists() || cwd.join("setup.py").exists() {
        parts.push("Python project");
    }
    if cwd.join("go.mod").exists() {
        parts.push("Go project (go.mod)");
    }
    if cwd.join("Dockerfile").exists()
        || cwd.join("docker-compose.yml").exists()
        || cwd.join("compose.yml").exists()
    {
        parts.push("Docker containerized");
    }
    if cwd.join("Makefile").exists() {
        parts.push("Has Makefile");
    }
    if cwd.join(".github").exists() {
        parts.push("GitHub Actions CI");
    }
    if cwd.join("Gemfile").exists() {
        parts.push("Ruby project (Gemfile)");
    }
    if cwd.join("pom.xml").exists() || cwd.join("build.gradle").exists() {
        parts.push("Java/JVM project");
    }
    if cwd.join("terraform.tf").exists() || cwd.join("main.tf").exists() {
        parts.push("Terraform project");
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join(", "))
}

async fn run_explain(backend: &Backend, command: &str) -> Result<()> {
    ui::print_command(command, None);

    let system = concat!(
        "You explain shell commands clearly and concisely.\n",
        "Break down each flag and argument.\n",
        "Keep it casual and helpful — like a senior dev explaining to a colleague.\n",
        "Output plain text only, no markdown formatting.\n",
        "Start with a one-line summary, then break down each part.",
    );

    let sp = ui::generation_spinner(&backend.label());
    let explanation = backend
        .call(system, &format!("Explain this command:\n{command}"))
        .await?;
    sp.finish_and_clear();

    ui::print_explanation(&explanation);
    Ok(())
}

async fn run_script(backend: &Backend, request: &CommandRequest, verbose: bool) -> Result<()> {
    let system = format!(
        r#"You are a shell script generator for {} ({}).
Write a complete, well-structured shell script that accomplishes the user's task.
Rules:
- Start with the appropriate shebang (#!/bin/bash, #!/usr/bin/env python3, etc.)
- Include error handling (set -euo pipefail for bash)
- Use meaningful variable names
- The script should be ready to save and execute
- Output ONLY the script, no explanation, no markdown fences
- Use tools and flags available on {} only"#,
        request.os, request.shell, request.os
    );

    let user = format!("Task: {}", request.prompt);

    if verbose {
        eprintln!("{}", style(format!("system prompt:\n{system}\n")).dim());
    }

    let sp = ui::generation_spinner(&backend.label());
    let raw = backend.call(&system, &user).await?;
    sp.finish_and_clear();

    let script = raw
        .trim()
        .strip_prefix("```bash\n")
        .or_else(|| raw.trim().strip_prefix("```sh\n"))
        .or_else(|| raw.trim().strip_prefix("```\n"))
        .unwrap_or(raw.trim());
    let script = script
        .strip_suffix("\n```")
        .or_else(|| script.strip_suffix("```"))
        .unwrap_or(script);

    println!("{}", script.trim());
    Ok(())
}

async fn run_recipe(backend: &Backend, request: &CommandRequest, verbose: bool) -> Result<()> {
    let system = format!(
        r#"You are a shell workflow generator for {} ({}).
Generate a step-by-step recipe — a numbered list of commands to accomplish the task.
Format:
Step 1: <brief description>
$ <command>

Step 2: <brief description>
$ <command>

Rules:
- Each step has ONE command (can use && or | within the command)
- Keep descriptions short (one line)
- Steps should be in the correct execution order
- Use tools and flags available on {} only
- Output ONLY the steps, no introduction or conclusion
- Maximum 10 steps
- Use markdown-free plain text only"#,
        request.os, request.shell, request.os
    );

    let user = format!("Task: {}", request.prompt);

    if verbose {
        eprintln!("{}", style(format!("system prompt:\n{system}\n")).dim());
    }

    let sp = ui::generation_spinner(&backend.label());
    let raw = backend.call(&system, &user).await?;
    sp.finish_and_clear();

    let text = raw.trim();
    let text = text
        .strip_prefix("```\n")
        .unwrap_or(text)
        .strip_suffix("\n```")
        .unwrap_or(text);

    eprintln!();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Step ") || trimmed.starts_with("step ") {
            if let Some((_num, desc)) = trimmed.split_once(": ") {
                eprintln!("  {} {}", style(_num).cyan().bold(), style(desc).white());
            } else {
                eprintln!("  {}", style(trimmed).cyan().bold());
            }
        } else if let Some(cmd) = trimmed.strip_prefix("$ ") {
            eprintln!("  {}", style(format!("  ❯ {cmd}")).white().bold());
        } else if !trimmed.is_empty() {
            eprintln!("  {}", style(trimmed).dim());
        } else {
            eprintln!();
        }
    }
    eprintln!();

    Ok(())
}

fn run_demo() {
    ui::banner();

    let demos = [
        (
            "compress all PNGs in this directory",
            "find . -name '*.png' -exec pngquant --quality=65-80 {} \\;",
            "compresses PNGs with pngquant for web optimization",
        ),
        (
            "show disk usage by folder sorted by size",
            "du -d 1 -h . | sort -hr | head -20",
            "shows top 20 largest directories",
        ),
        (
            "find all TODO comments in source code",
            "grep -rn 'TODO\\|FIXME\\|HACK' --include='*.rs' --include='*.ts' --include='*.py' .",
            "searches for TODO/FIXME/HACK annotations",
        ),
        (
            "extract audio from video.mp4 as mp3 320kbps",
            "ffmpeg -i video.mp4 -vn -b:a 320k audio.mp3",
            "extracts audio track at 320kbps, no video",
        ),
        (
            "create a git patch of the last 3 commits",
            "git format-patch -3 HEAD",
            "creates patch files for the last 3 commits",
        ),
    ];

    for (prompt, command, explanation) in &demos {
        eprintln!();
        eprintln!("  {} {}", style("❯").cyan().bold(), style(prompt).italic());
        ui::print_command(command, Some(explanation));
    }

    eprintln!();
    eprintln!(
        "  {} Run {} to get started!",
        style("→").green().bold(),
        style("eai setup").cyan().bold()
    );
    eprintln!();
}

fn detect_stdin_nature(data: &str) -> &'static str {
    let lower = data.to_lowercase();
    if lower.contains("error")
        || lower.contains("exception")
        || lower.contains("traceback")
        || lower.contains("panic")
        || lower.contains("failed")
    {
        "error output"
    } else if lower.contains("\"") && (lower.contains("{") || lower.contains("[")) {
        "JSON data"
    } else if data
        .lines()
        .next()
        .map(|l| l.contains(','))
        .unwrap_or(false)
        && data.lines().count() > 1
    {
        "CSV data"
    } else if lower.contains("<html") || lower.contains("<!doctype") {
        "HTML content"
    } else if data.lines().all(|l| {
        l.starts_with('#')
            || l.starts_with('-')
            || l.starts_with('>')
            || l.starts_with('`')
            || l.trim().is_empty()
    }) {
        "Markdown content"
    } else {
        "text"
    }
}

async fn check_and_prompt_update(handle: tokio::task::JoinHandle<Option<String>>) {
    let latest = match handle.await {
        Ok(Some(v)) => v,
        _ => return,
    };

    if !std::io::stderr().is_terminal() {
        return;
    }

    let Ok(wants_update) = update::prompt_update(&latest) else {
        return;
    };

    if !wants_update {
        return;
    }

    let Some((program, args)) = update::install_command() else {
        ui::status_warn(
            "download the latest version at https://github.com/feliperbroering/eai/releases/latest",
        );
        return;
    };

    eprintln!();
    let status = tokio::process::Command::new(program)
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await;

    match status {
        Ok(s) if s.success() => ui::print_update_success(&latest),
        _ => ui::status_warn(
            "update failed — try manually: curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash",
        ),
    }
}

async fn read_stdin_if_piped() -> Option<String> {
    if std::io::stdin().is_terminal() {
        return None;
    }

    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut buf);
        let _ = tx.send(buf);
    });

    let result = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .ok()
        .map(|buf| String::from_utf8_lossy(&buf).to_string());

    result.and_then(|text| {
        if text.trim().is_empty() {
            return None;
        }
        let max_chars = 4000;
        if text.chars().count() > max_chars {
            Some(format!(
                "{}...\n(truncated to first {max_chars} chars)",
                text.chars().take(max_chars).collect::<String>()
            ))
        } else {
            Some(text)
        }
    })
}

fn print_shell_integration(shell: ShellKind) {
    match shell {
        ShellKind::Zsh => {
            println!(
                r#"# eai shell integration — add to .zshrc:
# eval "$(eai init zsh)"

_eai_widget() {{
  local buf="${{BUFFER}}"
  if [[ -z "$buf" ]]; then
    BUFFER="eai "
    CURSOR=$#BUFFER
  else
    local result
    result=$(eai --dry --no-confirm "$buf" 2>/dev/null | tail -1 | sed 's/^[[:space:]]*//' | sed 's/^❯ //')
    if [[ -n "$result" ]]; then
      BUFFER="$result"
      CURSOR=$#BUFFER
    fi
  fi
  zle redisplay
}}
zle -N _eai_widget
bindkey '^E' _eai_widget"#
            );
        }
        ShellKind::Bash => {
            println!(
                r#"# eai shell integration — add to .bashrc:
# eval "$(eai init bash)"

_eai_readline() {{
  local buf="$READLINE_LINE"
  if [[ -z "$buf" ]]; then
    READLINE_LINE="eai "
    READLINE_POINT=${{#READLINE_LINE}}
  else
    local result
    result=$(eai --dry --no-confirm "$buf" 2>/dev/null | tail -1 | sed 's/^[[:space:]]*//' | sed 's/^❯ //')
    if [[ -n "$result" ]]; then
      READLINE_LINE="$result"
      READLINE_POINT=${{#READLINE_LINE}}
    fi
  fi
}}
bind -x '"\C-e": _eai_readline'"#
            );
        }
        ShellKind::Fish => {
            println!(
                r#"# eai shell integration — add to config.fish:
# eai init fish | source

function _eai_widget
  set -l buf (commandline)
  if test -z "$buf"
    commandline "eai "
    commandline -C (string length "eai ")
  else
    set -l result (eai --dry --no-confirm "$buf" 2>/dev/null | tail -1 | string trim | string replace '❯ ' '')
    if test -n "$result"
      commandline "$result"
      commandline -C (string length "$result")
    end
  end
end
bind \ce _eai_widget"#
            );
        }
        _ => {
            eprintln!(
                "Shell integration is not available for {}. Supported: zsh, bash, fish.",
                shell
            );
        }
    }
}

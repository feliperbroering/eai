use std::io::{IsTerminal, Read};
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use reqwest::Client;
use tokio::process::Command;

use console::style;

use crate::{
    cli::{Cli, Commands},
    config::{AppConfig, SearchEngine},
    history,
    llm::{self, Backend},
    search::{self, SearchResults},
    setup, tool_context, update,
    types::{CommandRequest, ExecutionResult, GeneratedCommand, HistoryEntry, OsKind, ShellKind},
    ui,
};

const MAX_ITERATIONS: usize = 5;

pub async fn run(cli: Cli) -> Result<()> {
    let config = AppConfig::load()?;

    match cli.command {
        Some(Commands::Setup) => setup::run_setup().await,
        Some(Commands::Config) => open_config().await,
        Some(Commands::History { search, limit }) => show_history(search.as_deref(), limit),
        None => {
            if cli.prompt.is_empty() {
                bail!("prompt is required — try: eai list files modified today");
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
    let stdin_data = read_stdin_if_piped().await;
    let http_client = Client::builder()
        .user_agent(format!("eai/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    let update_check = tokio::spawn({
        let client = http_client.clone();
        async move { update::check(&client).await }
    });

    ui::banner();

    let prompt = cli.prompt.join(" ");

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

    let mut request = CommandRequest {
        prompt: prompt.clone(),
        shell,
        os,
        context: None,
        search_results: None,
        tool_docs: tool_ctx.tool_docs,
        history: recent_history,
        stdin_data,
    };

    if cli.force_search {
        let search_results = perform_search(&http_client, &config.search.engine, &prompt).await?;
        request.search_results = search_results.as_prompt_context();
    }

    let mut generation_count = 0;
    let mut generated =
        generate_command(&backend, &request, cli.verbose, &mut generation_count).await?;

    if cli.dry {
        ui::print_command(&generated.command, generated.explanation.as_deref());
        check_and_prompt_update(update_check).await;
        return Ok(());
    }

    loop {
        ui::print_command(&generated.command, generated.explanation.as_deref());

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
            ui::print_failure(execution.exit_code);
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
        println!("No history entries found.");
        return Ok(());
    }

    for entry in entries {
        println!(
            "{} [{}] exit={} iter={} {}",
            entry.ts, entry.backend, entry.exit_code, entry.iterations, entry.command
        );
        println!("  {}", entry.prompt);
    }

    Ok(())
}

async fn generate_command(
    backend: &Backend,
    request: &CommandRequest,
    verbose: bool,
    generation_count: &mut usize,
) -> Result<GeneratedCommand> {
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
    result
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

async fn check_and_prompt_update(
    handle: tokio::task::JoinHandle<Option<String>>,
) {
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
        ui::status_warn(&format!(
            "download the latest version at https://github.com/feliperbroering/eai/releases/latest"
        ));
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
        _ => ui::status_warn("update failed — try manually: curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash"),
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

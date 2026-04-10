# eai — Agent Guidelines

## Project

Rust CLI that converts natural language to shell commands using LLMs. Single binary, no runtime dependencies.

## Architecture

```
src/
  main.rs          — entry point, tokio runtime
  cli.rs           — clap arg definitions (Cli, Commands)
  app.rs           — orchestration: prompt → generate → confirm → execute loop
  config.rs        — TOML config at ~/.config/eai/config.toml
  setup.rs         — interactive onboarding wizard (eai setup)
  ui.rs            — all terminal rendering: banner, gradient box, action bar, spinners
  types.rs         — shared types: BackendKind, ShellKind, CommandRequest, GeneratedCommand
  tool_context.rs  — detects CLI tools in prompt, loads their docs (tldr/--help)
  search.rs        — DuckDuckGo web search for syntax lookups
  history.rs       — append-only JSONL history at ~/.local/share/eai/history.jsonl
  llm/
    mod.rs         — Backend trait, prompt building, response parsing, backend resolution
    openai.rs      — OpenAI-compatible client (also used for Groq, OpenRouter)
    ollama.rs      — Ollama local client
    claude.rs      — Claude CLI wrapper
```

## Key patterns

- All UI output goes to **stderr** (`eprintln!`). Only command execution output goes to stdout.
- RGB gradients use raw ANSI escapes (`\x1b[38;2;R;G;Bm`) with `colors_enabled_stderr()` fallback.
- `GeneratedCommand` has `.command` (the shell command) and `.explanation` (optional `//` comment from LLM).
- `parse_response` in `llm/mod.rs` extracts command + explanation from LLM output. Tolerant of markdown fences.
- Tool extraction uses a separate LLM call. Filter: ASCII-only, `is_noise_word()` blocklist, max 5 tools.
- Pipe mode: `read_stdin_if_piped()` reads up to 4K chars from stdin when not a terminal.
- The `build_openai_compat()` helper in `llm/mod.rs` handles both Groq and OpenAI backends.

## Conventions

- Rust edition 2024 — `gen` is a reserved keyword, `env::set_var` requires `unsafe`.
- No comments that narrate what code does. Only explain non-obvious intent.
- UI consistency: use `ui::spinner()`, `ui::status_ok()`, `ui::status_warn()` for all status output.
- Tests live in `llm/mod.rs` (`#[cfg(test)]`). Run with `cargo test`.
- Build with `cargo build --release`. Target binary: `target/release/eai`.

## When modifying

- After editing, run `cargo check` then `cargo test`.
- If adding a new provider, add it to `setup.rs` PROVIDERS array and use `build_openai_compat()` in `llm/mod.rs`.
- If adding a new CLI flag, add to `cli.rs` Cli struct and handle in `app.rs` `run_prompt()`.
- Keep `is_noise_word()` in `tool_context.rs` updated when LLM returns unexpected tool names.

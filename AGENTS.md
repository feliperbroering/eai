# eai — Agent Guidelines

## Project

Rust CLI that converts natural language to shell commands using LLMs. Single binary, no runtime dependencies.

## Architecture

```
src/
  main.rs          — entry point, tokio runtime
  cli.rs           — clap arg definitions (Cli, Commands, shell completions)
  app.rs           — orchestration: prompt → generate → confirm → execute loop, modes (script/recipe/explain/demo)
  config.rs        — TOML config at ~/.config/eai/config.toml
  setup.rs         — interactive onboarding wizard (eai setup) + optional Tavily setup
  ui.rs            — all terminal rendering: banner, gradient box, action bar, spinners, typewriter animation
  types.rs         — shared types: BackendKind, ShellKind, CommandRequest, GeneratedCommand
  tool_context.rs  — tool detection, discovery, package registry verification, install flow
  tldr.rs          — embedded tldr-pages lookup (7000+ commands, zstd-compressed, O(1) HashMap)
  cache.rs         — file-based prompt→command cache at ~/.cache/eai/cache.jsonl
  aliases.rs       — command bookmarks at ~/.config/eai/aliases.json
  search.rs        — web search: Tavily (preferred) or DuckDuckGo (fallback)
  history.rs       — append-only JSONL history at ~/.local/share/eai/history.jsonl
  llm/
    mod.rs         — Backend trait, prompt building, response parsing, OS flag correction, backend resolution
    openai.rs      — OpenAI-compatible client (also used for Groq, OpenRouter)
    ollama.rs      — Ollama local client
    claude.rs      — Claude CLI wrapper
build.rs           — downloads tldr-pages at build time, parses markdown, serializes with bincode+zstd
```

## Key patterns

- All UI output goes to **stderr** (`eprintln!`). Only command execution output goes to stdout.
- RGB gradients use raw ANSI escapes (`\x1b[38;2;R;G;Bm`) with `colors_enabled_stderr()` fallback.
- `GeneratedCommand` has `.command` (the shell command) and `.explanation` (optional `//` comment from LLM).
- `parse_response` in `llm/mod.rs` extracts command + explanation from LLM output. Tolerant of markdown fences. Applies OS-specific flag corrections.
- Tool extraction uses a separate LLM call. Filter: ASCII-only, `is_noise_word()` blocklist, max 5 tools. Skipped for coreutils-only prompts.
- Pipe mode: `read_stdin_if_piped()` reads up to 4K chars from stdin; auto-detects content type (JSON, CSV, error output, etc.).
- The `build_openai_compat()` helper in `llm/mod.rs` handles both Groq and OpenAI backends.
- API keys read from env vars; `env_var()` in `llm/mod.rs` falls back to reading from shell profile.
- Tool docs resolution: embedded tldr-pages (instant O(1) lookup) + `--help` output combined. Even tools not installed get tldr docs.
- Tool discovery: when all extracted tools are missing, searches web + LLM suggests alternatives, verifies against package registries (brew, PyPI, npm, crates.io), offers interactive install.
- Search hierarchy: Tavily (if `TAVILY_API_KEY` set) → DuckDuckGo (fallback). Configured via `search.engine` in config.
- Setup hides API key input (Password prompt) and validates keys with retry on 401.
- Prompt→command cache (hash-based) avoids repeated LLM calls for identical prompts.
- Post-generation validation checks if the main binary exists in PATH and warns if not.
- Project context detection (Cargo.toml, package.json, go.mod, etc.) enriches the LLM prompt.
- 429 rate-limit auto-retry (3s wait) before surfacing the error.
- Typewriter animation on first command display (TTY only).

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
- If adding a new search engine, add variant to `SearchEngine` in `config.rs` and implement in `search.rs`.
- If adding a new package registry, add `check_<registry>()` in `tool_context.rs` and wire into `verify_suggestions()`.

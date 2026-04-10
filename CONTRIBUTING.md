# Contributing to eai

Thanks for your interest in contributing! Here's how to get started.

## Development setup

```bash
git clone https://github.com/feliperbroering/eai
cd eai
cargo build --release
cargo test
```

Requires Rust 1.85+ (edition 2024).

## Making changes

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Run `cargo check && cargo test` before submitting
4. Open a pull request

## Architecture

See [AGENTS.md](AGENTS.md) for a full overview. Key files:

- `src/app.rs` — main orchestration loop
- `src/llm/` — LLM backends (Groq, OpenAI, Ollama, Claude CLI)
- `src/tool_context.rs` — tool detection, discovery, and installation
- `src/search.rs` — web search (Tavily / DuckDuckGo)
- `src/ui.rs` — all terminal rendering
- `src/setup.rs` — onboarding wizard

## Conventions

- All UI output goes to **stderr**. Only command execution output goes to stdout.
- No comments that narrate what code does. Only explain non-obvious intent.
- Use `ui::spinner()`, `ui::status_ok()`, `ui::status_warn()` for status output.
- Edition 2024: `gen` is reserved, `env::set_var` requires `unsafe`.

## Adding a new LLM provider

1. Add to `setup.rs` PROVIDERS array
2. Use `build_openai_compat()` in `llm/mod.rs` if OpenAI-compatible

## Adding a new search engine

1. Add variant to `SearchEngine` in `config.rs`
2. Implement in `search.rs`

## Adding a new package registry

1. Add `check_<registry>()` in `tool_context.rs`
2. Wire into `verify_suggestions()` and `check_registry()`

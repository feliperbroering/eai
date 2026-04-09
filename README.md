# eai

**E aí?** Describe what you want, get the command, confirm, run.

Uses cheap/local LLMs (Ollama, Groq, GLM) because generating bash doesn't need premium LLM. If the command fails, it learns from the error and tries again.

```
$ eai "find all rust files bigger than 1mb"
▶ find . -name "*.rs" -size +1024k
[Enter ✓ | e: edit | r: refine | s: search | Ctrl+C ✗]
```

## Why

You know what you want to do. You don't remember the flags. You alt-tab to a browser, google it, copy the command, come back. `eai` keeps you in the terminal.

Unlike other AI shell tools, `eai`:

- **Doesn't need an API key** — runs on Ollama locally by default
- **Doesn't burn expensive tokens** — shell commands are trivial for small models
- **Has a feedback loop** — if the command fails, the error goes back to the LLM automatically
- **Can search the web** — press `s` to look up syntax before running

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash
```

Or build from source:

```bash
cargo install --git https://github.com/feliperbroering/eai
```

For the default (free, local) experience, install [Ollama](https://ollama.com) and pull a small model:

```bash
ollama pull qwen3:4b
```

No Ollama? Set `GROQ_API_KEY` for free cloud inference via [Groq](https://console.groq.com/keys).

## Usage

```bash
# Basic — generate, confirm, run
eai "compress all PNGs in this directory"

# Force a specific backend
eai -b groq "undo last 3 git commits"

# Force a specific model
eai -m qwen3:8b "docker remove dangling images"

# Dry run — show command, don't offer to execute
eai --dry "kill whatever is on port 3000"

# Force web search before generating
eai --search "ripgrep exclude node_modules"

# Verbose — show the prompt sent to the LLM
eai -v "tar extract gz keeping permissions"
```

## How it works

```
 you type                LLM generates              you confirm
┌──────────┐           ┌──────────────┐           ┌──────────────┐
│ eai "..." │──prompt──▶│ qwen3:4b     │──command──▶│ ▶ rm -rf ... │
└──────────┘           └──────────────┘           │ [Enter/Ctrl+C]│
                                                   └──────┬───────┘
                                                          │
                                          ┌───────────────▼────────────────┐
                                          │ Enter: execute                 │
                                          │ e: edit command before running │
                                          │ r: refine (add context, retry) │
                                          │ s: web search, then retry      │
                                          │ Ctrl+C: cancel                 │
                                          └────────────────────────────────┘
```

### Feedback loop

If the command fails (non-zero exit code) or returns empty output, `eai` offers to retry. The error output and your feedback go back to the LLM as context:

```
$ eai "find all .env files excluding node_modules"
▶ find . -name ".env" --exclude node_modules
[Enter ✓ | e: edit | r: refine | s: search | Ctrl+C ✗]
> (Enter)
find: unknown predicate `--exclude'
⚠ Command failed (exit 1). What now?
[r: refine with context | s: search the web | q: quit]
> r
▶ find . -name ".env" -not -path "*/node_modules/*"
[Enter ✓ | ...]
```

Up to 5 iterations to prevent infinite loops.

### Web search

Press `s` at any prompt to search the web (via DuckDuckGo, no API key needed). Results are injected into the LLM prompt and the command is regenerated:

```
$ eai "use ffmpeg to extract audio from video.mp4 as mp3"
▶ ffmpeg -i video.mp4 -vn -acodec mp3 output.mp3
[Enter ✓ | e: edit | r: refine | s: search | Ctrl+C ✗]
> s
🔍 Searching: "ffmpeg extract audio mp3 from mp4"
▶ ffmpeg -i video.mp4 -vn -acodec libmp3lame -q:a 2 output.mp3
[Enter ✓ | ...]
```

## Backends

`eai` auto-detects the best available backend in this order:

| # | Backend | Config needed | Cost |
|---|---------|---------------|------|
| 1 | **Ollama** (default) | Ollama running locally | Free |
| 2 | **Groq** | `GROQ_API_KEY` env var | Free tier |
| 3 | **OpenAI-compatible** | `OPENAI_API_KEY` + `OPENAI_BASE_URL` | Varies |
| 4 | **Claude Code CLI** | `claude` in PATH | Uses subscription |

Override with `-b`:

```bash
eai -b groq "list all listening ports"
eai -b claude-cli "complex multi-step pipeline"
```

## Configuration

Config lives at `~/.config/eai/config.toml`:

```toml
[default]
backend = "ollama"       # ollama | groq | openai | claude-cli
shell = "zsh"            # auto-detected by default
confirm = true

[ollama]
model = "qwen3:4b"
url = "http://localhost:11434"

[groq]
api_key_env = "GROQ_API_KEY"
model = "llama-3.3-70b-versatile"

[openai]
api_key_env = "OPENAI_API_KEY"
base_url = "https://open.bigmodel.cn/api/paas/v4"  # GLM-4 example
model = "glm-4-flash"

[claude-cli]
# Uses `claude -p` — no additional config needed

[search]
enabled = true
engine = "ddg"           # ddg (default, no API key) | serper
```

Edit with:

```bash
eai config
```

## History

Every execution is logged to `~/.local/share/eai/history.jsonl`:

```json
{
  "ts": "2026-04-09T14:30:00Z",
  "prompt": "find rust files bigger than 1mb",
  "command": "find . -name '*.rs' -size +1024k",
  "exit_code": 0,
  "backend": "ollama/qwen3:4b",
  "iterations": 2
}
```

Browse with:

```bash
eai history
eai history --search "docker"
```

## CLI Reference

```
eai 0.1.0
E aí? Natural language to shell commands.

USAGE:
    eai <PROMPT>
    eai [FLAGS] <PROMPT>
    eai <SUBCOMMAND>

FLAGS:
    -b, --backend <BACKEND>    ollama | groq | openai | claude-cli
    -m, --model <MODEL>        Override model for the chosen backend
    -s, --shell <SHELL>        zsh | bash | fish (default: auto)
    --dry                      Show command without offering execution
    --no-confirm               Execute without confirmation (use with caution)
    --search                   Force web search before generating
    -v, --verbose              Show the prompt sent to the LLM

SUBCOMMANDS:
    config                     Open config in $EDITOR
    history                    Show recent commands
    history --search <QUERY>   Search command history
```

## Design decisions

**Why local LLMs by default?** Generating shell commands is trivial for even small models. Paying for GPT-4o to produce `find . -name "*.rs"` is wasteful. Ollama + qwen3:4b runs in <1s with zero cost.

**Why Rust?** A zsh function works for the basic case, but doesn't scale to config files, multiple backends, history, web search, and feedback loops. Rust gives us a single self-contained binary that distributes via `cargo install`.

**Why DuckDuckGo for search?** Zero API key, zero cost, zero tracking. Good enough for "ripgrep exclude syntax" or "macOS find -size format".

**Why not Claude/GPT as default?** Higher latency (2-5s vs <1s), consumes subscription tokens, overkill for the task. But it's an excellent premium fallback when the local model gets it wrong — just press `r` and switch with `-b claude-cli`.

## Inspired by

- [llm-cmd](https://github.com/simonw/llm-cmd) by Simon Willison — the original "LLM generates shell commands" plugin
- [ai-shell](https://github.com/BuilderIO/ai-shell) by Builder.io — similar concept, Node.js + OpenAI API key required
- [shell-ai](https://github.com/ricklamers/shell-ai) — LangChain-based, Python, API key required

`eai` differs by defaulting to free local models, having a built-in feedback loop, and supporting web search as a fallback — all in a single Rust binary with no runtime dependencies.

## License

MIT

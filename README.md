<p align="center">
<pre>
  ███████╗ █████╗ ██╗    ╭─────╮
  ██╔════╝██╔══██╗██║    │ >_  │
  █████╗  ███████║██║    ╰─────╯
  ██╔══╝  ██╔══██║██║
  ███████╗██║  ██║██║
  ╚══════╝╚═╝  ╚═╝╚═╝
</pre>
</p>

<h3 align="center">don't memorize 1000 flags. just prompt it.</h3>

<p align="center">
  <a href="https://github.com/feliperbroering/eai/actions/workflows/ci.yml"><img src="https://github.com/feliperbroering/eai/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/eai"><img src="https://img.shields.io/crates/v/eai.svg" alt="crates.io"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  <a href="https://github.com/feliperbroering/eai/releases"><img src="https://img.shields.io/github/v/release/feliperbroering/eai" alt="Release"></a>
</p>

<p align="center">
  Describe what you want in plain English. Get the shell command. Confirm. Run.<br>
  Single Rust binary. Free by default (Gemini/Groq/Ollama). Works in any language.
</p>

---

```bash
# ffmpeg (time slice + codec flags)
eai "extract audio from video.mp4 as mp3 320kbps, only from 1:30 to 3:45"
▶ ffmpeg -i video.mp4 -ss 00:01:30 -to 00:03:45 -vn -acodec libmp3lame -b:a 320k output.mp3

# find + sed across recursive .env files
eai "replace all occurrences of localhost:3000 with api.prod.com in every .env file recursively"
▶ find . -name ".env" -exec sed -i '' 's/localhost:3000/api.prod.com/g' {} +

# rsync with excludes, compression, and permissions
eai "sync my local ./dist to server 192.168.1.50:/var/www excluding node_modules and .git, with compression, preserving permissions"
▶ rsync -avz --exclude='node_modules' --exclude='.git' ./dist/ user@192.168.1.50:/var/www/

# iptables NAT redirect
eai "redirect all incoming traffic on port 80 to port 3000"
▶ sudo iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-port 3000

# git log analysis with shell pipeline
eai "show who committed the most in the last 7 days"
▶ git log --since="7 days ago" --format="%an" | sort | uniq -c | sort -rn
```

## Quick Start

### macOS / Linux

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash

# Setup (30 seconds — picks a free provider for you)
eai setup

# Go
eai "compress all PNGs in this directory"
```

### Windows (PowerShell)

```powershell
# Install (downloads precompiled binary from latest GitHub Release)
iwr https://raw.githubusercontent.com/feliperbroering/eai/main/install.ps1 -UseBasicParsing | iex

# Setup (30 seconds — picks a free provider for you)
eai setup

# Go
eai "compress all PNGs in this directory"
```

### Alternative (all platforms)

```bash
# Requires Rust toolchain installed
cargo install --git https://github.com/feliperbroering/eai
```

## Usage

```bash
eai "kill whatever is on port 3000"
eai "find rust files bigger than 1mb modified this week"
eai "undo last 3 git commits keeping changes"
eai "convert this video to gif at 10fps"
```

### Pipe mode — feed data as context

```bash
cat error.log | eai "what command fixes this"
git diff --stat | eai "write a commit message"
cat data.json | eai "extract all email addresses"
docker logs app | eai "find the error pattern"
```

### Explain mode — reverse eai

```bash
eai --explain "awk '{print \$NF}' access.log | sort | uniq -c | sort -rn"
eai --wtf "tar -xzf archive.tar.gz -C /tmp"
```

### Tool discovery

When you ask for a tool that isn't installed, eai searches the web, finds real alternatives, verifies them against package registries (brew, PyPI, npm, crates.io), and offers to install:

```
  🔍 Tools found for your task:

  1.  docling
     Converts PDF, DOCX, HTML to Markdown with table support
     https://github.com/DS4SD/docling
     v2.31.0
     pip install docling

  2.  marker
     Fast PDF to Markdown converter with OCR
     https://github.com/VikParuchuri/marker
     v1.3.2
     pip install marker-pdf

  ╭──────────────────────────────────────────────╮
  │  ● install 1-2  │  ● skip s  │  ● cancel ^C  │
  ╰──────────────────────────────────────────────╯
```

### Flags

```bash
eai --dry "..."          # show command, don't run
eai --explain "..."      # explain a command (alias: --wtf)
eai --search "..."       # force web search before generating
eai -b groq "..."        # force a specific backend
eai -m llama-3.3-70b-versatile "..."  # force a specific model
eai --no-confirm "..."   # skip confirmation (yolo)
eai -v "..."             # show system/user prompts (debug)
```

### Subcommands

```bash
eai setup                # interactive provider setup wizard
eai config               # open config in $EDITOR
eai history              # show recent commands
eai history --search docker
```

## How it works

1. You type `eai "..."` in plain English (or any language)
2. eai detects CLI tools in your prompt and loads their docs (tldr/--help)
3. If the tool isn't installed, eai discovers alternatives via web search, verifies them against package registries, and offers to install
4. The LLM generates the command + a brief explanation
5. You confirm, edit (with placeholder values you can customize), refine, or search before running
6. If it fails, the error goes back to the LLM — auto-retry up to 5x

## Providers

Run `eai setup` to connect your preferred backend:

| Provider | Model | Cost |
|----------|-------|------|
| **Gemini** ★ | Gemini 2.5 Flash Lite | Free (Google AI Studio) |
| **Groq** | Llama 3.3 70B | Free (14K req/day) |
| **OpenRouter** | GPT-4o, Claude, Gemini... | 1 key, all models |
| **OpenAI** | gpt-4o-mini | Pay per use |
| **Ollama** | qwen3:4b (local) | Free, no API key |
| **Claude CLI** | Claude (via `claude` CLI) | Requires Claude CLI |
| **Custom API** | GLM, Kimi, DeepSeek... | Any OpenAI-compatible |

Override per-command: `-b gemini`, `-b groq`, `-b openai`, `-b ollama`, `-b claude-cli`.

## Web Search

eai can search the web for syntax lookups and tool discovery:

| Engine | Quality | Setup |
|--------|---------|-------|
| **Tavily** ★ | High (AI-optimized) | Free — 1000 searches/month, no credit card |
| **DuckDuckGo** | Basic | Zero config (default fallback) |

`eai setup` offers to configure Tavily automatically.

## Config

`eai config` opens your config in `$EDITOR`:

```toml
[default]
backend = "gemini"
confirm = true

[gemini]
api_key_env = "GEMINI_API_KEY"
model = "gemini-2.5-flash-lite"

[groq]
api_key_env = "GROQ_API_KEY"
model = "llama-3.3-70b-versatile"

[ollama]
model = "qwen3:4b"

[search]
engine = "tavily"    # or "ddg" (default)
```

Config location:
- Linux: `~/.config/eai/config.toml`
- macOS: `~/Library/Application Support/eai/config.toml`
- Windows: `%APPDATA%\eai\config.toml`

## E2E Tests (Robot Framework)

```bash
python -m pip install -r tests/e2e/requirements.txt
robot tests/e2e/eai.robot
```

The suite uses a mocked `claude` CLI and validates end-to-end flows for:
- command generation in `--dry`
- command execution in shell
- `--explain`
- history persistence

## vs other tools

| Feature | eai | llm | aichat | shell-gpt |
|---|---|---|---|---|
| **Pipe context (stdin)** | ✓ | ✓ | ✓ | ✓ |
| **Explain mode** | ✓ (`--wtf`) | ✗ | ✗ | partial |
| **Free by default** | ✓ (Gemini/Groq/Ollama) | ✗ (OpenAI) | ✗ (Needs API) | ✗ (OpenAI) |
| **Auto-retry on error** | ✓ (feeds stderr back) | ✗ | ✗ | ✗ |
| **Web search** | ✓ (Tavily/DDG) | ✗ (plugins) | partial | ✗ (plugins) |
| **Tool doc detection** | ✓ (auto tldr/--help) | ✗ | ✗ | ✗ |
| **Tool discovery + install** | ✓ (registry-verified) | ✗ | ✗ | ✗ |
| **Setup wizard** | ✓ (30s) | ✗ | ✓ | ✗ |
| **Single binary** | ✓ (Rust) | Python | ✓ (Rust) | Python |

## Contributing

Contributions are welcome! See [AGENTS.md](AGENTS.md) for architecture overview and conventions.

```bash
git clone https://github.com/feliperbroering/eai
cd eai
cargo build --release
cargo test
```

## License

[MIT](LICENSE)

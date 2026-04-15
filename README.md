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
# ffmpeg — time slice + codec flags
eai "extract audio from video.mp4 as mp3 320kbps, only from 1:30 to 3:45"
▶ ffmpeg -i video.mp4 -ss 00:01:30 -to 00:03:45 -vn -acodec libmp3lame -b:a 320k output.mp3

# find + sed across recursive .env files
eai "replace all occurrences of localhost:3000 with api.prod.com in every .env file recursively"
▶ find . -name ".env" -exec sed -i '' 's/localhost:3000/api.prod.com/g' {} +

# rsync with excludes, compression, and permissions
eai "sync ./dist to server 192.168.1.50:/var/www excluding node_modules and .git"
▶ rsync -avz --exclude='node_modules' --exclude='.git' ./dist/ user@192.168.1.50:/var/www/

# git log analysis
eai "show who committed the most in the last 7 days"
▶ git log --since="7 days ago" --format="%an" | sort | uniq -c | sort -rn
```

## Quick Start

### macOS / Linux

```bash
# Homebrew (recommended)
brew install feliperbroering/tap/eai

# or install script
curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash
```

### Windows

```powershell
# WinGet
winget install feliperbroering.eai

# or PowerShell script
iwr https://raw.githubusercontent.com/feliperbroering/eai/main/install.ps1 -UseBasicParsing | iex
```

### From source

```bash
cargo install --git https://github.com/feliperbroering/eai
```

### Then

```bash
eai setup                     # 30s — picks a free provider
eai "compress all PNGs here"  # go
```

## Usage

### Basic — natural language to shell

```bash
eai "kill whatever is on port 3000"
eai "find rust files bigger than 1mb modified this week"
eai "undo last 3 git commits keeping changes"
eai "convert this video to gif at 10fps"
```

### Pipe mode — feed data as context

eai auto-detects piped content type (JSON, CSV, error output, HTML, Markdown):

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

### Script mode — full scripts instead of one-liners

Outputs to stdout — pipe to a file:

```bash
eai --script "backup my database and compress it" > backup.sh
eai --script "setup a new Node.js project with TypeScript" > setup.sh
```

### Recipe mode — step-by-step workflows

```bash
eai --recipe "deploy a docker container to production"
```

```
  Step 1: Build the Docker image
    ❯ docker build -t myapp .

  Step 2: Tag for registry
    ❯ docker tag myapp registry.example.com/myapp:latest

  Step 3: Push to registry
    ❯ docker push registry.example.com/myapp:latest

  Step 4: Deploy on server
    ❯ ssh server 'docker pull registry.example.com/myapp:latest && docker run -d -p 80:3000 myapp'
```

### Tool discovery — find & install tools

When you ask for a tool that isn't installed, eai searches the web, verifies against package registries (brew, PyPI, npm, crates.io), and offers to install:

```
  🔍 Tools found for your task:

  1.  docling                                    ★ 18.2k
     Converts PDF, DOCX, HTML to Markdown with table support
     https://github.com/DS4SD/docling
     pip install docling

  2.  marker                                     ★ 19.1k
     Fast PDF to Markdown converter with OCR
     https://github.com/VikParuchuri/marker
     pip install marker-pdf

  ╭──────────────────────────────────────────────╮
  │  ● install 1-2  │  ● skip s  │  ● cancel ^C  │
  ╰──────────────────────────────────────────────╯
```

### Shell integration — Ctrl+E

Transform what you're typing directly into a shell command:

```bash
eval "$(eai init zsh)"    # add to .zshrc
eval "$(eai init bash)"   # add to .bashrc
eai init fish | source    # add to config.fish
```

Then type a description on your prompt and press **Ctrl+E** — eai replaces it with the generated command.

### Command bookmarks — save & reuse

```bash
eai save deploy "git push origin main" --desc "push to production"
eai save logs "docker logs -f --tail 100 app" --desc "tail app logs"

eai @deploy               # runs saved command directly (no LLM call)
eai @logs                 # instant — no API key needed

eai aliases               # list all bookmarks
eai unsave deploy         # remove a bookmark
```

### Shell completions

```bash
# zsh (add to .zshrc)
eval "$(eai completions zsh)"

# bash (add to .bashrc)
eval "$(eai completions bash)"

# fish (add to config.fish)
eai completions fish | source
```

### Demo mode — try without API key

```bash
eai --demo
```

Shows curated prompt→command examples to see what eai can do — no LLM call, no API key needed.

## All flags

```
eai [FLAGS] <PROMPT>

FLAGS:
  --dry              Show the command but don't run it
  --explain / --wtf  Explain a command (break down each flag)
  --script           Generate a full shell script
  --recipe           Generate a multi-step workflow
  --search           Force web search before generating
  --demo             Offline demo with sample commands
  --no-confirm       Skip confirmation (yolo mode)
  -b <BACKEND>       Force backend (gemini, groq, openai, ollama, claude-cli)
  -m <MODEL>         Override model name
  -s <SHELL>         Target shell (zsh, bash, fish, sh, powershell, cmd)
  -v, --verbose      Show system/user prompts sent to the LLM
```

## All subcommands

```
eai setup                       Interactive provider setup wizard
eai config                      Open config file in $EDITOR
eai history [--search <q>]      Show command history (fuzzy search)
eai completions <shell>         Generate shell completions (zsh/bash/fish)
eai init <shell>                Output Ctrl+E shell integration
eai save <name> <cmd> [--desc]  Bookmark a command
eai aliases                     List all bookmarks
eai unsave <name>               Remove a bookmark
```

## How it works

1. You type `eai "..."` in plain English (or any language)
2. eai detects your project context (Cargo.toml, package.json, go.mod, Dockerfile, etc.) and adapts commands accordingly
3. If the prompt was seen before, the cached result is returned instantly
4. Otherwise, eai loads tool docs — 7000+ commands from [tldr-pages](https://github.com/tldr-pages/tldr) embedded in the binary (zero latency, works offline), combined with local `--help` output
5. If the tool isn't installed, eai discovers alternatives via web search, verifies them against package registries, and offers to install
6. The LLM generates the command + a brief explanation, grounded on real documentation
7. OS-specific flag correction ensures the command works on your OS (macOS/BSD vs Linux/GNU)
8. Post-generation validation warns if the main binary isn't in your PATH
9. You confirm, edit, refine, or search before running
10. If it fails, the error goes back to the LLM — auto-retry up to 5x

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

Rate limit auto-retry: if a provider returns HTTP 429, eai waits 3 seconds and retries automatically.

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

Data locations:
- Cache: `~/.cache/eai/cache.jsonl`
- History: `~/.local/share/eai/history.jsonl`
- Bookmarks: `~/.config/eai/aliases.json`

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
| **Pipe context (stdin)** | ✓ (auto-detects JSON/CSV/error) | ✓ | ✓ | ✓ |
| **Explain mode** | ✓ (`--wtf`) | ✗ | ✗ | partial |
| **Script/recipe generation** | ✓ (`--script`, `--recipe`) | ✗ | ✗ | ✗ |
| **Command caching** | ✓ (instant repeat queries) | ✗ | ✗ | ✗ |
| **Command bookmarks** | ✓ (`save`/`@alias`) | ✗ | ✗ | ✗ |
| **Shell integration (Ctrl+E)** | ✓ (zsh/bash/fish) | ✗ | ✗ | ✗ |
| **Shell completions** | ✓ (zsh/bash/fish) | ✗ | ✗ | ✗ |
| **Project-aware** | ✓ (auto-detects Cargo/npm/Go/Docker) | ✗ | ✗ | ✗ |
| **OS-aware flags** | ✓ (GNU↔BSD auto-correction) | ✗ | ✗ | ✗ |
| **Post-generation validation** | ✓ (warns if binary missing) | ✗ | ✗ | ✗ |
| **Free by default** | ✓ (Gemini/Groq/Ollama) | ✗ (OpenAI) | ✗ (Needs API) | ✗ (OpenAI) |
| **Auto-retry on error** | ✓ (feeds stderr back) | ✗ | ✗ | ✗ |
| **Web search** | ✓ (Tavily/DDG) | ✗ (plugins) | partial | ✗ (plugins) |
| **Tool doc detection** | ✓ (7000+ embedded + --help) | ✗ | ✗ | ✗ |
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

## Acknowledgements

eai embeds documentation from [tldr-pages](https://github.com/tldr-pages/tldr), licensed under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/). Thanks to the tldr-pages team and [contributors](https://github.com/tldr-pages/tldr/graphs/contributors) for maintaining this incredible resource.

## License

[MIT](LICENSE)

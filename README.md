```
  ███████╗ █████╗ ██╗    ╭─────╮
  ██╔════╝██╔══██╗██║    │ >_  │
  █████╗  ███████║██║    ╰─────╯
  ██╔══╝  ██╔══██║██║
  ███████╗██║  ██║██║
  ╚══════╝╚═╝  ╚═╝╚═╝

  don't memorize 1000 flags. just prompt it.
```

Describe what you want in plain English. Get the shell command. Confirm. Run.

```
  ╭───────────────────────────────────────────────────────────────────╮
  │   ❯ docker ps --format "table {{.Names}}\t{{.Status}}"           │
  ╰───────────────────────────────────────────────────────────────────╯
  // lists containers showing name and status columns

  ╭──────────────────────────────────────────────────────────────────────╮
  │  ● run ↵  │  ● edit e  │  ● refine r  │  ● search s  │  ● quit ^C  │
  ╰──────────────────────────────────────────────────────────────────────╯
```

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash
```

Or `cargo install --git https://github.com/feliperbroering/eai`

First run? `eai setup` walks you through connecting a provider in 30 seconds.

## Usage

```bash
eai "compress all PNGs in this directory"
eai "kill whatever is on port 3000"
eai "find rust files bigger than 1mb modified this week"
eai "undo last 3 git commits keeping changes"
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

### Flags

```bash
eai --dry "..."          # show command, don't run
eai --explain "..."      # explain a command (alias: --wtf)
eai --search "..."       # force web search before generating
eai -b groq "..."        # force a specific backend
eai -m gpt-4o-mini "..." # force a specific model
eai --no-confirm "..."   # skip confirmation (yolo)
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
2. eai detects tools in your prompt and loads their docs (tldr/--help)
3. The LLM generates the command + a brief explanation
4. You confirm, edit, refine, or search before running
5. If it fails, the error goes back to the LLM — auto-retry up to 5x

## Providers

Run `eai setup` to connect your preferred backend:

| Provider | Model | Cost |
|----------|-------|------|
| **Groq** ★ | Llama 3.3 70B | Free (14K req/day) |
| **OpenRouter** | GPT-4o, Claude, Gemini... | 1 key, all models |
| **OpenAI** | gpt-4o-mini | Pay per use |
| **Ollama** | qwen2.5:3b (local) | Free, no API key |
| **Custom API** | GLM, Kimi, DeepSeek... | Any OpenAI-compatible |

Override per-command with `-b groq`, `-b openai`, `-b ollama`.

## Config

Lives at `~/.config/eai/config.toml`. Edit with `eai config`.

```toml
[default]
backend = "groq"
confirm = true

[groq]
api_key_env = "GROQ_API_KEY"
model = "llama-3.3-70b-versatile"

[ollama]
model = "qwen2.5:3b"
```

## vs other tools

| | eai | llm | aichat | shell-gpt |
|---|---|---|---|---|
| Pipe context | ✓ | ✗ | ✗ | ✗ |
| Explain mode | ✓ | ✗ | ✗ | ✗ |
| Free by default | ✓ | ✗ | ✗ | ✗ |
| Feedback loop | ✓ | ✗ | partial | ✗ |
| Web search | ✓ | ✗ | ✗ | ✗ |
| Tool doc detection | ✓ | ✗ | ✗ | ✗ |
| Setup wizard | ✓ | ✗ | ✗ | ✗ |
| Single binary | ✓ (Rust) | Python | Rust | Python |

## License

MIT

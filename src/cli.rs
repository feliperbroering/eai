use clap::{CommandFactory, Parser, Subcommand};

use crate::types::{BackendKind, ShellKind};

fn long_version() -> &'static str {
    concat!(
        env!("CARGO_PKG_VERSION"),
        "\nEmbedded docs: tldr-pages (CC-BY 4.0) — https://github.com/tldr-pages/tldr"
    )
}

#[derive(Debug, Parser)]
#[command(
    name = "eai",
    version,
    long_version = long_version(),
    about = "E ai? don't memorize 1000 flags — just prompt it.",
    trailing_var_arg = true
)]
pub struct Cli {
    /// AI backend to use
    #[arg(short = 'b', long, value_enum)]
    pub backend: Option<BackendKind>,

    /// Model name to use (overrides config)
    #[arg(short = 'm', long)]
    pub model: Option<String>,

    /// Target shell for generated commands
    #[arg(short = 's', long, value_enum)]
    pub shell: Option<ShellKind>,

    /// Show the command but don't run it
    #[arg(long)]
    pub dry: bool,

    /// Skip confirmation prompt (yolo mode)
    #[arg(long)]
    pub no_confirm: bool,

    /// Force web search before generating
    #[arg(long = "search")]
    pub force_search: bool,

    /// Explain a command instead of generating one (alias: --wtf)
    #[arg(long, alias = "wtf")]
    pub explain: bool,

    /// Generate a full shell script instead of a one-liner
    #[arg(long)]
    pub script: bool,

    /// Generate a multi-step recipe instead of a one-liner
    #[arg(long)]
    pub recipe: bool,

    /// Run demo with sample prompts (no API key needed)
    #[arg(long)]
    pub demo: bool,

    /// Show system/user prompts sent to the LLM
    #[arg(short = 'v', long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(value_name = "PROMPT", num_args = 1.., allow_hyphen_values = true)]
    pub prompt: Vec<String>,
}

impl Cli {
    pub fn print_help() {
        let _ = Self::command().print_help();
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Interactive setup wizard — connect your AI provider
    Setup,
    /// Open config file in $EDITOR
    Config,
    /// Show command history
    History {
        #[arg(long)]
        search: Option<String>,

        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Output shell integration (eval "$(eai init zsh)")
    Init {
        /// Shell to generate integration for
        #[arg(value_enum)]
        shell: ShellKind,
    },
    /// Save a command as a bookmark
    Save {
        /// Alias name
        name: String,
        /// Command to save
        command: String,
        /// Optional description
        #[arg(long)]
        desc: Option<String>,
    },
    /// List saved command aliases
    Aliases,
    /// Remove a saved alias
    Unsave {
        /// Alias name to remove
        name: String,
    },
    /// Clear the prompt→command cache
    ClearCache,
}

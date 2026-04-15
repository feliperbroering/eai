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
    #[arg(short = 'b', long, value_enum)]
    pub backend: Option<BackendKind>,

    #[arg(short = 'm', long)]
    pub model: Option<String>,

    #[arg(short = 's', long, value_enum)]
    pub shell: Option<ShellKind>,

    #[arg(long)]
    pub dry: bool,

    #[arg(long)]
    pub no_confirm: bool,

    #[arg(long = "search")]
    pub force_search: bool,

    #[arg(long, alias = "wtf")]
    pub explain: bool,

    #[arg(long)]
    pub script: bool,

    #[arg(long, help = "Generate a multi-step recipe instead of a one-liner")]
    pub recipe: bool,

    #[arg(long, help = "Run demo with sample prompts (no API key needed)")]
    pub demo: bool,

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
}

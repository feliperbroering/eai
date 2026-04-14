use clap::{CommandFactory, Parser, Subcommand};

use crate::types::{BackendKind, ShellKind};

#[derive(Debug, Parser)]
#[command(
    name = "eai",
    version,
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
}

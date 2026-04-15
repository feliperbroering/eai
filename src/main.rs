mod app;
mod cli;
mod config;
mod history;
mod llm;
mod search;
mod setup;
mod tldr;
mod tool_context;
mod types;
mod ui;
mod update;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    app::run(cli).await
}

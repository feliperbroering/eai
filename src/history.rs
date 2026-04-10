use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
};

use anyhow::{Context, Result};

use crate::{
    config::{ensure_parent, history_path},
    types::HistoryEntry,
};

pub fn append(entry: &HistoryEntry) -> Result<()> {
    let path = history_path()?;
    ensure_parent(&path)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open history file {}", path.display()))?;

    serde_json::to_writer(&mut file, entry)?;
    writeln!(file)?;
    Ok(())
}

pub fn load_recent(limit: usize) -> Result<Vec<HistoryEntry>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path)
        .with_context(|| format!("failed to open history file {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let entry = match serde_json::from_str::<HistoryEntry>(&line) {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        entries.push(entry);
    }

    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }

    Ok(entries)
}

pub fn search(query: Option<&str>, limit: usize) -> Result<Vec<HistoryEntry>> {
    let mut entries = load_recent(usize::MAX)?;

    if let Some(query) = query {
        let query = query.to_lowercase();
        entries.retain(|entry| {
            entry.prompt.to_lowercase().contains(&query)
                || entry.command.to_lowercase().contains(&query)
                || entry.backend.to_lowercase().contains(&query)
        });
    }

    entries.reverse();
    entries.truncate(limit);
    Ok(entries)
}

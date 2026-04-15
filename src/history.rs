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
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f64, HistoryEntry)> = entries
            .into_iter()
            .filter_map(|entry| {
                let text =
                    format!("{} {} {}", entry.prompt, entry.command, entry.backend).to_lowercase();

                let all_words_match = query_words.iter().all(|w| text.contains(w));
                if !all_words_match {
                    return None;
                }

                let mut score = 0.0;
                if entry.command.to_lowercase().contains(&query_lower) {
                    score += 10.0;
                }
                if entry.prompt.to_lowercase().contains(&query_lower) {
                    score += 5.0;
                }
                if entry.exit_code == 0 {
                    score += 2.0;
                }
                score += query_words.iter().filter(|w| text.contains(**w)).count() as f64;

                Some((score, entry))
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        entries = scored.into_iter().map(|(_, e)| e).collect();
    } else {
        entries.reverse();
    }

    entries.truncate(limit);
    Ok(entries)
}

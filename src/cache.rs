use std::collections::hash_map::DefaultHasher;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    hash: u64,
    command: String,
    explanation: Option<String>,
}

fn cache_path() -> Option<PathBuf> {
    let dir = dirs::cache_dir()?.join("eai");
    Some(dir.join("cache.jsonl"))
}

fn compute_hash(prompt: &str, os: &str, shell: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    prompt.trim().to_lowercase().hash(&mut hasher);
    os.hash(&mut hasher);
    shell.hash(&mut hasher);
    hasher.finish()
}

pub fn lookup(prompt: &str, os: &str, shell: &str) -> Option<(String, Option<String>)> {
    let path = cache_path()?;
    if !path.exists() {
        return None;
    }

    let target_hash = compute_hash(prompt, os, shell);
    let file = fs::File::open(&path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.ok()?;
        if let Ok(entry) = serde_json::from_str::<CacheEntry>(&line) {
            if entry.hash == target_hash {
                return Some((entry.command, entry.explanation));
            }
        }
    }

    None
}

pub fn clear() -> bool {
    let Some(path) = cache_path() else {
        return false;
    };
    if path.exists() {
        fs::remove_file(&path).is_ok()
    } else {
        false
    }
}

pub fn store(prompt: &str, os: &str, shell: &str, command: &str, explanation: Option<&str>) {
    let Some(path) = cache_path() else { return };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let entry = CacheEntry {
        hash: compute_hash(prompt, os, shell),
        command: command.to_string(),
        explanation: explanation.map(|s| s.to_string()),
    };

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let _ = serde_json::to_writer(&mut file, &entry);
    let _ = writeln!(file);
}

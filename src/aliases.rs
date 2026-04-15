use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct AliasStore {
    aliases: HashMap<String, AliasEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AliasEntry {
    pub command: String,
    pub description: Option<String>,
}

fn aliases_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("cannot find config dir")?
        .join("eai");
    fs::create_dir_all(&dir)?;
    Ok(dir.join("aliases.json"))
}

fn load_store() -> Result<AliasStore> {
    let path = aliases_path()?;
    if !path.exists() {
        return Ok(AliasStore::default());
    }
    let data = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&data).unwrap_or_default())
}

fn save_store(store: &AliasStore) -> Result<()> {
    let path = aliases_path()?;
    let data = serde_json::to_string_pretty(store)?;
    fs::write(&path, data)?;
    Ok(())
}

pub fn save(name: &str, command: &str, description: Option<&str>) -> Result<()> {
    let mut store = load_store()?;
    store.aliases.insert(
        name.to_string(),
        AliasEntry {
            command: command.to_string(),
            description: description.map(|s| s.to_string()),
        },
    );
    save_store(&store)?;
    Ok(())
}

pub fn get(name: &str) -> Result<Option<AliasEntry>> {
    let store = load_store()?;
    Ok(store.aliases.get(name).cloned())
}

pub fn list() -> Result<Vec<(String, AliasEntry)>> {
    let store = load_store()?;
    let mut entries: Vec<_> = store.aliases.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

pub fn remove(name: &str) -> Result<bool> {
    let mut store = load_store()?;
    let removed = store.aliases.remove(name).is_some();
    if removed {
        save_store(&store)?;
    }
    Ok(removed)
}

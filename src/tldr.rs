use std::collections::HashMap;
use std::io::Cursor;
use std::sync::LazyLock;

use bincode::Decode;

const TLDR_BLOB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tldr.bin.zst"));

#[derive(Decode)]
struct TldrEntry {
    description: String,
    examples: Vec<(String, String)>,
}

#[derive(Decode)]
struct TldrIndex {
    common: HashMap<String, TldrEntry>,
    osx: HashMap<String, TldrEntry>,
    linux: HashMap<String, TldrEntry>,
    windows: HashMap<String, TldrEntry>,
}

static INDEX: LazyLock<TldrIndex> = LazyLock::new(|| {
    let decompressed = zstd::decode_all(Cursor::new(TLDR_BLOB)).expect("valid zstd");
    let (index, _): (TldrIndex, usize) =
        bincode::decode_from_slice(&decompressed, bincode::config::standard())
            .expect("valid bincode");
    index
});

fn platform_map() -> &'static HashMap<String, TldrEntry> {
    match std::env::consts::OS {
        "macos" => &INDEX.osx,
        "linux" => &INDEX.linux,
        "windows" => &INDEX.windows,
        _ => &INDEX.common,
    }
}

/// Look up a command's tldr docs, preferring the current OS platform.
pub fn lookup(command: &str) -> Option<String> {
    let entry = platform_map()
        .get(command)
        .or_else(|| INDEX.common.get(command))?;

    let mut out = String::new();

    if !entry.description.is_empty() {
        out.push_str(&entry.description);
        out.push('\n');
    }

    for (desc, cmd) in &entry.examples {
        out.push_str(&format!("- {desc}:\n  {cmd}\n"));
    }

    Some(out)
}

#[cfg(test)]
pub fn total_commands() -> usize {
    INDEX.common.len() + INDEX.osx.len() + INDEX.linux.len() + INDEX.windows.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_known_command() {
        let result = lookup("curl");
        assert!(result.is_some(), "curl should be in tldr");
        let text = result.unwrap();
        assert!(text.contains("curl"), "should mention curl");
    }

    #[test]
    fn lookup_unknown_command() {
        assert!(lookup("zzz_nonexistent_tool_xyz").is_none());
    }

    #[test]
    fn index_has_many_commands() {
        assert!(total_commands() > 5000, "should have 5000+ commands");
    }

    #[test]
    fn platform_specific_lookup() {
        // `du` should be in at least common or osx
        let result = lookup("du");
        assert!(result.is_some(), "du should be in tldr");
    }

    #[test]
    fn common_fallback_works() {
        // `tar` is almost certainly in common
        let result = lookup("tar");
        assert!(result.is_some(), "tar should be in tldr common");
    }
}

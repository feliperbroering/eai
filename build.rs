use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::{env, fs};

use bincode::Encode;

const TLDR_ZIP_URL: &str =
    "https://github.com/tldr-pages/tldr/releases/latest/download/tldr-pages.en.zip";

const PLATFORMS: &[&str] = &["common", "osx", "linux"];

#[derive(Encode)]
struct TldrEntry {
    description: String,
    examples: Vec<(String, String)>,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").unwrap();
    let blob_path = Path::new(&out_dir).join("tldr.bin.zst");

    if blob_path.exists() {
        return;
    }

    let zip_bytes = download_tldr_zip();
    let index = parse_zip(&zip_bytes);

    let encoded = bincode::encode_to_vec(&index, bincode::config::standard()).unwrap();

    let compressed = zstd::encode_all(Cursor::new(&encoded), 19).unwrap();

    let mut f = fs::File::create(&blob_path).unwrap();
    f.write_all(&compressed).unwrap();

    eprintln!(
        "tldr: {} commands, {} KB raw, {} KB compressed",
        index.len(),
        encoded.len() / 1024,
        compressed.len() / 1024,
    );
}

fn download_tldr_zip() -> Vec<u8> {
    let cached = Path::new(&env::var("OUT_DIR").unwrap()).join("tldr-pages.en.zip");
    if cached.exists() {
        return fs::read(&cached).unwrap();
    }

    eprintln!("Downloading tldr-pages...");
    let resp = reqwest::blocking::Client::builder()
        .user_agent("eai-build")
        .build()
        .unwrap()
        .get(TLDR_ZIP_URL)
        .send()
        .unwrap();

    let bytes = resp.bytes().unwrap().to_vec();
    fs::write(&cached, &bytes).unwrap();
    bytes
}

fn parse_zip(zip_bytes: &[u8]) -> HashMap<String, TldrEntry> {
    let reader = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader).unwrap();
    let mut index: HashMap<String, TldrEntry> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();

        if !name.ends_with(".md") || name.ends_with("LICENSE.md") {
            continue;
        }

        let platform = PLATFORMS.iter().find(|p| {
            name.starts_with(&format!("{p}/"))
                || name.contains(&format!("/{p}/"))
        });
        if platform.is_none() {
            continue;
        }

        let cmd_name = Path::new(&name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if cmd_name.is_empty() {
            continue;
        }

        // common has lowest priority — platform-specific overrides
        if index.contains_key(&cmd_name) && name.contains("/common/") {
            continue;
        }

        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap_or_default();

        if let Some(entry) = parse_tldr_page(&contents) {
            index.insert(cmd_name, entry);
        }
    }

    index
}

fn parse_tldr_page(content: &str) -> Option<TldrEntry> {
    let mut description = String::new();
    let mut examples: Vec<(String, String)> = Vec::new();
    let mut current_desc: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('#') {
            continue;
        }

        if line.starts_with('>') {
            let text = line.trim_start_matches('>').trim();
            if !text.is_empty() && description.is_empty() {
                description = text.to_string();
            }
            continue;
        }

        if line.starts_with('-') {
            let text = line.trim_start_matches('-').trim().trim_end_matches(':');
            current_desc = Some(text.to_string());
            continue;
        }

        if line.starts_with('`') && line.ends_with('`') {
            let cmd = line.trim_matches('`');
            if let Some(desc) = current_desc.take() {
                examples.push((desc, cmd.to_string()));
            }
            continue;
        }
    }

    if description.is_empty() && examples.is_empty() {
        return None;
    }

    Some(TldrEntry {
        description,
        examples,
    })
}

use std::path::PathBuf;

use anyhow::Result;
use epub::doc::EpubDoc;
use qena_core::model::DocumentRecord;
use qena_core::util::{now_rfc3339, sha256_hex, slugify};
use qena_core::{Catalog, Config, Source};
use tracing::{info, warn};

use crate::{FetchOptions, FetchSummary, client, read_list, relative_path};

pub async fn fetch(
    cfg: &Config,
    source: &Source,
    options: &FetchOptions,
    catalog: &Catalog,
) -> Result<FetchSummary> {
    let mut ids: Vec<u64> = Vec::new();

    if let Some(path) = source.param_str("ids_file") {
        for item in read_list(std::path::Path::new(path))? {
            if let Ok(id) = item.parse::<u64>() {
                ids.push(id);
            }
        }
    }
    if let Some(items) = source.param("ids").and_then(|value| value.as_array()) {
        for item in items {
            let id = item
                .as_integer()
                .and_then(|value| u64::try_from(value).ok())
                .or_else(|| item.as_str().and_then(|value| value.parse().ok()));
            if let Some(id) = id {
                ids.push(id);
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();

    if let Some(limit) = options.limit {
        ids.truncate(limit);
    }

    let mut summary = FetchSummary::new(&source.id);
    summary.skipped = ids.len();
    if options.dry_run {
        for id in &ids {
            info!(source = %source.id, id = %id, "indirilecek");
        }
        return Ok(summary);
    }

    let client = client(cfg)?;
    let books_dir = cfg.books_dir("gutenberg");
    std::fs::create_dir_all(&books_dir)?;
    summary.skipped = 0;

    for id in ids {
        let Some((download_url, extension, mime, data)) = download(&client, id).await else {
            warn!(source = %source.id, id = %id, "Gutenberg kopyası bulunamadı");
            summary.failed += 1;
            continue;
        };

        let (title, author, lang, path) = match extension {
            "epub" => {
                let temp_path = books_dir.join(format!("{id}.epub"));
                std::fs::write(&temp_path, &data)?;
                let (title_opt, author, lang) = epub_metadata(&temp_path);
                let title = title_opt.unwrap_or_else(|| format!("Gutenberg #{id}"));
                let final_path = books_dir.join(format!("{id}-{}.epub", slugify(&title)));
                let _ = std::fs::rename(&temp_path, &final_path);
                let path = if final_path.exists() {
                    final_path
                } else {
                    temp_path
                };
                (title, author, lang, path)
            }
            _ => {
                let text = String::from_utf8_lossy(&data);
                let (title, author) = text_metadata(&text, id);
                let path = books_dir.join(format!("{id}-{}.txt", slugify(&title)));
                std::fs::write(&path, &data)?;
                (title, author, "en".to_string(), path)
            }
        };

        let doc = DocumentRecord {
            id: format!("gutenberg:{id}"),
            source_id: source.id.clone(),
            kind: "book".to_string(),
            title: title.clone(),
            lang,
            url: Some(format!("https://www.gutenberg.org/ebooks/{id}")),
            license: source.license.clone(),
            attribution: source.attribution.clone(),
            path: relative_path(cfg, &path),
            mime: mime.to_string(),
            hash: sha256_hex(&data),
            fetched_at: now_rfc3339(),
            revision: None,
            metadata_json: serde_json::json!({
                "authors": author,
                "download_url": download_url,
                "format": extension,
            })
            .to_string(),
        };
        catalog.upsert_document(&doc)?;
        summary.fetched += 1;
        summary.bytes += data.len() as u64;
        info!(source = %source.id, title = %title, "indirildi");

        tokio::time::sleep(std::time::Duration::from_millis(cfg.fetch.delay_ms)).await;
    }

    Ok(summary)
}

async fn download(
    client: &reqwest::Client,
    id: u64,
) -> Option<(String, &'static str, &'static str, Vec<u8>)> {
    let candidates = [
        (
            format!("https://www.gutenberg.org/ebooks/{id}.epub.images"),
            "epub",
            "application/epub+zip",
        ),
        (
            format!("https://www.gutenberg.org/ebooks/{id}.epub.noimages"),
            "epub",
            "application/epub+zip",
        ),
        (
            format!("https://www.gutenberg.org/cache/epub/{id}/pg{id}.txt"),
            "txt",
            "text/plain; charset=utf-8",
        ),
    ];

    for (url, extension, mime) in candidates {
        let response = match client.get(&url).send().await {
            Ok(response) => response,
            Err(error) => {
                warn!(id = %id, error = %error, "Gutenberg isteği başarısız");
                continue;
            }
        };
        if !response.status().is_success() {
            continue;
        }
        match response.bytes().await {
            Ok(bytes) => return Some((url, extension, mime, bytes.to_vec())),
            Err(error) => {
                warn!(id = %id, error = %error, "Gutenberg indirmesi okunamadı");
            }
        }
    }
    None
}

fn epub_metadata(path: &PathBuf) -> (Option<String>, String, String) {
    let Ok(doc) = EpubDoc::new(path) else {
        return (None, String::new(), "en".to_string());
    };
    let title = doc.get_title();
    let author = doc
        .mdata("creator")
        .map(|item| item.value.clone())
        .unwrap_or_default();
    let lang = doc
        .mdata("language")
        .map(|item| item.value.clone())
        .unwrap_or_else(|| "en".to_string());
    (title, author, lang)
}

fn text_metadata(text: &str, id: u64) -> (String, String) {
    let mut title = format!("Gutenberg #{id}");
    let mut author = String::new();
    for line in text.lines().take(120) {
        if let Some(value) = line.strip_prefix("Title:") {
            let value = value.trim();
            if !value.is_empty() {
                title = value.to_string();
            }
        } else if let Some(value) = line.strip_prefix("Author:") {
            let value = value.trim();
            if !value.is_empty() {
                author = value.to_string();
            }
        }
    }
    (title, author)
}

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use qena_core::model::DocumentRecord;
use qena_core::util::{now_rfc3339, sha256_hex, slugify};
use qena_core::{Catalog, Config, Source};
use tracing::info;

use crate::{FetchOptions, FetchSummary, relative_path};

pub async fn fetch(
    cfg: &Config,
    source: &Source,
    options: &FetchOptions,
    catalog: &Catalog,
) -> Result<FetchSummary> {
    let dir = PathBuf::from(source.param_str("dir").unwrap_or("guides"));
    let mut files = markdown_files(&dir)?;
    files.sort();

    if let Some(limit) = options.limit {
        files.truncate(limit);
    }

    let mut summary = FetchSummary::new(&source.id);
    summary.skipped = files.len();
    if options.dry_run {
        for file in &files {
            info!(source = %source.id, file = %file.display(), "kopyalanacak");
        }
        return Ok(summary);
    }

    let target_dir = cfg.guides_dir();
    std::fs::create_dir_all(&target_dir)?;
    summary.skipped = 0;

    for file in files {
        let text = std::fs::read_to_string(&file)
            .with_context(|| format!("{} okunamadı", file.display()))?;
        let stem = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("rehber");
        let slug = slugify(stem);
        let title = text
            .lines()
            .find_map(|line| line.strip_prefix("# ").map(str::trim))
            .filter(|title| !title.is_empty())
            .unwrap_or(stem)
            .to_string();

        let target = target_dir.join(format!("{slug}.md"));
        std::fs::write(&target, text.as_bytes())?;

        let doc = DocumentRecord {
            id: format!("guide:{slug}"),
            source_id: source.id.clone(),
            kind: "guide".to_string(),
            title,
            lang: source.param_str("lang").unwrap_or("tr").to_string(),
            url: None,
            license: source.license.clone(),
            attribution: source.attribution.clone(),
            path: relative_path(cfg, &target),
            mime: "text/markdown".to_string(),
            hash: sha256_hex(text.as_bytes()),
            fetched_at: now_rfc3339(),
            revision: None,
            metadata_json: serde_json::json!({ "original": file.file_name() }).to_string(),
        };
        catalog.upsert_document(&doc)?;
        summary.fetched += 1;
        summary.bytes += text.len() as u64;
        info!(source = %source.id, title = %doc.title, "kopyalandı");
    }

    Ok(summary)
}

fn markdown_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("{} okunamadı", dir.display()));
        }
    };
    Ok(entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect())
}

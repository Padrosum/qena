//! Ağa çıkan tek Qena bileşeni. Her indirici açık bir `qena fetch` çağrısıyla
//! çalışır; dizinleyici ve web sunucusu bu crate'i kullanmaz.

mod guides;
mod gutenberg;
mod protomaps;
mod web;
mod wikipedia;

use std::path::Path;

use anyhow::{Context, Result, bail};
use qena_core::{Catalog, Config, Source};
use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    pub limit: Option<usize>,
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FetchSummary {
    pub source: String,
    pub fetched: usize,
    pub skipped: usize,
    pub failed: usize,
    pub bytes: u64,
}

impl FetchSummary {
    fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            ..Self::default()
        }
    }
}

pub use web::save_url;

pub async fn fetch_source(
    cfg: &Config,
    source: &Source,
    options: &FetchOptions,
) -> Result<FetchSummary> {
    cfg.ensure_dirs()?;
    let catalog = Catalog::open(&cfg.catalog_path())?;
    if !options.dry_run {
        catalog.upsert_source(source)?;
    }

    let summary = match source.kind.as_str() {
        "wikipedia_api" => wikipedia::fetch(cfg, source, options, &catalog).await,
        "gutenberg" => gutenberg::fetch(cfg, source, options, &catalog).await,
        "guide_pack" => guides::fetch(cfg, source, options, &catalog).await,
        "wikisource_api" => bail!("wikisource_api henüz uygulanmadı (M3)"),
        "standard_ebooks" => bail!("standard_ebooks henüz uygulanmadı (M3)"),
        "protomaps_region" => protomaps::fetch(cfg, source, options, &catalog).await,
        "natural_earth" => bail!("natural_earth henüz uygulanmadı (M2)"),
        "url_list" => web::fetch(cfg, source, options, &catalog).await,
        "web_save" => web::fetch(cfg, source, options, &catalog).await,
        "zim" => bail!("zim henüz uygulanmadı (M3)"),
        other => bail!("bilinmeyen kaynak türü: {other}"),
    }?;

    if !options.dry_run {
        info!(
            source = %source.id,
            fetched = summary.fetched,
            failed = summary.failed,
            bytes = summary.bytes,
            "indirme bitti"
        );
    }
    Ok(summary)
}

pub(crate) fn client(cfg: &Config) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(&cfg.general.user_agent)
        .timeout(std::time::Duration::from_secs(cfg.fetch.timeout_secs))
        .build()
        .context("HTTP istemcisi kurulamadı")
}

pub(crate) fn read_list(path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("liste okunamadı: {}", path.display()))?;
    let mut items = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let item = line.split('#').next().unwrap_or(line).trim();
        if !item.is_empty() && !items.iter().any(|existing| existing == item) {
            items.push(item.to_string());
        }
    }
    Ok(items)
}

pub(crate) fn relative_path(cfg: &Config, path: &Path) -> String {
    path.strip_prefix(&cfg.general.data_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

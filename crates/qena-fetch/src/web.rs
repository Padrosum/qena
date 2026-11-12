use std::path::Path;

use anyhow::{Context, Result, bail};
use qena_core::model::DocumentRecord;
use qena_core::util::{now_rfc3339, sha256_hex, slugify};
use qena_core::{Catalog, Config, Source};
use tracing::info;
use url::Url;

use crate::{FetchOptions, FetchSummary, read_list, relative_path};

pub async fn save_url(
    cfg: &Config,
    catalog: &Catalog,
    url: &str,
    source_id: &str,
    license: &str,
    attribution: &str,
) -> Result<DocumentRecord> {
    let parsed = Url::parse(url).with_context(|| format!("geçersiz URL: {url}"))?;
    if !(parsed.scheme() == "http" || parsed.scheme() == "https") {
        bail!("yalnızca http/https URL'leri kaydedilir");
    }
    let host = parsed
        .host_str()
        .unwrap_or("web")
        .replace([':', '/', '\\'], "-");
    let url_owned = url.to_string();
    let options = cfg.clone();

    let (data, title) =
        tokio::task::spawn_blocking(move || -> Result<(Vec<u8>, Option<String>)> {
            let monolith_options = monolith::core::MonolithOptions {
                output_format: monolith::core::MonolithOutputFormat::HTML,
                user_agent: Some(options.general.user_agent.clone()),
                timeout: options.fetch.timeout_secs,
                silent: true,
                ..Default::default()
            };
            let session = monolith::session::Session::new(None, None, monolith_options);
            monolith::core::create_monolithic_document(session, url_owned)
                .map_err(|error| anyhow::anyhow!("monolith: {error}"))
        })
        .await??;

    let now = chrono::Utc::now();
    let stamp = now.format("%Y%m%d-%H%M%S").to_string();
    let nonce = now.timestamp_millis();
    let title = title.unwrap_or_else(|| url.to_string());
    let directory = cfg.web_dir().join(&host);
    std::fs::create_dir_all(&directory)?;
    let file_path = directory.join(format!("{stamp}-{nonce}-{}.html", slugify(&title)));
    std::fs::write(&file_path, &data)?;

    let doc = DocumentRecord {
        id: format!("web:{}:{nonce}", &sha256_hex(url.as_bytes())[..12]),
        source_id: source_id.to_string(),
        kind: "web".to_string(),
        title,
        lang: "und".to_string(),
        url: Some(url.to_string()),
        license: license.to_string(),
        attribution: attribution.to_string(),
        path: relative_path(cfg, &file_path),
        mime: "text/html".to_string(),
        hash: sha256_hex(&data),
        fetched_at: now_rfc3339(),
        revision: None,
        metadata_json: serde_json::json!({
            "host": host,
            "bytes": data.len(),
            "capture": "monolith-html",
        })
        .to_string(),
    };
    catalog.upsert_document(&doc)?;
    info!(url = %url, path = %file_path.display(), bytes = data.len(), "web kaydı alındı");
    Ok(doc)
}

pub async fn fetch(
    cfg: &Config,
    source: &Source,
    options: &FetchOptions,
    catalog: &Catalog,
) -> Result<FetchSummary> {
    let mut urls = source.param_array("urls");
    if let Some(path) = source.param_str("urls_file") {
        urls.extend(read_list(Path::new(path))?);
    }
    urls.sort();
    urls.dedup();
    if let Some(limit) = options.limit {
        urls.truncate(limit);
    }
    if urls.is_empty() {
        bail!("{}: urls veya urls_file gerekli", source.id);
    }

    let mut summary = FetchSummary::new(&source.id);
    summary.skipped = urls.len();
    if options.dry_run {
        for url in &urls {
            info!(source = %source.id, url = %url, "kaydedilecek");
        }
        return Ok(summary);
    }
    summary.skipped = 0;

    for url in urls {
        match save_url(
            cfg,
            catalog,
            &url,
            &source.id,
            &source.license,
            &source.attribution,
        )
        .await
        {
            Ok(doc) => {
                summary.fetched += 1;
                summary.bytes += doc
                    .metadata()
                    .get("bytes")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
            }
            Err(error) => {
                summary.failed += 1;
                info!(source = %source.id, url = %url, error = %error, "kayıt başarısız");
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(cfg.fetch.delay_ms)).await;
    }
    Ok(summary)
}

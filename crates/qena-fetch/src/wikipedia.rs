use anyhow::Result;
use qena_core::model::DocumentRecord;
use qena_core::util::{now_rfc3339, sha256_hex, slugify};
use qena_core::{Catalog, Config, Source};
use serde::Deserialize;
use tracing::{info, warn};

use crate::{FetchOptions, FetchSummary, client, read_list, relative_path};

#[derive(Debug, Deserialize)]
struct ApiResponse {
    parse: Option<ParseResponse>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    info: String,
}

#[derive(Debug, Deserialize)]
struct ParseResponse {
    title: String,
    pageid: i64,
    revid: i64,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    displaytitle: Option<String>,
}

pub async fn fetch(
    cfg: &Config,
    source: &Source,
    options: &FetchOptions,
    catalog: &Catalog,
) -> Result<FetchSummary> {
    let lang = source.param_str("lang").unwrap_or("tr");
    let mut titles = Vec::new();

    if let Some(path) = source.param_str("titles_file") {
        titles.extend(read_list(std::path::Path::new(path))?);
    }
    for title in source.param_array("titles") {
        if !titles.iter().any(|existing| existing == &title) {
            titles.push(title);
        }
    }

    if let Some(limit) = options.limit {
        titles.truncate(limit);
    }

    let mut summary = FetchSummary::new(&source.id);
    summary.skipped = titles.len();

    if options.dry_run {
        for title in &titles {
            info!(source = %source.id, title = %title, "indirilecek");
        }
        return Ok(summary);
    }

    let client = client(cfg)?;
    let api = format!("https://{lang}.wikipedia.org/w/api.php");
    let wiki_dir = cfg.wiki_dir(lang);
    std::fs::create_dir_all(&wiki_dir)?;
    summary.skipped = 0;
    let delay_ms = source
        .param_usize("delay_ms")
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(1000)
        .max(250);

    for title in titles {
        let mut response = None;
        for attempt in 0..4u64 {
            match client
                .get(&api)
                .query(&[
                    ("action", "parse"),
                    ("page", title.as_str()),
                    ("prop", "text|revid|displaytitle"),
                    ("format", "json"),
                    ("formatversion", "2"),
                    ("redirects", "1"),
                ])
                .send()
                .await
            {
                Ok(result) if result.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    let wait = result
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.parse::<u64>().ok())
                        .unwrap_or(5);
                    warn!(
                        source = %source.id,
                        title = %title,
                        wait_secs = wait,
                        attempt = attempt + 1,
                        "Vikipedi hız sınırı; bekleniyor"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(wait + attempt * 2)).await;
                }
                Ok(result) => {
                    response = Some(result);
                    break;
                }
                Err(error) => {
                    warn!(source = %source.id, title = %title, error = %error, "istek başarısız");
                    break;
                }
            }
        }

        let Some(response) = response else {
            summary.failed += 1;
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            continue;
        };

        let status = response.status();
        let body = response.bytes().await?;
        if !status.is_success() {
            warn!(source = %source.id, title = %title, status = %status, "HTTP hatası");
            summary.failed += 1;
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            continue;
        }

        let parsed: ApiResponse = match serde_json::from_slice(&body) {
            Ok(parsed) => parsed,
            Err(error) => {
                warn!(source = %source.id, title = %title, error = %error, "JSON çözülemedi");
                summary.failed += 1;
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                continue;
            }
        };

        if let Some(error) = parsed.error {
            warn!(source = %source.id, title = %title, code = %error.code, info = %error.info, "Vikipedi hatası");
            summary.failed += 1;
            tokio::time::sleep(std::time::Duration::from_millis(cfg.fetch.delay_ms)).await;
            continue;
        }

        let Some(parse) = parsed.parse else {
            warn!(source = %source.id, title = %title, "sayfa bulunamadı");
            summary.failed += 1;
            tokio::time::sleep(std::time::Duration::from_millis(cfg.fetch.delay_ms)).await;
            continue;
        };

        let Some(html) = parse.text else {
            warn!(source = %source.id, title = %title, "sayfa metni boş");
            summary.failed += 1;
            tokio::time::sleep(std::time::Duration::from_millis(cfg.fetch.delay_ms)).await;
            continue;
        };

        let file_name = format!("{}-{}.html", parse.pageid, slugify(&parse.title));
        let file_path = wiki_dir.join(&file_name);
        std::fs::write(&file_path, html.as_bytes())?;

        let doc = DocumentRecord {
            id: format!("wiki:{lang}:{}", parse.pageid),
            source_id: source.id.clone(),
            kind: "wiki".to_string(),
            title: parse.title.clone(),
            lang: lang.to_string(),
            url: Some(format!(
                "https://{lang}.wikipedia.org/?curid={}",
                parse.pageid
            )),
            license: source.license.clone(),
            attribution: source.attribution.clone(),
            path: relative_path(cfg, &file_path),
            mime: "text/html".to_string(),
            hash: sha256_hex(html.as_bytes()),
            fetched_at: now_rfc3339(),
            revision: Some(parse.revid.to_string()),
            metadata_json: serde_json::json!({
                "pageid": parse.pageid,
                "revid": parse.revid,
                "displaytitle": parse.displaytitle,
                "source_title": title,
            })
            .to_string(),
        };
        catalog.upsert_document(&doc)?;
        summary.fetched += 1;
        summary.bytes += html.len() as u64;
        info!(source = %source.id, title = %parse.title, "indirildi");

        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
    }

    Ok(summary)
}

use anyhow::{Context, Result, bail};
use qena_core::model::DocumentRecord;
use qena_core::util::{now_rfc3339, sha256_hex};
use qena_core::{Catalog, Config, Source};
use serde_json::Value;
use tracing::info;

use crate::{FetchOptions, FetchSummary, client, relative_path};

pub async fn fetch(
    cfg: &Config,
    source: &Source,
    options: &FetchOptions,
    catalog: &Catalog,
) -> Result<FetchSummary> {
    let region = source.param_str("region").unwrap_or("turkey");
    let bbox = source.param_str("bbox").unwrap_or("25.5,35.8,44.8,42.2");
    let maxzoom = source.param_usize("maxzoom").unwrap_or(10);
    let build = source.param_str("build").unwrap_or("latest");
    let target = cfg
        .general
        .data_dir
        .join("maps")
        .join(format!("{region}.pmtiles"));
    std::fs::create_dir_all(target.parent().unwrap_or(&cfg.general.data_dir))?;

    let mut summary = FetchSummary::new(&source.id);
    if target.exists() && !options.force {
        summary.skipped = 1;
        info!(source = %source.id, path = %target.display(), "harita zaten var; --force ile yenile");
        return Ok(summary);
    }

    let build_key = if build == "latest" {
        let client = client(cfg)?;
        let builds: Vec<Value> = client
            .get("https://build-metadata.protomaps.dev/builds.json")
            .send()
            .await
            .context("Protomaps derleme listesi alınamadı")?
            .json()
            .await
            .context("Protomaps derleme listesi çözülemedi")?;
        builds
            .last()
            .and_then(|value| value.get("key"))
            .and_then(|value| value.as_str())
            .context("Protomaps derleme anahtarı bulunamadı")?
            .to_string()
    } else if build.ends_with(".pmtiles") {
        build.to_string()
    } else {
        format!("{build}.pmtiles")
    };

    let source_url = format!("https://build.protomaps.com/{build_key}");
    let maxzoom_arg = maxzoom.to_string();

    if options.dry_run {
        info!(
            source = %source.id,
            region = %region,
            bbox = %bbox,
            maxzoom = maxzoom,
            url = %source_url,
            "indirilecek"
        );
        summary.skipped = 1;
        return Ok(summary);
    }

    let status = tokio::process::Command::new("pmtiles")
        .args([
            "extract",
            &source_url,
            target.to_str().context("hedef yol UTF-8 değil")?,
            "--bbox",
            bbox,
            "--maxzoom",
            &maxzoom_arg,
            "--download-threads",
            "4",
        ])
        .status()
        .await
        .context("pmtiles bulunamadı; ~/.local/bin/pmtiles kurulu mu?")?;

    if !status.success() {
        let _ = std::fs::remove_file(&target);
        bail!("pmtiles extract başarısız: {status}");
    }

    let data = std::fs::read(&target)?;
    let doc = DocumentRecord {
        id: format!("map:{region}"),
        source_id: source.id.clone(),
        kind: "map".to_string(),
        title: source.title.clone(),
        lang: "tr".to_string(),
        url: Some(source_url),
        license: source.license.clone(),
        attribution: source.attribution.clone(),
        path: relative_path(cfg, &target),
        mime: "application/vnd.pmtiles".to_string(),
        hash: sha256_hex(&data),
        fetched_at: now_rfc3339(),
        revision: Some(build_key),
        metadata_json: serde_json::json!({
            "region": region,
            "bbox": bbox,
            "maxzoom": maxzoom,
        })
        .to_string(),
    };
    catalog.upsert_document(&doc)?;
    summary.fetched = 1;
    summary.bytes = data.len() as u64;
    info!(source = %source.id, path = %target.display(), bytes = data.len(), "harita indirildi");
    Ok(summary)
}

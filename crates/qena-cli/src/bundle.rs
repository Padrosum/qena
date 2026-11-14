use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use qena_core::Config;
use qena_core::util::{now_rfc3339, sha256_hex};
use serde_json::{Value, json};
use walkdir::WalkDir;

const MANIFEST: &str = "MANIFEST.json";

pub fn export(cfg: &Config, output: &Path, force: bool, with_binary: bool) -> Result<()> {
    if output.exists() && !force {
        bail!(
            "{} zaten var; üzerine yazmak için --force kullan",
            output.display()
        );
    }
    let root = std::env::current_dir().context("çalışma klasörü okunamadı")?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let staging = std::env::temp_dir().join(format!("qena-export-{}-{nonce}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let mut copied_dirs = 0usize;
    for name in [
        "sources",
        "packages",
        "lists",
        "guides",
        "assets",
        "docs",
        "packaging",
        "scripts",
    ] {
        let source = root.join(name);
        if source.exists() {
            copy_dir(&source, &staging.join(name))?;
            copied_dirs += 1;
        }
    }
    for name in ["README.md", "CHANGELOG.md", "LICENSE"] {
        let source = root.join(name);
        if source.is_file() {
            fs::copy(&source, staging.join(name))?;
        }
    }
    if cfg.general.data_dir.exists() {
        copy_dir(&cfg.general.data_dir, &staging.join("data"))?;
        copied_dirs += 1;
    } else {
        bail!("veri dizini yok: {}", cfg.general.data_dir.display());
    }

    let mut portable = cfg.clone();
    portable.general.data_dir = PathBuf::from("data");
    portable.general.assets_dir = PathBuf::from("assets");
    portable.sources.dir = PathBuf::from("sources");
    portable.packages.dir = PathBuf::from("packages");
    fs::write(
        staging.join("qena.toml"),
        toml::to_string_pretty(&portable).context("taşınabilir qena.toml üretilemedi")?,
    )?;

    if with_binary {
        let exe = std::env::current_exe().context("qena ikilisi bulunamadı")?;
        fs::copy(&exe, staging.join("qena")).context("qena ikilisi pakete eklenemedi")?;
    }

    let files = collect_files(&staging)?;
    let total_bytes: u64 = files
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .sum();
    let manifest = json!({
        "format": "qena-bundle-v1",
        "created_at": now_rfc3339(),
        "qena_version": env!("CARGO_PKG_VERSION"),
        "with_binary": with_binary,
        "files": files
            .iter()
            .map(|path| {
                let relative = path.strip_prefix(&staging).unwrap_or(path);
                let hash = fs::read(path).map(|bytes| sha256_hex(&bytes)).unwrap_or_default();
                (
                    relative.to_string_lossy().replace('\\', "/"),
                    Value::String(hash),
                )
            })
            .collect::<serde_json::Map<String, Value>>(),
    });
    fs::write(
        staging.join(MANIFEST),
        serde_json::to_string_pretty(&manifest)?,
    )?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(output).with_context(|| format!("{} yazılamadı", output.display()))?;
    let encoder = zstd::Encoder::new(file, 10).context("zstd başlatılamadı")?;
    let mut tar = tar::Builder::new(encoder);
    tar.append_dir_all(".", &staging)
        .context("arşiv oluşturulamadı")?;
    tar.finish()?;
    let encoder = tar.into_inner()?;
    encoder.finish()?;

    let bundle_bytes = fs::metadata(output)?.len();
    println!(
        "Paket: {} · {} dosya · kaynak {} bayt · paket {} bayt · {} dizin",
        output.display(),
        files.len() + 1,
        total_bytes,
        bundle_bytes,
        copied_dirs
    );
    if !with_binary {
        println!("Not: ikili pakete dahil değil; --with-binary ile ekleyebilirsin.");
    }
    fs::remove_dir_all(&staging).ok();
    Ok(())
}

pub fn import(bundle: &Path, into: &Path, force: bool) -> Result<()> {
    if !bundle.exists() {
        bail!("paket yok: {}", bundle.display());
    }
    if into.exists() {
        let empty = fs::read_dir(into)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !empty && !force {
            bail!(
                "{} boş değil; üzerine açmak için --force kullan",
                into.display()
            );
        }
    }
    fs::create_dir_all(into)?;
    let into = into.canonicalize()?;

    let file = File::open(bundle)?;
    let decoder = zstd::Decoder::new(file).context("zstd paketi açılamadı")?;
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        if path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            bail!("pakette güvensiz yol: {}", path.display());
        }
        let target = into.join(&path);
        if !target.starts_with(&into) {
            bail!("paket dışına çıkılamaz: {}", path.display());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&target)?;
    }

    let manifest_path = into.join(MANIFEST);
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .with_context(|| format!("{} okunamadı", manifest_path.display()))?,
    )
    .context("MANIFEST.json çözülemedi")?;
    if manifest["format"] != "qena-bundle-v1" {
        bail!("bilinmeyen paket biçimi: {}", manifest["format"]);
    }
    let mut checked = 0usize;
    let mut broken = Vec::new();
    if let Some(files) = manifest["files"].as_object() {
        for (relative, expected) in files {
            let path = into.join(relative);
            let actual = fs::read(&path)
                .map(|bytes| sha256_hex(&bytes))
                .unwrap_or_default();
            if Some(actual.as_str()) != expected.as_str() {
                broken.push(relative.clone());
            } else {
                checked += 1;
            }
        }
    }
    if !broken.is_empty() {
        bail!("bozuk dosyalar: {}", broken.join(", "));
    }

    println!(
        "Paket açıldı: {} · {} dosya doğrulandı · qena {}",
        into.display(),
        checked,
        manifest["qena_version"].as_str().unwrap_or("?")
    );
    println!(
        "Çalıştır: qena --config {} serve",
        into.join("qena.toml").display()
    );
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    for entry in WalkDir::new(from).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        let relative = path.strip_prefix(from)?;
        let target = to.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target).with_context(|| format!("{} kopyalanamadı", path.display()))?;
        }
    }
    Ok(())
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .collect();
    files.sort();
    Ok(files)
}

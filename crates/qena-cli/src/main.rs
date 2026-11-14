use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use qena_core::util::now_rfc3339;
use qena_core::{Catalog, Config, Package, Source};
use qena_fetch::{FetchOptions, fetch_source};
use tracing_subscriber::EnvFilter;

mod bundle;

#[derive(Debug, Parser)]
#[command(name = "qena", version, about = "Kötü günler için yerel bilgi sandığı")]
struct Cli {
    /// Qena yapılandırma dosyası
    #[arg(long, global = true, default_value = "qena.toml")]
    config: PathBuf,

    /// Veri dizinini geçici olarak değiştir
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Veri dizinini ve kataloğu hazırla
    Init,
    /// Kaynak bildirimlerini yönet
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    /// Hazır veri paketlerini listele/göster/indir
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    /// Açıkça seçilen kaynağı/kategoriyi indir
    Fetch {
        /// Kaynak kimliği (ör. wikipedia-tr-curated)
        source: Option<String>,
        /// Kategori adı (ör. wikipedia, books, maps, guides)
        #[arg(long)]
        category: Option<String>,
        /// Kaynak başına en fazla öğe
        #[arg(long)]
        limit: Option<usize>,
        /// Yalnızca planı göster, indirme
        #[arg(long)]
        dry_run: bool,
        /// Var olan dosyayı yenile
        #[arg(long)]
        force: bool,
        /// Web kaydı kuyruğunu işle
        #[arg(long)]
        queue: bool,
    },
    /// Yerel Tantivy dizinini oluştur/güncelle
    Index {
        /// Dizini sıfırdan kur
        #[arg(long)]
        rebuild: bool,
    },
    /// Dizinde ara
    Search {
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        lang: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Yerel web arayüzünü başlat
    Serve {
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        host: Option<String>,
    },
    /// Bir URL'yi web kaydı kuyruğuna ekle
    Save { url: String },
    /// Arşivi tek pakete dönüştür
    Export {
        /// Çıktı .tar.zst yolu
        output: PathBuf,
        /// Var olan paketin üzerine yaz
        #[arg(long)]
        force: bool,
        /// Qena ikilisini de pakete ekle
        #[arg(long)]
        with_binary: bool,
    },
    /// Paketi yeni bir dizine aç ve doğrula
    Import {
        /// Paket yolu
        archive: PathBuf,
        /// Açılacak dizin
        #[arg(long, default_value = ".")]
        into: PathBuf,
        /// Var olan dosyaların üzerine yaz
        #[arg(long)]
        force: bool,
    },
    /// Kurulumu ve dizini denetle
    Doctor,
    /// Belge, dizin ve kuyruk durumu
    Status,
}

#[derive(Debug, Subcommand)]
enum SourceCommand {
    /// Kaynakları listele
    List,
}

#[derive(Debug, Subcommand)]
enum PackageCommand {
    /// Hazır paketleri listele
    List {
        /// JSON çıktısı
        #[arg(long)]
        json: bool,
        /// Açıklama, lisans ve kaynak ayrıntılarıyla göster
        #[arg(long)]
        long: bool,
        /// Yalnızca bu kategorideki paketler
        #[arg(long)]
        category: Option<String>,
        /// Yalnızca tüm kaynakları indirilmiş paketler
        #[arg(long)]
        installed: bool,
    },
    /// Paket ayrıntılarını göster
    Show { id: String },
    /// Paketteki tüm kaynakları indir
    Fetch {
        id: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let mut cfg = Config::load(Some(&cli.config))?;
    if let Some(data_dir) = cli.data_dir {
        cfg.general.data_dir = if data_dir.is_absolute() {
            data_dir
        } else {
            std::env::current_dir()?.join(data_dir)
        };
    }

    match cli.command {
        Command::Init => init(&cli.config, &cfg),
        Command::Source { command } => match command {
            SourceCommand::List => list_sources(&cfg),
        },
        Command::Package { command } => match command {
            PackageCommand::List {
                json,
                long,
                category,
                installed,
            } => list_packages(&cfg, json, long, category.as_deref(), installed),
            PackageCommand::Show { id } => show_package(&cfg, &id),
            PackageCommand::Fetch { id, dry_run, force } => {
                fetch_package(&cfg, &id, dry_run, force).await
            }
        },
        Command::Fetch {
            source,
            category,
            limit,
            dry_run,
            force,
            queue,
        } => {
            if queue {
                process_queue(&cfg, limit).await
            } else {
                fetch(&cfg, source, category, limit, dry_run, force).await
            }
        }
        Command::Index { rebuild } => index(&cfg, rebuild),
        Command::Search {
            query,
            kind,
            source,
            lang,
            limit,
        } => search(&cfg, &query.join(" "), kind, source, lang, limit),
        Command::Serve { port, host } => {
            let host = host.unwrap_or_else(|| cfg.general.host.clone());
            let port = port.unwrap_or(cfg.general.port);
            qena_web::serve(cfg, &host, port).await
        }
        Command::Save { url } => save(&cfg, &url).await,
        Command::Export {
            output,
            force,
            with_binary,
        } => bundle::export(&cfg, &output, force, with_binary),
        Command::Import {
            archive,
            into,
            force,
        } => bundle::import(&archive, &into, force),
        Command::Doctor => doctor(&cfg),
        Command::Status => status(&cfg),
    }
}

fn init(config_path: &std::path::Path, cfg: &Config) -> Result<()> {
    let written = Config::write_default(config_path)?;
    cfg.ensure_dirs()?;
    let catalog = Catalog::open(&cfg.catalog_path())?;
    catalog.upsert_source(&Source {
        id: "qena".to_string(),
        category: "system".to_string(),
        title: "Qena çekirdeği".to_string(),
        kind: "system".to_string(),
        license: "GPL-3.0-or-later".to_string(),
        attribution: "Qena".to_string(),
        enabled: true,
        params: toml::Value::Table(toml::map::Map::new()),
    })?;
    println!(
        "{}",
        if written {
            "qena.toml yazıldı."
        } else {
            "qena.toml zaten var; dokunulmadı."
        }
    );
    println!("Veri dizini: {}", cfg.general.data_dir.display());
    println!("Katalog: {}", cfg.catalog_path().display());
    println!("Sonraki: qena source list");
    Ok(())
}

fn list_sources(cfg: &Config) -> Result<()> {
    let sources = Source::load_all(&cfg.sources.dir)?;
    if sources.is_empty() {
        println!("Kaynak yok: {}", cfg.sources.dir.display());
        return Ok(());
    }
    println!("{:<24} {:<12} {:<10} BAŞLIK", "KİMLİK", "KATEGORİ", "TÜR");
    for source in sources {
        println!(
            "{:<24} {:<12} {:<10} {}{}",
            source.id,
            source.category,
            source.kind,
            source.title,
            if source.enabled { " *" } else { "" }
        );
    }
    Ok(())
}

fn list_packages(
    cfg: &Config,
    json: bool,
    long: bool,
    category: Option<&str>,
    installed_only: bool,
) -> Result<()> {
    let mut packages = Package::load_all(&cfg.packages.dir)?;
    if let Some(category) = category {
        packages.retain(|package| package.category == category);
    }
    let catalog = Catalog::open(&cfg.catalog_path())?;
    let counts: HashMap<String, i64> = catalog.source_counts()?.into_iter().collect();
    if installed_only {
        packages.retain(|package| package_status(package, &counts) == "hazır");
    }
    if packages.is_empty() {
        println!("Paket yok: {}", cfg.packages.dir.display());
        return Ok(());
    }

    if json {
        let views: Vec<PackageView<'_>> = packages
            .iter()
            .map(|package| PackageView {
                package,
                status: package_status(package, &counts),
                source_count: package.sources.len(),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&views)?);
        return Ok(());
    }

    if long {
        for package in &packages {
            println!("{}  [{}]", package.title, package.id);
            println!(
                "  Kategori: {} · Boyut: {} · Kaynak: {} · Durum: {}",
                package.category,
                package.estimated_size,
                package.sources.len(),
                package_status(package, &counts)
            );
            println!("  Açıklama: {}", package.description);
            println!("  Lisans: {}", package.license);
            if !package.tags.is_empty() {
                println!("  Etiketler: {}", package.tags.join(", "));
            }
            println!("  Kaynaklar:");
            for source_id in &package.sources {
                let limit = package
                    .limit_for(source_id)
                    .map(|limit| format!(" · limit {limit}"))
                    .unwrap_or_default();
                println!("    - {source_id}{limit}");
            }
            println!();
        }
        return Ok(());
    }

    println!(
        "{:<20} {:<12} {:<10} {:<7} {:<6} BAŞLIK",
        "KİMLİK", "KATEGORİ", "BOYUT", "KAYNAK", "DURUM"
    );
    for package in &packages {
        println!(
            "{:<20} {:<12} {:<10} {:<7} {:<6} {}",
            package.id,
            package.category,
            package.estimated_size,
            package.sources.len(),
            package_status(package, &counts),
            package.title
        );
    }
    println!("\nAyrıntı: qena package show <kimlik> · Uzun liste: qena package list --long");
    Ok(())
}

#[derive(serde::Serialize)]
struct PackageView<'a> {
    #[serde(flatten)]
    package: &'a Package,
    status: &'static str,
    source_count: usize,
}

fn package_status(package: &Package, counts: &HashMap<String, i64>) -> &'static str {
    let present = package
        .sources
        .iter()
        .filter(|source| counts.get(*source).copied().unwrap_or(0) > 0)
        .count();
    if present == package.sources.len() {
        "hazır"
    } else if present == 0 {
        "yok"
    } else {
        "kısmi"
    }
}

fn show_package(cfg: &Config, id: &str) -> Result<()> {
    let package = Package::find(&cfg.packages.dir, id)?;
    let catalog = Catalog::open(&cfg.catalog_path())?;
    let counts: HashMap<String, i64> = catalog.source_counts()?.into_iter().collect();
    println!("{}", package.title);
    println!("Kimlik: {}", package.id);
    println!("Kategori: {}", package.category);
    println!("Boyut: {}", package.estimated_size);
    println!("Durum: {}", package_status(&package, &counts));
    println!("Lisans: {}", package.license);
    println!("Açıklama: {}", package.description);
    if !package.tags.is_empty() {
        println!("Etiketler: {}", package.tags.join(", "));
    }
    println!("Kaynaklar:");
    for source_id in &package.sources {
        let source = Source::find(&cfg.sources.dir, source_id);
        let limit = package
            .limit_for(source_id)
            .map(|limit| format!(" · limit {limit}"))
            .unwrap_or_default();
        match source {
            Ok(source) => println!("  - {source_id} · {}{limit}", source.title),
            Err(_) => println!("  - {source_id} · (kaynak tanımı yok){limit}"),
        }
    }
    println!("\nİndir: qena package fetch {}", package.id);
    Ok(())
}

async fn fetch_package(cfg: &Config, id: &str, dry_run: bool, force: bool) -> Result<()> {
    let package = Package::find(&cfg.packages.dir, id)?;
    println!("Paket: {} ({})", package.title, package.id);
    let mut fetched = 0usize;
    let mut failed = 0usize;
    let mut bytes = 0u64;

    for source_id in &package.sources {
        let source = Source::find(&cfg.sources.dir, source_id)?;
        let options = FetchOptions {
            limit: package.limit_for(source_id),
            dry_run,
            force,
        };
        let started = now_rfc3339();
        let result = fetch_source(cfg, &source, &options).await;
        let catalog = Catalog::open(&cfg.catalog_path())?;
        match result {
            Ok(summary) => {
                if !dry_run {
                    catalog.record_run(
                        &source.id,
                        &started,
                        "ok",
                        summary.fetched as i64,
                        summary.bytes as i64,
                        None,
                    )?;
                }
                fetched += summary.fetched;
                failed += summary.failed;
                bytes += summary.bytes;
                println!(
                    "{}: {} indirildi, {} atlandı, {} hata, {} bayt",
                    source.id, summary.fetched, summary.skipped, summary.failed, summary.bytes
                );
            }
            Err(error) => {
                if !dry_run {
                    catalog.record_run(
                        &source.id,
                        &started,
                        "error",
                        0,
                        0,
                        Some(&format!("{error:#}")),
                    )?;
                }
                return Err(error);
            }
        }
    }

    println!(
        "Paket {}: {} indirildi, {} hata, {} bayt{}",
        package.id,
        fetched,
        failed,
        bytes,
        if dry_run { " (dry-run)" } else { "" }
    );
    Ok(())
}

async fn fetch(
    cfg: &Config,
    source_id: Option<String>,
    category: Option<String>,
    limit: Option<usize>,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let sources = match (source_id, category) {
        (Some(id), None) => vec![Source::find(&cfg.sources.dir, &id)?],
        (None, Some(category)) => {
            let all = Source::load_all(&cfg.sources.dir)?;
            let selected: Vec<Source> = all
                .into_iter()
                .filter(|source| source.category == category)
                .collect();
            if selected.is_empty() {
                bail!("kategoride kaynak yok: {category}");
            }
            selected
        }
        (None, None) => bail!("bir kaynak kimliği ya da --category ver"),
        (Some(_), Some(_)) => bail!("kaynak kimliği ve --category birlikte kullanılamaz"),
    };

    let options = FetchOptions {
        limit,
        dry_run,
        force,
    };
    for source in sources {
        let started = now_rfc3339();
        let result = fetch_source(cfg, &source, &options).await;
        let catalog = Catalog::open(&cfg.catalog_path())?;
        match result {
            Ok(summary) => {
                catalog.record_run(
                    &source.id,
                    &started,
                    "ok",
                    summary.fetched as i64,
                    summary.bytes as i64,
                    None,
                )?;
                println!(
                    "{}: {} indirildi, {} atlandı, {} hata, {} bayt",
                    source.id, summary.fetched, summary.skipped, summary.failed, summary.bytes
                );
            }
            Err(error) => {
                catalog.record_run(
                    &source.id,
                    &started,
                    "error",
                    0,
                    0,
                    Some(&format!("{error:#}")),
                )?;
                return Err(error);
            }
        }
    }
    Ok(())
}

fn index(cfg: &Config, rebuild: bool) -> Result<()> {
    let stats = qena_index::index_all(cfg, rebuild)?;
    println!(
        "Dizin: {} belge, {} indekslendi, {} atlandı, {} hata",
        stats.total, stats.indexed, stats.skipped, stats.failed
    );
    Ok(())
}

fn search(
    cfg: &Config,
    query: &str,
    kind: Option<String>,
    source: Option<String>,
    lang: Option<String>,
    limit: usize,
) -> Result<()> {
    let index = qena_index::open_index(&cfg.index_dir(), false)?;
    let options = qena_index::SearchOptions {
        limit,
        kind,
        source,
        lang,
    };
    let hits = qena_index::search(&index, query, &options)?;
    if hits.is_empty() {
        println!("Sonuç yok.");
        return Ok(());
    }
    for (number, hit) in hits.iter().enumerate() {
        println!(
            "{}. {}  [{} · {} · {:.2}]",
            number + 1,
            hit.title,
            hit.kind,
            hit.lang,
            hit.score
        );
        let snippet = hit
            .snippet
            .replace("<b>", "")
            .replace("</b>", "")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">");
        if !snippet.is_empty() {
            println!("   {}", snippet.replace('\n', " "));
        }
        println!("   qena doc {}", hit.id);
    }
    Ok(())
}

async fn save(cfg: &Config, url: &str) -> Result<()> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        bail!("URL http:// veya https:// ile başlamalı");
    }
    let catalog = Catalog::open(&cfg.catalog_path())?;
    let doc = qena_fetch::save_url(cfg, &catalog, url, "web", "site-terms", "web").await?;
    let stats = qena_index::index_all(cfg, false)?;
    println!("Kaydedildi: {}", doc.title);
    println!("Dosya: {}", doc.path);
    println!("Dizin: {} indekslendi", stats.indexed);
    Ok(())
}

async fn process_queue(cfg: &Config, limit: Option<usize>) -> Result<()> {
    let catalog = Catalog::open(&cfg.catalog_path())?;
    let pending = catalog.pending_urls(limit.unwrap_or(20))?;
    if pending.is_empty() {
        println!("Kuyruk boş.");
        return Ok(());
    }
    let mut done = 0usize;
    let mut failed = 0usize;
    for (id, url) in pending {
        catalog.mark_queue(id, "running", None)?;
        match qena_fetch::save_url(cfg, &catalog, &url, "web", "site-terms", "web").await {
            Ok(doc) => {
                catalog.mark_queue(id, "done", None)?;
                done += 1;
                println!("Kaydedildi: {}", doc.title);
            }
            Err(error) => {
                catalog.mark_queue(id, "error", Some(&format!("{error:#}")))?;
                failed += 1;
                println!("Hata ({url}): {error:#}");
            }
        }
    }
    let stats = qena_index::index_all(cfg, false)?;
    println!(
        "Kuyruk: {done} kaydedildi, {failed} hata; dizin {} indekslendi",
        stats.indexed
    );
    Ok(())
}

fn doctor(cfg: &Config) -> Result<()> {
    println!("Qena doktor");
    println!("Veri dizini: {}", cfg.general.data_dir.display());
    check(cfg.general.data_dir.exists(), "veri dizini");
    check(cfg.general.assets_dir.exists(), "varlık dizini (assets)");
    check(cfg.catalog_path().exists(), "katalog");
    check(cfg.index_dir().join("meta.json").exists(), "Tantivy dizini");
    let sources = Source::load_all(&cfg.sources.dir)?;
    println!("Kaynak bildirimi: {}", sources.len());
    let packages = Package::load_all(&cfg.packages.dir)?;
    println!("Hazır paket: {}", packages.len());
    let pmtiles = std::process::Command::new("pmtiles")
        .arg("version")
        .output()
        .is_ok();
    check(pmtiles, "pmtiles aracı (harita indirme)");
    let maps = cfg.general.data_dir.join("maps");
    let map_count = std::fs::read_dir(&maps)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "pmtiles")
                })
                .count()
        })
        .unwrap_or(0);
    println!("Harita paketi: {map_count}");
    let catalog = Catalog::open(&cfg.catalog_path())?;
    println!("Belge: {}", catalog.total()?);
    println!("Kuyruk: {}", catalog.queue_len()?);
    Ok(())
}

fn check(ok: bool, label: &str) {
    println!("  [{}] {}", if ok { "OK" } else { "EKSİK" }, label);
}

fn status(cfg: &Config) -> Result<()> {
    let catalog = Catalog::open(&cfg.catalog_path())?;
    let total = catalog.total()?;
    let counts = catalog.counts()?;
    let queue = catalog.queue_len()?;
    println!("Belge: {total}");
    for (kind, count) in counts {
        println!("  {kind}: {count}");
    }
    println!("Kuyruk: {queue}");
    let index = qena_index::open_index(&cfg.index_dir(), false)?;
    let reader = index.reader()?;
    println!("Dizin: {} belge", reader.searcher().num_docs());
    Ok(())
}

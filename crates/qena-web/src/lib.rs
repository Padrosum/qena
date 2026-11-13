//! Yerel web arayüzü. Yalnızca 127.0.0.1 üzerinde dinler; ağa çıkmaz.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use axum::Router;
use axum::body::Body;
use axum::extract::{Form, Path as AxumPath, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Json, Redirect, Response};
use axum::routing::get;
use maud::{DOCTYPE, Markup, PreEscaped, html};
use pulldown_cmark::{Options, Parser, html as cmark_html};
use qena_core::{Catalog, Config};
use qena_index::{SearchHit, SearchOptions, epub_chapters, extract_text, open_index, search};
use serde::Deserialize;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing::info;

#[derive(Clone)]
struct AppState {
    cfg: Arc<Config>,
    index: Arc<tantivy::Index>,
}

pub async fn serve(cfg: Config, host: &str, port: u16) -> Result<()> {
    let index = open_index(&cfg.index_dir(), false)?;
    let assets_dir = cfg.general.assets_dir.clone();
    let state = AppState {
        cfg: Arc::new(cfg),
        index: Arc::new(index),
    };

    let app = Router::new()
        .route("/", get(home))
        .route("/search", get(search_page))
        .route("/doc/{id}", get(doc_page))
        .route("/raw/{id}", get(raw))
        .route("/library", get(library))
        .route("/library/{id}", get(book_page))
        .route("/guides", get(guides))
        .route("/guides/{slug}", get(guide_page))
        .route("/maps", get(maps_page))
        .route("/maps/{region}/tilejson.json", get(tilejson))
        .route("/styles/{region}/{theme}", get(style_json))
        .route("/tiles/{region}/{z}/{x}/{y}", get(tile))
        .route("/save", get(save_page).post(save_submit))
        .route("/status", get(status_page))
        .nest_service("/assets", ServeDir::new(assets_dir))
        .fallback(not_found)
        .with_state(state);

    let address = format!("{host}:{port}");
    let listener = TcpListener::bind(&address)
        .await
        .with_context(|| format!("{address} dinlenemedi"))?;
    info!("Qena hazır: http://{address}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
struct SearchQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    lang: String,
}

#[derive(Debug, Default, Deserialize)]
struct MapQuery {
    #[serde(default)]
    region: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveForm {
    url: String,
}

#[derive(Debug, Deserialize)]
struct StatusQuery {
    #[serde(default)]
    queued: String,
}

async fn home(State(state): State<AppState>) -> WebResult<Html<String>> {
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    let total = catalog.total()?;
    let counts = catalog.counts()?;
    let queue = catalog.queue_len()?;
    let count = |kind: &str| {
        counts
            .iter()
            .find(|(name, _)| name == kind)
            .map(|(_, value)| *value)
            .unwrap_or(0)
    };

    let body = html! {
        section class="hero" {
            p class="kicker" { "Project Qena" }
            h1 { "Kötü günler için yerel bilgi sandığı." }
            p class="lead" {
                "İnternet, bulut ve dış servis olmadan çalışır. Aradığın bilgi bu makinede kalır."
            }
            form method="get" action="/search" class="big-search" {
                input type="search" name="q" placeholder="Deprem, su arıtma, ilk yardım, İstanbul…" autofocus;
                button { "Ara" }
            }
            p class="muted" {
                (total) " belge · " (count("wiki")) " Vikipedi maddesi · "
                (count("book")) " kitap · " (count("guide")) " rehber"
                @if queue > 0 { " · " (queue) " web kaydı kuyrukta" }
            }
        }

        section class="grid" {
            (category_card("Vikipedi", "Seçili maddeler, çevrimdışı okunur.", "/search?kind=wiki"))
            (category_card("Kitaplar", "Kamu malı kitaplar ve metinler.", "/library"))
            (category_card("Rehberler", "Deprem, su, gıda, ilk yardım kontrol listeleri.", "/guides"))
            (category_card("Haritalar", "Anadolu, Orta Doğu ve Avrupa; sokak ve fiziki katmanlar.", "/maps"))
            (category_card("Web kayıtları", "Bir URL'yi tek dosya olarak sakla.", "/save"))
            (category_card("Durum", "Dizin, kuyruk ve veri dizini.", "/status"))
        }

        section class="note" {
            h2 { "Nasıl çalışır?" }
            p {
                "Ağ yalnızca " code { "qena fetch" } " komutunda kullanılır. "
                "Dizinleyici ve bu arayüz dışarıdan veri çekmez."
            }
        }
    };
    Ok(Html(page("Ana sayfa", "", body).into_string()))
}

async fn search_page(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> WebResult<Html<String>> {
    let options = SearchOptions {
        limit: state.cfg.search.limit,
        kind: non_empty(&params.kind),
        source: non_empty(&params.source),
        lang: non_empty(&params.lang),
    };

    let (hits, error) = match search(&state.index, &params.q, &options) {
        Ok(hits) => (hits, None),
        Err(error) => (Vec::new(), Some(format!("{error:#}"))),
    };
    let error_message = error.unwrap_or_default();
    let has_error = !error_message.is_empty();

    let body = html! {
        section class="search-head" {
            h1 { "Arama" }
            form method="get" action="/search" class="big-search" {
                input type="search" name="q" value=(params.q) placeholder="Ne arıyorsun?" autofocus;
                button { "Ara" }
            }
            p class="muted" {
                @if params.q.trim().is_empty() {
                    "Tüm belgeler gösteriliyor."
                } @else {
                    (hits.len()) " sonuç"
                }
            }
        }
        @if has_error {
            div class="error" { "Sorgu çalıştırılamadı: " (error_message) }
        }
        @if hits.is_empty() && !has_error {
            p class="empty" { "Sonuç yok. Farklı bir kelime dene; Türkçe ekler ve aksanlar sadeleştirilir." }
        } @else {
            section class="results" {
                @for hit in &hits {
                    (result_card(hit))
                }
            }
        }
    };
    Ok(Html(page("Arama", &params.q, body).into_string()))
}

async fn doc_page(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> WebResult<Html<String>> {
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    let Some(doc) = catalog.document(&id)? else {
        return Ok(Html(
            page(
                "Bulunamadı",
                "",
                html! { h1 { "Belge bulunamadı" } p { (id) } },
            )
            .into_string(),
        ));
    };
    let path = safe_path(&state.cfg, &doc.path)?;
    let content = if doc.kind == "guide" {
        let markdown = std::fs::read_to_string(&path).unwrap_or_default();
        markdown_markup(&without_first_heading(&markdown))
    } else {
        let text = extract_text(&path, &doc.mime)
            .unwrap_or_else(|error| format!("Metin çıkarılamadı: {error:#}"));
        let preview = if doc.kind == "book" {
            text.chars().take(40_000).collect::<String>()
        } else {
            text
        };
        html! { div class="text" { (preview) } }
    };

    let body = html! {
        article class="doc" {
            header class="doc-head" {
                p class="kicker" { (kind_label(&doc.kind)) }
                h1 { (doc.title) }
                p class="meta" {
                    "Dil: " (doc.lang) " · Kaynak: " (doc.source_id)
                    @if let Some(revision) = &doc.revision { " · Revizyon: " (revision) }
                }
                p class="meta" {
                    "Lisans: " (doc.license) " · " (doc.attribution)
                }
                p class="meta" {
                    "İndirildi: " (doc.fetched_at)
                }
                div class="actions" {
                    a class="button" href=(format!("/raw/{}", doc.id)) { "Orijinali aç" }
                    @if let Some(url) = &doc.url {
                        a class="button ghost" href=(url) target="_blank" rel="noreferrer" { "Kaynak bağlantısı" }
                    }
                }
            }
            (content)
        }
    };
    Ok(Html(page(&doc.title, "", body).into_string()))
}

async fn raw(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> WebResult<Response> {
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    let Some(doc) = catalog.document(&id)? else {
        return Ok((StatusCode::NOT_FOUND, "Belge bulunamadı").into_response());
    };
    let path = safe_path(&state.cfg, &doc.path)?;
    let bytes = tokio::fs::read(&path)
        .await
        .with_context(|| format!("{} okunamadı", path.display()))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dosya");
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("inline; filename=\"{filename}\""))
            .unwrap_or(HeaderValue::from_static("inline")),
    );
    if doc.kind == "web" {
        response.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "sandbox; default-src 'none'; img-src data:; style-src 'unsafe-inline'",
            ),
        );
        response.headers_mut().insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
    }
    Ok(response)
}

async fn library(State(state): State<AppState>) -> WebResult<Html<String>> {
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    let books = catalog.documents_by_kind("book")?;
    let body = html! {
        section class="search-head" {
            h1 { "Kitaplar" }
            p class="muted" { (books.len()) " kitap · kamu malı / özgür lisans" }
        }
        section class="results" {
            @for book in &books {
                article class="card" {
                    h2 { a href=(format!("/library/{}", book.id)) { (book.title) } }
                    p class="meta" {
                        (book.metadata().get("authors").and_then(|value| value.as_str()).unwrap_or(""))
                        " · " (book.lang) " · " (book.license)
                    }
                    div class="actions" {
                        a class="button" href=(format!("/raw/{}", book.id)) { "EPUB/PDF indir" }
                    }
                }
            }
        }
    };
    Ok(Html(page("Kitaplar", "", body).into_string()))
}

async fn book_page(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> WebResult<Html<String>> {
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    let Some(doc) = catalog.document(&id)? else {
        return Ok(Html(
            page(
                "Bulunamadı",
                "",
                html! { h1 { "Kitap bulunamadı" } p { (id) } },
            )
            .into_string(),
        ));
    };
    let body = book_body(&state.cfg, &doc)?;
    Ok(Html(page(&doc.title, "", body).into_string()))
}

fn book_body(cfg: &Config, doc: &qena_core::DocumentRecord) -> anyhow::Result<Markup> {
    let path = safe_path(cfg, &doc.path)?;
    let chapters = if doc.mime.contains("epub")
        || path
            .extension()
            .is_some_and(|extension| extension == "epub")
    {
        epub_chapters(&path).unwrap_or_default()
    } else {
        Vec::new()
    };

    let (toc, content) = if chapters.is_empty() {
        let text = extract_text(&path, &doc.mime)
            .unwrap_or_else(|error| format!("Metin çıkarılamadı: {error:#}"));
        (
            html! {},
            html! {
                section class="chapter" {
                    h2 { (doc.title) }
                    div class="text" { (text) }
                }
            },
        )
    } else {
        let toc = html! {
            nav class="toc" {
                h2 { "İçindekiler" }
                ol {
                    @for (index, chapter) in chapters.iter().enumerate() {
                        li {
                            a href=(format!("#bolum-{}", index + 1)) { (chapter.title) }
                        }
                    }
                }
            }
        };
        let content = html! {
            @for (index, chapter) in chapters.iter().enumerate() {
                section id=(format!("bolum-{}", index + 1)) class="chapter" {
                    h2 { (chapter.title) }
                    div class="text" { (chapter.text) }
                    p class="chapter-back" {
                        a href="#top" { "İçindekilere dön" }
                    }
                }
            }
        };
        (toc, content)
    };

    let metadata = doc.metadata();
    let authors = metadata
        .get("authors")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();

    Ok(html! {
        article class="doc reader" id="top" {
            header class="doc-head" {
                p class="kicker" { "Kitap" }
                h1 { (doc.title) }
                @if !authors.is_empty() {
                    p class="meta" { (authors) }
                }
                p class="meta" {
                    "Dil: " (doc.lang) " · " (chapters.len()) " bölüm · " (doc.license)
                }
                p class="meta" { (doc.attribution) }
                div class="actions" {
                    a class="button" href=(format!("/raw/{}", doc.id)) { "EPUB/PDF indir" }
                    @if let Some(url) = &doc.url {
                        a class="button ghost" href=(url) target="_blank" rel="noreferrer" { "Kaynak bağlantısı" }
                    }
                }
            }
            (toc)
            (content)
        }
    })
}

async fn guides(State(state): State<AppState>) -> WebResult<Html<String>> {
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    let guides = catalog.documents_by_kind("guide")?;
    let body = html! {
        section class="search-head" {
            h1 { "Rehberler" }
            p class="muted" { "Yazdırılabilir, çevrimdışı kontrol listeleri." }
        }
        section class="grid" {
            @for guide in &guides {
                a class="card link-card" href=(format!("/guides/{}", guide.id.trim_start_matches("guide:"))) {
                    h2 { (guide.title) }
                    p class="meta" { (guide.license) }
                }
            }
        }
    };
    Ok(Html(page("Rehberler", "", body).into_string()))
}

async fn guide_page(
    State(state): State<AppState>,
    AxumPath(slug): AxumPath<String>,
) -> WebResult<Html<String>> {
    let id = if slug.starts_with("guide:") {
        slug
    } else {
        format!("guide:{slug}")
    };
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    let Some(doc) = catalog.document(&id)? else {
        return Ok(Html(
            page(
                "Bulunamadı",
                "",
                html! { h1 { "Rehber bulunamadı" } p { (id) } },
            )
            .into_string(),
        ));
    };
    let path = safe_path(&state.cfg, &doc.path)?;
    let markdown = std::fs::read_to_string(&path).unwrap_or_default();
    let body = html! {
        article class="doc" {
            header class="doc-head" {
                p class="kicker" { "Rehber" }
                h1 { (doc.title) }
                p class="meta" { "Lisans: " (doc.license) " · " (doc.attribution) }
            }
            (markdown_markup(&without_first_heading(&markdown)))
        }
    };
    Ok(Html(page(&doc.title, "", body).into_string()))
}

async fn maps_page(
    State(state): State<AppState>,
    Query(params): Query<MapQuery>,
) -> WebResult<Html<String>> {
    let maps_dir = state.cfg.general.data_dir.join("maps");
    let mut regions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&maps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "pmtiles")
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            {
                regions.push(stem.to_string());
            }
        }
    }
    regions.sort();
    let selected = params
        .region
        .filter(|region| regions.iter().any(|item| item == region))
        .or_else(|| {
            regions
                .iter()
                .find(|region| region.as_str() == "turkey")
                .cloned()
        })
        .or_else(|| regions.first().cloned())
        .unwrap_or_default();
    let has_regions = !regions.is_empty();

    let body = html! {
        section class="search-head" {
            h1 { "Haritalar" }
            p class="lead" {
                "Çevrimdışı vektör haritalar. Sokak katmanları OpenStreetMap (ODbL), "
                "fiziki/idari katmanlar Natural Earth (kamu malı)."
            }
        }
        @if !has_regions {
            div class="note" {
                p {
                    "Sokak haritası henüz indirilmedi. "
                    code { "qena fetch protomaps-turkiye" }
                    " ile Türkiye paketini ekle; fiziki harita şimdiden çalışır."
                }
            }
        }
        div class="map-wrap" {
            div id="map" {}
            div id="map-status" class="map-status" { "Harita yükleniyor…" }
            div class="map-controls" {
                @if has_regions {
                    label class="map-region" {
                        "Bölge "
                        select id="region" {
                            @for region in &regions {
                                @if region == &selected {
                                    option value=(region) selected { (region) }
                                } @else {
                                    option value=(region) { (region) }
                                }
                            }
                        }
                    }
                }
                button type="button" data-theme="street-light" { "Sokak (açık)" }
                button type="button" data-theme="street-dark" { "Sokak (koyu)" }
                button type="button" data-theme="humanitarian" { "İnsani (yüksek kontrast)" }
                button type="button" data-theme="physical" { "Fiziki / İdari" }
            }
        }
        section class="note" {
            h2 { "Bölgeler" }
            @if regions.is_empty() {
                p { "Henüz PMTiles paketi yok." }
            } @else {
                ul {
                    @for region in &regions {
                        li {
                            a href=(format!("/maps?region={region}")) { (region) }
                            " — " code { (format!("/tiles/{region}/{{z}}/{{x}}/{{y}}")) }
                        }
                    }
                }
            }
            p class="meta" {
                "Atıf: © OpenStreetMap katkıcıları (ODbL) · Protomaps · Natural Earth"
            }
        }
        link rel="stylesheet" href="/assets/maplibre/maplibre-gl.css";
        script src="/assets/maplibre/maplibre-gl.js" {}
        script { (PreEscaped(format!("const QENA_MAP_REGION = {selected:?}; {MAP_JS}"))) }
    };
    Ok(Html(page("Haritalar", "", body).into_string()))
}

async fn style_json(
    State(state): State<AppState>,
    AxumPath((region, theme)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> WebResult<Json<serde_json::Value>> {
    if !["street-light", "street-dark", "humanitarian", "physical"].contains(&theme.as_str()) {
        return Err(AppError(anyhow::anyhow!("bilinmeyen stil: {theme}")));
    }
    let base = public_base(&headers);
    let path = state
        .cfg
        .general
        .assets_dir
        .join("styles")
        .join(format!("{theme}.json"));
    let text = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("{} okunamadı", path.display()))?;
    let mut value: serde_json::Value = serde_json::from_str(&text)?;
    value["glyphs"] = serde_json::json!(format!("{base}/assets/fonts/{{fontstack}}/{{range}}.pbf"));
    if let Some(sprite) = value
        .get("sprite")
        .and_then(|sprite| sprite.as_str())
        .map(str::to_string)
        && sprite.starts_with('/')
    {
        value["sprite"] = serde_json::json!(format!("{base}{sprite}"));
    }

    if theme == "physical" {
        if let Some(sources) = value
            .get_mut("sources")
            .and_then(|sources| sources.as_object_mut())
        {
            for source in sources.values_mut() {
                if let Some(data) = source.get("data").and_then(|data| data.as_str())
                    && data.starts_with('/')
                {
                    source["data"] = serde_json::json!(format!("{base}{data}"));
                }
            }
        }
    } else {
        let map_path = state
            .cfg
            .general
            .data_dir
            .join("maps")
            .join(format!("{region}.pmtiles"));
        if !map_path.exists() {
            return Err(AppError(anyhow::anyhow!(
                "harita paketi yok: {}",
                map_path.display()
            )));
        }
        let (minzoom, maxzoom, bounds) =
            tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
                let reader = pmtiles::PmTilesReader::new_with_path(&map_path)?;
                let header = reader.get_header();
                Ok((
                    header.min_zoom,
                    header.max_zoom,
                    [
                        header.min_longitude,
                        header.min_latitude,
                        header.max_longitude,
                        header.max_latitude,
                    ],
                ))
            })
            .await??;
        value["sources"] = serde_json::json!({
            "protomaps": {
                "type": "vector",
                "tiles": [format!("{base}/tiles/{region}/{{z}}/{{x}}/{{y}}")],
                "minzoom": minzoom,
                "maxzoom": maxzoom,
                "bounds": bounds,
                "attribution": "© OpenStreetMap katkıcıları (ODbL) · Protomaps"
            }
        });
    }
    Ok(Json(value))
}

async fn tilejson(
    State(state): State<AppState>,
    AxumPath(region): AxumPath<String>,
    headers: HeaderMap,
) -> WebResult<Json<serde_json::Value>> {
    let base = public_base(&headers);
    let path = state
        .cfg
        .general
        .data_dir
        .join("maps")
        .join(format!("{region}.pmtiles"));
    if !path.exists() {
        return Err(AppError(anyhow::anyhow!(
            "harita paketi yok: {}",
            path.display()
        )));
    }
    let (minzoom, maxzoom, min_lon, min_lat, max_lon, max_lat, center_lon, center_lat, center_zoom) =
        tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let reader = pmtiles::PmTilesReader::new_with_path(&path)?;
            let header = reader.get_header();
            Ok((
                header.min_zoom,
                header.max_zoom,
                header.min_longitude,
                header.min_latitude,
                header.max_longitude,
                header.max_latitude,
                header.center_longitude,
                header.center_latitude,
                header.center_zoom,
            ))
        })
        .await??;

    Ok(Json(serde_json::json!({
        "tilejson": "3.0.0",
        "name": region,
        "scheme": "xyz",
        "tiles": [format!("{base}/tiles/{region}/{{z}}/{{x}}/{{y}}")],
        "minzoom": minzoom,
        "maxzoom": maxzoom,
        "bounds": [min_lon, min_lat, max_lon, max_lat],
        "center": [center_lon, center_lat, center_zoom],
        "attribution": "© OpenStreetMap katkıcıları (ODbL) · Protomaps"
    })))
}

async fn tile(
    State(state): State<AppState>,
    AxumPath((region, z, x, y)): AxumPath<(String, u8, u32, u32)>,
) -> WebResult<Response> {
    let path = state
        .cfg
        .general
        .data_dir
        .join("maps")
        .join(format!("{region}.pmtiles"));
    let bytes = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<Vec<u8>>> {
        let reader = pmtiles::PmTilesReader::new_with_path(&path)?;
        let coordinate = pmtiles::TileCoord::new(z, x, y)?;
        Ok(reader
            .get_tile_decompressed(coordinate)?
            .map(|tile| tile.to_vec()))
    })
    .await??;

    let Some(bytes) = bytes else {
        return Ok((StatusCode::NOT_FOUND, "karo yok").into_response());
    };
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.mapbox-vector-tile"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    Ok(response)
}

async fn save_page() -> Html<String> {
    let body = html! {
        section class="search-head" {
            h1 { "Web kaydı" }
            p class="lead" { "Bir URL'yi arşive ekle. Kuyruk, " code { "qena fetch --queue" } " ile işlenir; bu arayüz ağa çıkmaz." }
        }
        form method="post" action="/save" class="save-form" {
            label { "URL" input type="url" name="url" placeholder="https://…" required; }
            button { "Kuyruğa ekle" }
        }
    };
    Html(page("Web kaydı", "", body).into_string())
}

async fn save_submit(
    State(state): State<AppState>,
    Form(form): Form<SaveForm>,
) -> WebResult<Redirect> {
    let url = form.url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError(anyhow::anyhow!(
            "URL http:// veya https:// ile başlamalı"
        )));
    }
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    catalog.enqueue_url(url)?;
    Ok(Redirect::to("/status?queued=1"))
}

async fn status_page(
    State(state): State<AppState>,
    Query(query): Query<StatusQuery>,
) -> WebResult<Html<String>> {
    let catalog = Catalog::open(&state.cfg.catalog_path())?;
    let total = catalog.total()?;
    let counts = catalog.counts()?;
    let queue = catalog.queue_len()?;
    let reader = state.index.reader()?;
    let indexed = reader.searcher().num_docs();

    let body = html! {
        section class="search-head" {
            h1 { "Durum" }
            @if !query.queued.is_empty() {
                p class="ok" { "URL kuyruğa eklendi." }
            }
        }
        section class="grid" {
            div class="card" { h2 { "Belgeler" } p class="big" { (total) } }
            div class="card" { h2 { "Dizin" } p class="big" { (indexed) } p class="meta" { "Tantivy" } }
            div class="card" { h2 { "Kuyruk" } p class="big" { (queue) } p class="meta" { "bekleyen web kaydı" } }
        }
        section class="note" {
            h2 { "Kategoriler" }
            ul {
                @for (kind, count) in &counts {
                    li { (kind_label(kind)) ": " (count) }
                }
            }
            h2 { "Yollar" }
            p { "Veri: " code { (state.cfg.general.data_dir.display().to_string()) } }
            p { "Dizin: " code { (state.cfg.index_dir().display().to_string()) } }
        }
    };
    Ok(Html(page("Durum", "", body).into_string()))
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html(page("Bulunamadı", "", html! { h1 { "Sayfa yok" } }).into_string()),
    )
}

fn category_card(title: &str, description: &str, href: &str) -> Markup {
    html! {
        a class="card link-card" href=(href) {
            h2 { (title) }
            p { (description) }
        }
    }
}

fn result_card(hit: &SearchHit) -> Markup {
    let href = if hit.kind == "book" {
        format!("/library/{}", hit.id)
    } else {
        format!("/doc/{}", hit.id)
    };
    html! {
        article class="card" {
            p class="kicker" { (kind_label(&hit.kind)) " · " (hit.lang) }
            h2 { a href=(href) { (hit.title) } }
            @if !hit.snippet.is_empty() {
                p class="snippet" { (PreEscaped(hit.snippet.clone())) }
            }
            p class="meta" {
                "Kaynak: " (hit.source) " · Lisans: " (hit.license)
                " · " (hit.fetched_at)
            }
        }
    }
}

fn markdown_markup(markdown: &str) -> Markup {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut out = String::new();
    cmark_html::push_html(&mut out, parser);
    PreEscaped(out)
}

fn without_first_heading(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut skipped = false;
    for line in markdown.lines() {
        if !skipped && out.trim().is_empty() && line.trim_start().starts_with("# ") {
            skipped = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn page(title: &str, query: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="tr" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · Qena" }
                style { (PreEscaped(CSS)) }
            }
            body {
                header class="top" {
                    a class="brand" href="/" { "Qena" }
                    form method="get" action="/search" class="search" {
                        input type="search" name="q" value=(query) placeholder="Ara…";
                        button { "Ara" }
                    }
                    nav {
                        a href="/library" { "Kitaplar" }
                        a href="/guides" { "Rehberler" }
                        a href="/maps" { "Haritalar" }
                        a href="/save" { "Kaydet" }
                        a href="/status" { "Durum" }
                    }
                }
                main { (content) }
                footer {
                    p { "Qena · yerel, çevrimdışı bilgi sandığı · kod GPL-3.0" }
                    p { "İçerik lisansı kaynağında; her belgede gösterilir." }
                }
            }
        }
    }
}

fn public_base(headers: &HeaderMap) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("127.0.0.1:8787");
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("http");
    format!("{scheme}://{host}")
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "wiki" => "Vikipedi",
        "book" => "Kitap",
        "guide" => "Rehber",
        "web" => "Web kaydı",
        _ => "Belge",
    }
}

fn non_empty(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.trim().to_string())
    }
}

fn safe_path(cfg: &Config, relative: &str) -> Result<PathBuf> {
    let base = cfg
        .general
        .data_dir
        .canonicalize()
        .context("veri dizini bulunamadı")?;
    let path = base.join(relative);
    let path = path
        .canonicalize()
        .with_context(|| format!("{} bulunamadı", path.display()))?;
    if !path.starts_with(&base) {
        bail!("veri dizini dışına çıkılamaz");
    }
    Ok(path)
}

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Qena hatası: {:#}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

type WebResult<T> = Result<T, AppError>;

const MAP_JS: &str = r#"
const state = { region: QENA_MAP_REGION, theme: 'street-light' };
const styleUrl = () => state.theme === 'physical'
  ? '/styles/' + state.region + '/physical'
  : '/styles/' + state.region + '/' + state.theme;
const status = document.getElementById('map-status');

const map = new maplibregl.Map({
  container: 'map',
  style: styleUrl(),
  center: [35.5, 39.0],
  zoom: 5,
  attributionControl: false
});
map.on('error', (event) => {
  const message = event && event.error ? event.error.message : String(event);
  if (status) status.textContent = 'Harita hatası: ' + message;
});
map.on('style.load', () => {
  map.resize();
  map.triggerRepaint();
  if (status) status.textContent = 'Stil yüklendi · kaynak bekleniyor…';
});
map.on('sourcedata', (event) => {
  if (status && event.sourceId && event.isSourceLoaded) {
    status.textContent = 'Kaynak yüklendi · ' + (map.areTilesLoaded() ? 'karolar hazır' : 'karolar yükleniyor…');
  }
});
map.on('idle', () => {
  if (status) status.textContent = 'Harita hazır · ' + map.getStyle().layers.length + ' katman';
});
map.addControl(new maplibregl.NavigationControl(), 'top-right');
map.addControl(new maplibregl.ScaleControl({ unit: 'metric' }), 'bottom-left');
map.addControl(new maplibregl.AttributionControl({
  customAttribution: '© OpenStreetMap katkıcıları (ODbL) · Protomaps · Natural Earth'
}), 'bottom-right');
setTimeout(() => {
  map.resize();
  map.triggerRepaint();
}, 1000);

const regionSelect = document.getElementById('region');
if (regionSelect) {
  regionSelect.addEventListener('change', () => {
    state.region = regionSelect.value;
    map.setStyle(styleUrl());
  });
}
document.querySelectorAll('[data-theme]').forEach((button) => {
  button.addEventListener('click', () => {
    state.theme = button.dataset.theme;
    map.setStyle(styleUrl());
  });
});
"#;

const CSS: &str = r#"
:root {
  --bg: #f7f2e8;
  --fg: #211b14;
  --muted: #6f6355;
  --card: #fffaf1;
  --line: #ddcfbb;
  --accent: #8a4f2b;
  --accent-fg: #fff8ee;
  --ok: #2f6b45;
  --err: #9a2f2f;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #171410;
    --fg: #ece2d2;
    --muted: #a99a86;
    --card: #211c16;
    --line: #3a3127;
    --accent: #d79a6a;
    --accent-fg: #24170d;
    --ok: #8fce9f;
    --err: #f0a0a0;
  }
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--fg);
  font: 16px/1.55 ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
}
a { color: var(--accent); }
code {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: .92em;
  background: color-mix(in srgb, var(--line) 45%, transparent);
  padding: .1em .35em;
  border-radius: 4px;
}
.top {
  display: flex;
  flex-wrap: wrap;
  gap: .8rem 1.2rem;
  align-items: center;
  padding: .9rem clamp(1rem, 4vw, 3rem);
  border-bottom: 1px solid var(--line);
  position: sticky;
  top: 0;
  background: color-mix(in srgb, var(--bg) 92%, transparent);
  backdrop-filter: blur(8px);
}
.brand {
  font-weight: 800;
  letter-spacing: .08em;
  text-decoration: none;
  font-size: 1.2rem;
}
.top nav { display: flex; flex-wrap: wrap; gap: .9rem; margin-left: auto; }
.top nav a { text-decoration: none; color: var(--muted); }
.top nav a:hover { color: var(--accent); }
.search { display: flex; gap: .4rem; flex: 1 1 240px; max-width: 460px; }
.search input, .big-search input, .save-form input {
  width: 100%;
  padding: .6rem .75rem;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--card);
  color: var(--fg);
  font: inherit;
}
.search button, .big-search button, .save-form button, .button {
  border: 1px solid var(--accent);
  background: var(--accent);
  color: var(--accent-fg);
  padding: .55rem .9rem;
  border-radius: 8px;
  font: inherit;
  cursor: pointer;
  text-decoration: none;
  display: inline-block;
}
.button.ghost { background: transparent; color: var(--accent); }
main { width: min(1050px, 100% - 2rem); margin: 0 auto; padding: 2rem 0 4rem; }
.hero { padding: 2.2rem 0 1.2rem; }
.kicker {
  text-transform: uppercase;
  letter-spacing: .14em;
  font-size: .72rem;
  color: var(--muted);
  margin: 0 0 .4rem;
}
h1 { font-size: clamp(1.6rem, 3.4vw, 2.5rem); line-height: 1.15; margin: .2rem 0 .6rem; }
h2 { font-size: 1.15rem; margin: 0 0 .4rem; }
.lead { font-size: 1.1rem; color: var(--muted); max-width: 62ch; }
.big-search { display: flex; gap: .5rem; margin: 1.2rem 0 .6rem; max-width: 720px; }
.muted, .meta { color: var(--muted); font-size: .9rem; }
.grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: 1rem; margin: 1.4rem 0; }
.card {
  background: var(--card);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 1rem 1.1rem;
}
.link-card { text-decoration: none; color: inherit; display: block; }
.link-card:hover { border-color: var(--accent); }
.results { display: grid; gap: .9rem; margin-top: 1.2rem; }
.snippet { margin: .4rem 0; }
.snippet b { background: color-mix(in srgb, var(--accent) 22%, transparent); padding: 0 .1em; border-radius: 3px; }
.note, .doc, .search-head { margin-top: 1.4rem; }
.note { border-top: 1px solid var(--line); padding-top: 1.2rem; }
.doc-head { border-bottom: 1px solid var(--line); padding-bottom: 1rem; margin-bottom: 1.2rem; }
.doc-head .actions { display: flex; gap: .6rem; flex-wrap: wrap; margin-top: .8rem; }
.text { white-space: pre-wrap; max-width: 78ch; font-size: 1rem; }
.doc h2, .doc h3 { margin-top: 1.4em; }
.doc p { max-width: 75ch; }
.doc ul, .doc ol { max-width: 75ch; }
.doc blockquote { border-left: 3px solid var(--line); margin-left: 0; padding-left: 1rem; color: var(--muted); }
.doc table { border-collapse: collapse; width: 100%; display: block; overflow-x: auto; }
.doc th, .doc td { border: 1px solid var(--line); padding: .4rem .6rem; text-align: left; }
.toc {
  background: var(--card);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 1rem 1.2rem;
  margin: 1.2rem 0 1.8rem;
}
.toc h2 { font-size: 1.05rem; }
.toc ol { margin: .4rem 0 0; padding-left: 1.4rem; columns: 2; column-gap: 2rem; }
.toc li { margin: .15rem 0; break-inside: avoid; }
.chapter { border-top: 1px solid var(--line); margin-top: 1.8rem; padding-top: 1.2rem; }
.chapter:first-of-type { border-top: 0; }
.chapter h2 { font-size: 1.35rem; }
.chapter-back { font-size: .85rem; }
@media (max-width: 620px) { .toc ol { columns: 1; } }
.save-form { display: grid; gap: .8rem; max-width: 620px; margin-top: 1.2rem; }
.save-form label { display: grid; gap: .35rem; font-weight: 600; }
.big { font-size: 2rem; font-weight: 800; margin: .2rem 0; }
.ok { color: var(--ok); }
.error { color: var(--err); border: 1px solid var(--err); padding: .7rem .9rem; border-radius: 8px; }
.empty { color: var(--muted); }
footer {
  border-top: 1px solid var(--line);
  padding: 1.2rem clamp(1rem, 4vw, 3rem);
  color: var(--muted);
  font-size: .85rem;
}
footer p { margin: .2rem 0; }
.map-wrap {
  position: relative;
  margin: 1.2rem 0;
  border: 1px solid var(--line);
  border-radius: 12px;
  overflow: hidden;
}
#map { height: 70vh; min-height: 420px; background: var(--card); }
.map-status {
  position: absolute;
  left: .7rem;
  top: .7rem;
  z-index: 2;
  max-width: min(80%, 520px);
  background: color-mix(in srgb, var(--card) 92%, transparent);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: .35rem .55rem;
  color: var(--muted);
  font-size: .8rem;
  pointer-events: none;
}
.map-controls {
  display: flex;
  flex-wrap: wrap;
  gap: .4rem;
  padding: .6rem;
  border-top: 1px solid var(--line);
  background: var(--card);
}
.map-controls button {
  border: 1px solid var(--line);
  background: transparent;
  color: var(--fg);
  padding: .4rem .7rem;
  border-radius: 7px;
  font: inherit;
  cursor: pointer;
}
.map-controls button:hover { border-color: var(--accent); color: var(--accent); }
.map-region { display: flex; align-items: center; gap: .4rem; font-size: .9rem; }
.map-region select {
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--card);
  color: var(--fg);
  padding: .4rem .6rem;
  font: inherit;
}
@media print {
  .top, footer, .actions { display: none; }
  body { background: #fff; color: #000; }
  .doc { margin: 0; }
}
"#;

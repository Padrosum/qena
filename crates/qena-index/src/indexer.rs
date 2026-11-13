use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use epub::doc::EpubDoc;
use html2text::render::TrivialDecorator;
use qena_core::{Catalog, Config};
use scraper::{Html, Selector};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TantivyDocument, TextFieldIndexing, TextOptions,
};
use tantivy::tokenizer::{
    AsciiFoldingFilter, Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer,
    TextAnalyzer,
};
use tantivy::{Index, IndexWriter, Term};
use tracing::{info, warn};

pub const TOKENIZER: &str = "tr_fold";

#[derive(Debug, Clone)]
pub struct Fields {
    pub id: Field,
    pub title: Field,
    pub body: Field,
    pub headings: Field,
    pub kind: Field,
    pub source: Field,
    pub lang: Field,
    pub url: Field,
    pub license: Field,
    pub attribution: Field,
    pub path: Field,
    pub fetched_at: Field,
    pub revision: Field,
}

impl Fields {
    pub fn new(schema: &Schema) -> Self {
        let field = |name: &str| schema.get_field(name).expect("şema alanı eksik");
        Self {
            id: field("id"),
            title: field("title"),
            body: field("body"),
            headings: field("headings"),
            kind: field("kind"),
            source: field("source"),
            lang: field("lang"),
            url: field("url"),
            license: field("license"),
            attribution: field("attribution"),
            path: field("path"),
            fetched_at: field("fetched_at"),
            revision: field("revision"),
        }
    }
}

#[derive(Debug, Default)]
pub struct IndexStats {
    pub total: usize,
    pub indexed: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub text: String,
}

fn text_options() -> TextOptions {
    TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(TOKENIZER)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored()
}

fn build_schema() -> Schema {
    let mut builder = Schema::builder();
    let text = text_options();
    builder.add_text_field("id", tantivy::schema::STRING | tantivy::schema::STORED);
    builder.add_text_field("title", text.clone());
    builder.add_text_field("body", text.clone());
    builder.add_text_field("headings", text);
    for name in ["kind", "source", "lang"] {
        builder.add_text_field(
            name,
            tantivy::schema::STRING | tantivy::schema::STORED | tantivy::schema::FAST,
        );
    }
    for name in [
        "url",
        "license",
        "attribution",
        "path",
        "fetched_at",
        "revision",
    ] {
        builder.add_text_field(name, tantivy::schema::STRING | tantivy::schema::STORED);
    }
    builder.build()
}

fn analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .filter(AsciiFoldingFilter)
        .filter(Stemmer::new(Language::Turkish))
        .build()
}

pub fn open_index(dir: &Path, rebuild: bool) -> Result<Index> {
    if rebuild && dir.exists() {
        std::fs::remove_dir_all(dir).with_context(|| format!("{} silinemedi", dir.display()))?;
    }
    std::fs::create_dir_all(dir)?;

    let index = if dir.join("meta.json").exists() {
        Index::open_in_dir(dir).with_context(|| format!("{} açılamadı", dir.display()))?
    } else {
        Index::create_in_dir(dir, build_schema())
            .with_context(|| format!("{} oluşturulamadı", dir.display()))?
    };
    index.tokenizers().register(TOKENIZER, analyzer());
    Ok(index)
}

pub fn index_all(cfg: &Config, rebuild: bool) -> Result<IndexStats> {
    let catalog = Catalog::open(&cfg.catalog_path())?;
    let index = open_index(&cfg.index_dir(), rebuild)?;
    let fields = Fields::new(&index.schema());
    let documents = catalog.documents()?;
    let manifest_path = cfg.index_dir().join(".manifest.json");
    let mut manifest: HashMap<String, String> = if rebuild {
        HashMap::new()
    } else {
        std::fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    };

    let mut writer: IndexWriter = index.writer(100_000_000)?;
    let mut stats = IndexStats {
        total: documents.len(),
        ..IndexStats::default()
    };

    for doc in documents {
        if doc.kind == "map" {
            stats.skipped += 1;
            continue;
        }
        if !rebuild && manifest.get(&doc.id) == Some(&doc.hash) {
            stats.skipped += 1;
            continue;
        }

        let path = cfg.general.data_dir.join(&doc.path);
        let text = match extract_text(&path, &doc.mime) {
            Ok(text) => text,
            Err(error) => {
                warn!(doc = %doc.id, error = %error, "metin çıkarılamadı");
                stats.failed += 1;
                continue;
            }
        };
        if text.trim().is_empty() {
            warn!(doc = %doc.id, "metin boş");
            stats.failed += 1;
            continue;
        }

        writer.delete_term(Term::from_field_text(fields.id, &doc.id));
        let mut tantivy_doc = TantivyDocument::default();
        tantivy_doc.add_text(fields.id, &doc.id);
        tantivy_doc.add_text(fields.title, &doc.title);
        tantivy_doc.add_text(fields.body, &text);
        tantivy_doc.add_text(fields.headings, &doc.title);
        tantivy_doc.add_text(fields.kind, &doc.kind);
        tantivy_doc.add_text(fields.source, &doc.source_id);
        tantivy_doc.add_text(fields.lang, &doc.lang);
        tantivy_doc.add_text(fields.url, doc.url.as_deref().unwrap_or(""));
        tantivy_doc.add_text(fields.license, &doc.license);
        tantivy_doc.add_text(fields.attribution, &doc.attribution);
        tantivy_doc.add_text(fields.path, &doc.path);
        tantivy_doc.add_text(fields.fetched_at, &doc.fetched_at);
        tantivy_doc.add_text(fields.revision, doc.revision.as_deref().unwrap_or(""));
        writer.add_document(tantivy_doc)?;

        manifest.insert(doc.id.clone(), doc.hash.clone());
        stats.indexed += 1;
    }

    writer.commit()?;
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).context("dizin manifestosu üretilemedi")?,
    )?;
    info!(
        total = stats.total,
        indexed = stats.indexed,
        skipped = stats.skipped,
        failed = stats.failed,
        "dizinleme bitti"
    );
    Ok(stats)
}

pub fn extract_text(path: &Path, mime: &str) -> Result<String> {
    if !path.exists() {
        bail!("dosya yok: {}", path.display());
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let text = if mime.contains("html") || extension == "html" || extension == "htm" {
        let html = std::fs::read_to_string(path)?;
        html_to_text(&html)?
    } else if mime.contains("epub") || extension == "epub" {
        epub_to_text(path)?
    } else if mime.contains("pdf") || extension == "pdf" {
        pdf_extract::extract_text(path).context("PDF metni çıkarılamadı")?
    } else {
        std::fs::read_to_string(path).unwrap_or_default()
    };

    let normalized = normalize(&text);
    Ok(normalized.chars().take(8_000_000).collect())
}

fn html_to_text(html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    for candidate in [".mw-parser-output", "article", "main", "body"] {
        if let Ok(selector) = Selector::parse(candidate)
            && let Some(element) = document.select(&selector).next()
        {
            let inner = element.inner_html();
            let text =
                html2text::from_read_with_decorator(inner.as_bytes(), 120, TrivialDecorator::new())
                    .context("HTML metne çevrilemedi")?;
            return Ok(text);
        }
    }
    let text = html2text::from_read_with_decorator(html.as_bytes(), 120, TrivialDecorator::new())
        .context("HTML metne çevrilemedi")?;
    Ok(text)
}

pub fn epub_chapters(path: &Path) -> Result<Vec<Chapter>> {
    let mut doc =
        EpubDoc::new(path).with_context(|| format!("{} EPUB olarak açılamadı", path.display()))?;
    let mut labels = std::collections::HashMap::new();
    let mut stack: Vec<&epub::doc::NavPoint> = doc.toc.iter().collect();
    while let Some(nav) = stack.pop() {
        if let Some(chapter) = doc.resource_uri_to_chapter(&nav.content) {
            labels.entry(chapter).or_insert_with(|| nav.label.clone());
        }
        stack.extend(nav.children.iter());
    }

    let chapters = doc.get_num_chapters();
    let mut result = Vec::new();
    for chapter in 0..chapters {
        if !doc.set_current_chapter(chapter) {
            continue;
        }
        let Some((html, _mime)) = doc.get_current_str() else {
            continue;
        };
        let text = match html_to_text(&html) {
            Ok(text) => text,
            Err(_) => continue,
        };
        if text.trim().len() < 20 {
            continue;
        }
        let title = labels
            .get(&chapter)
            .cloned()
            .filter(|label| !label.trim().is_empty())
            .or_else(|| chapter_title(&html))
            .unwrap_or_else(|| format!("Bölüm {}", chapter + 1));
        result.push(Chapter { title, text });
    }
    Ok(result)
}

fn chapter_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    for candidate in ["h1", "h2"] {
        if let Ok(selector) = Selector::parse(candidate) {
            for element in document.select(&selector) {
                let title = element.text().collect::<String>();
                let title = clean_title(title);
                if !title.is_empty() {
                    return Some(title.chars().take(120).collect());
                }
            }
        }
    }
    if let Ok(selector) = Selector::parse("title")
        && let Some(element) = document.select(&selector).next()
    {
        let title = clean_title(element.text().collect::<String>());
        if !title.is_empty() {
            return Some(title.chars().take(120).collect());
        }
    }
    None
}

fn clean_title(title: String) -> String {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(position) = title.rfind("CHAPTER ") {
        let tail = &title[position..];
        let rest = tail["CHAPTER ".len()..].trim_start();
        if rest
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit() || "IVXLCDM".contains(character))
        {
            return tail.to_string();
        }
    }
    title
}

fn epub_to_text(path: &Path) -> Result<String> {
    let mut out = String::new();
    for chapter in epub_chapters(path)? {
        out.push_str(&chapter.title);
        out.push_str("\n\n");
        out.push_str(&chapter.text);
        out.push_str("\n\n");
    }
    Ok(out)
}

fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank = 0usize;
    for line in text.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            blank += 1;
            if blank <= 1 {
                out.push('\n');
            }
        } else {
            blank = 0;
            out.push_str(line);
            out.push('\n');
        }
    }
    out.trim().to_string()
}

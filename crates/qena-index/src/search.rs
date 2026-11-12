use anyhow::{Context, Result};
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{Field, IndexRecordOption, TantivyDocument, Value};
use tantivy::snippet::SnippetGenerator;
use tantivy::{Index, Term};

use crate::indexer::Fields;

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub limit: usize,
    pub kind: Option<String>,
    pub source: Option<String>,
    pub lang: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            kind: None,
            source: None,
            lang: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub source: String,
    pub lang: String,
    pub url: String,
    pub license: String,
    pub attribution: String,
    pub fetched_at: String,
    pub score: f32,
    pub snippet: String,
}

pub fn search(index: &Index, query_text: &str, options: &SearchOptions) -> Result<Vec<SearchHit>> {
    let reader = index.reader().context("dizin okuyucusu açılamadı")?;
    let searcher = reader.searcher();
    let fields = Fields::new(&index.schema());
    let limit = if options.limit == 0 {
        20
    } else {
        options.limit
    };
    let trimmed = query_text.trim();

    let base: Box<dyn Query> = if trimmed.is_empty() {
        Box::new(AllQuery)
    } else {
        let parser =
            QueryParser::for_index(index, vec![fields.title, fields.body, fields.headings]);
        Box::new(
            parser
                .parse_query(trimmed)
                .context("sorgu ayrıştırılamadı")?,
        )
    };

    let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, base)];
    if let Some(kind) = options.kind.as_deref().filter(|value| !value.is_empty()) {
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(fields.kind, kind),
                IndexRecordOption::Basic,
            )),
        ));
    }
    if let Some(source) = options.source.as_deref().filter(|value| !value.is_empty()) {
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(fields.source, source),
                IndexRecordOption::Basic,
            )),
        ));
    }
    if let Some(lang) = options.lang.as_deref().filter(|value| !value.is_empty()) {
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(fields.lang, lang),
                IndexRecordOption::Basic,
            )),
        ));
    }
    let query = BooleanQuery::new(clauses);

    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(limit).order_by_score())
        .context("arama başarısız")?;
    let snippet_generator = SnippetGenerator::create(&searcher, &query, fields.body).ok();

    let mut hits = Vec::with_capacity(top_docs.len());
    for (score, address) in top_docs {
        let doc: TantivyDocument = searcher.doc(address)?;
        let snippet = snippet_generator
            .as_ref()
            .map(|generator| generator.snippet_from_doc(&doc).to_html())
            .unwrap_or_default();
        hits.push(SearchHit {
            id: text(&doc, fields.id),
            title: text(&doc, fields.title),
            kind: text(&doc, fields.kind),
            source: text(&doc, fields.source),
            lang: text(&doc, fields.lang),
            url: text(&doc, fields.url),
            license: text(&doc, fields.license),
            attribution: text(&doc, fields.attribution),
            fetched_at: text(&doc, fields.fetched_at),
            score,
            snippet,
        });
    }
    Ok(hits)
}

fn text(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::{Fields, open_index};
    use tantivy::schema::TantivyDocument;

    fn test_index(dir: &std::path::Path) -> Index {
        let index = open_index(dir, true).unwrap();
        let fields = Fields::new(&index.schema());
        let mut writer = index.writer(50_000_000).unwrap();
        let mut doc = TantivyDocument::default();
        doc.add_text(fields.id, "wiki:tr:1");
        doc.add_text(fields.title, "Gıda güvenliği");
        doc.add_text(
            fields.body,
            "Kitaplar ve deprem çantası için gıda saklama yöntemleri.",
        );
        doc.add_text(fields.kind, "wiki");
        doc.add_text(fields.source, "test");
        doc.add_text(fields.lang, "tr");
        writer.add_document(doc).unwrap();
        writer.commit().unwrap();
        index
    }

    #[test]
    fn turkish_folding_and_stemming() {
        let dir = tempfile::tempdir().unwrap();
        let index = test_index(dir.path());

        let hits = search(&index, "gida", &SearchOptions::default()).unwrap();
        assert_eq!(hits.len(), 1, "aksansız gida, Gıda ile eşleşmeli");

        let hits = search(&index, "kitaplar", &SearchOptions::default()).unwrap();
        assert_eq!(hits.len(), 1, "kitaplar, kitap köküyle eşleşmeli");
    }

    #[test]
    fn kind_filter_excludes_other_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let index = test_index(dir.path());
        let options = SearchOptions {
            kind: Some("book".to_string()),
            ..SearchOptions::default()
        };
        let hits = search(&index, "gida", &options).unwrap();
        assert!(hits.is_empty());
    }
}

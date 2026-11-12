//! Tantivy dizini, Türkçe analiz ve belge metni çıkarma.

mod indexer;
mod search;

pub use indexer::{IndexStats, extract_text, index_all, open_index};
pub use search::{SearchHit, SearchOptions, search};

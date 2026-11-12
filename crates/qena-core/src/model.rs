use serde::{Deserialize, Serialize};

/// Katalogda tutulan belge kaydı. `path`, `data_dir` içindeki göreli yoldur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub id: String,
    pub source_id: String,
    pub kind: String,
    pub title: String,
    pub lang: String,
    pub url: Option<String>,
    pub license: String,
    pub attribution: String,
    pub path: String,
    pub mime: String,
    pub hash: String,
    pub fetched_at: String,
    pub revision: Option<String>,
    pub metadata_json: String,
}

impl DocumentRecord {
    pub fn metadata(&self) -> serde_json::Value {
        serde_json::from_str(&self.metadata_json).unwrap_or(serde_json::Value::Null)
    }
}

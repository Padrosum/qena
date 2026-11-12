use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceManifest {
    pub source: Source,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub category: String,
    pub title: String,
    pub kind: String,
    pub license: String,
    pub attribution: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "empty_table")]
    pub params: toml::Value,
}

fn empty_table() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

impl Source {
    pub fn load_all(dir: &Path) -> Result<Vec<Source>> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(error).with_context(|| format!("{} okunamadı", dir.display()));
            }
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .collect();
        paths.sort();

        let mut sources = Vec::new();
        for path in paths {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("{} okunamadı", path.display()))?;
            let manifest: SourceManifest = toml::from_str(&text)
                .with_context(|| format!("{} kaynak bildirimi değil", path.display()))?;
            let source = manifest.source;
            if source.id.trim().is_empty()
                || source.category.trim().is_empty()
                || source.kind.trim().is_empty()
                || source.license.trim().is_empty()
                || source.attribution.trim().is_empty()
            {
                bail!(
                    "{}: id, category, kind, license ve attribution zorunlu",
                    path.display()
                );
            }
            sources.push(source);
        }
        sources.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| a.title.cmp(&b.title))
        });
        Ok(sources)
    }

    pub fn find(dir: &Path, id: &str) -> Result<Source> {
        let sources = Self::load_all(dir)?;
        sources
            .into_iter()
            .find(|source| source.id == id)
            .with_context(|| format!("kaynak bulunamadı: {id}"))
    }

    pub fn param(&self, key: &str) -> Option<&toml::Value> {
        self.params.get(key)
    }

    pub fn param_str(&self, key: &str) -> Option<&str> {
        self.param(key).and_then(|value| value.as_str())
    }

    pub fn param_bool(&self, key: &str) -> bool {
        self.param(key)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    pub fn param_usize(&self, key: &str) -> Option<usize> {
        self.param(key)
            .and_then(|value| value.as_integer())
            .and_then(|value| usize::try_from(value).ok())
    }

    pub fn param_array(&self, key: &str) -> Vec<String> {
        self.param(key)
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }
}

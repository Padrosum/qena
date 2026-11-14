use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package: Package,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub title: String,
    pub description: String,
    pub license: String,
    pub estimated_size: String,
    pub sources: Vec<String>,
    #[serde(default)]
    pub limits: HashMap<String, usize>,
}

impl Package {
    pub fn load_all(dir: &Path) -> Result<Vec<Package>> {
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

        let mut packages = Vec::new();
        for path in paths {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("{} okunamadı", path.display()))?;
            let manifest: PackageManifest = toml::from_str(&text)
                .with_context(|| format!("{} paket bildirimi değil", path.display()))?;
            let package = manifest.package;
            if package.id.trim().is_empty()
                || package.title.trim().is_empty()
                || package.description.trim().is_empty()
                || package.license.trim().is_empty()
                || package.sources.is_empty()
            {
                bail!(
                    "{}: id, title, description, license ve en az bir sources zorunlu",
                    path.display()
                );
            }
            if packages
                .iter()
                .any(|existing: &Package| existing.id == package.id)
            {
                bail!("yinelenen paket kimliği: {}", package.id);
            }
            packages.push(package);
        }
        packages.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(packages)
    }

    pub fn find(dir: &Path, id: &str) -> Result<Package> {
        Self::load_all(dir)?
            .into_iter()
            .find(|package| package.id == id)
            .with_context(|| format!("paket bulunamadı: {id}"))
    }

    pub fn limit_for(&self, source_id: &str) -> Option<usize> {
        self.limits.get(source_id).copied()
    }
}

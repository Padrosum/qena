use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub sources: Sources,
    pub fetch: Fetch,
    pub search: Search,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub data_dir: PathBuf,
    pub host: String,
    pub port: u16,
    pub user_agent: String,
}

impl Default for General {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            host: "127.0.0.1".to_string(),
            port: 8787,
            user_agent: "Qena/0.1 (https://github.com/Padrosum/qena; offline archive)".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Sources {
    pub dir: PathBuf,
}

impl Default for Sources {
    fn default() -> Self {
        Self {
            dir: PathBuf::from("sources"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Fetch {
    pub delay_ms: u64,
    pub timeout_secs: u64,
}

impl Default for Fetch {
    fn default() -> Self {
        Self {
            delay_ms: 250,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Search {
    pub limit: usize,
}

impl Default for Search {
    fn default() -> Self {
        Self { limit: 20 }
    }
}

impl Config {
    /// Yapılandırmayı okur. Dosya yoksa varsayılanı döndürür.
    /// Göreli yollar, yapılandırma dosyasının klasörüne (yoksa çalışma
    /// klasörüne) göre çözülür.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let cwd = std::env::current_dir().context("çalışma klasörü okunamadı")?;
        let (mut cfg, base) = match path {
            Some(p) if p.exists() => {
                let text =
                    fs::read_to_string(p).with_context(|| format!("{} okunamadı", p.display()))?;
                let cfg: Config = toml::from_str(&text)
                    .with_context(|| format!("{} ayrıştırılamadı", p.display()))?;
                let base = p.parent().unwrap_or(Path::new(".")).to_path_buf();
                (cfg, base)
            }
            _ => (Config::default(), cwd),
        };
        cfg.resolve(&base);
        Ok(cfg)
    }

    pub fn resolve(&mut self, base: &Path) {
        if self.general.data_dir.is_relative() {
            self.general.data_dir = base.join(&self.general.data_dir);
        }
        if self.sources.dir.is_relative() {
            self.sources.dir = base.join(&self.sources.dir);
        }
    }

    pub fn write_default(path: &Path) -> Result<bool> {
        if path.exists() {
            return Ok(false);
        }
        let text = toml::to_string_pretty(&Config::default())
            .context("varsayılan yapılandırma üretilemedi")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(path, text).with_context(|| format!("{} yazılamadı", path.display()))?;
        Ok(true)
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        let base = &self.general.data_dir;
        for dir in [
            base.to_path_buf(),
            self.wiki_dir("tr"),
            self.books_dir("gutenberg"),
            self.guides_dir(),
            self.web_dir(),
            base.join("index"),
            base.join("licenses"),
            base.join("maps"),
            base.join("assets"),
        ] {
            fs::create_dir_all(&dir)
                .with_context(|| format!("{} oluşturulamadı", dir.display()))?;
        }
        Ok(())
    }

    pub fn catalog_path(&self) -> PathBuf {
        self.general.data_dir.join("catalog.db")
    }

    pub fn index_dir(&self) -> PathBuf {
        self.general.data_dir.join("index")
    }

    pub fn wiki_dir(&self, lang: &str) -> PathBuf {
        self.general
            .data_dir
            .join("content")
            .join("wiki")
            .join(lang)
    }

    pub fn books_dir(&self, source: &str) -> PathBuf {
        self.general
            .data_dir
            .join("content")
            .join("books")
            .join(source)
    }

    pub fn guides_dir(&self) -> PathBuf {
        self.general.data_dir.join("content").join("guides")
    }

    pub fn web_dir(&self) -> PathBuf {
        self.general.data_dir.join("content").join("web")
    }
}

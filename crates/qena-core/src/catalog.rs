use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

use crate::model::DocumentRecord;
use crate::source::Source;
use crate::util::now_rfc3339;

pub struct Catalog {
    conn: Connection,
}

impl Catalog {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("{} oluşturulamadı", parent.display()))?;
        }
        let conn =
            Connection::open(path).with_context(|| format!("{} açılamadı", path.display()))?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS sources (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                title TEXT NOT NULL,
                kind TEXT NOT NULL,
                license TEXT NOT NULL,
                attribution TEXT NOT NULL,
                fetched_at TEXT
            );

            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                lang TEXT NOT NULL,
                url TEXT,
                license TEXT NOT NULL,
                attribution TEXT NOT NULL,
                path TEXT NOT NULL,
                mime TEXT NOT NULL,
                hash TEXT NOT NULL,
                fetched_at TEXT NOT NULL,
                revision TEXT,
                metadata_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS documents_kind ON documents(kind);
            CREATE INDEX IF NOT EXISTS documents_source ON documents(source_id);

            CREATE TABLE IF NOT EXISTS fetch_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                status TEXT NOT NULL,
                items INTEGER NOT NULL DEFAULT 0,
                bytes INTEGER NOT NULL DEFAULT 0,
                error TEXT
            );

            CREATE TABLE IF NOT EXISTS web_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                started_at TEXT,
                error TEXT
            );
            "#,
        )
        .context("katalog şeması kurulamadı")?;
        Ok(Self { conn })
    }

    pub fn upsert_source(&self, source: &Source) -> Result<()> {
        self.conn
            .execute(
                r#"
            INSERT INTO sources(id, category, title, kind, license, attribution, fetched_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                category = excluded.category,
                title = excluded.title,
                kind = excluded.kind,
                license = excluded.license,
                attribution = excluded.attribution,
                fetched_at = excluded.fetched_at
            "#,
                params![
                    source.id,
                    source.category,
                    source.title,
                    source.kind,
                    source.license,
                    source.attribution,
                    now_rfc3339(),
                ],
            )
            .context("kaynak yazılamadı")?;
        Ok(())
    }

    pub fn upsert_document(&self, doc: &DocumentRecord) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO documents(
                    id, source_id, kind, title, lang, url, license, attribution,
                    path, mime, hash, fetched_at, revision, metadata_json
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                ON CONFLICT(id) DO UPDATE SET
                    source_id = excluded.source_id,
                    kind = excluded.kind,
                    title = excluded.title,
                    lang = excluded.lang,
                    url = excluded.url,
                    license = excluded.license,
                    attribution = excluded.attribution,
                    path = excluded.path,
                    mime = excluded.mime,
                    hash = excluded.hash,
                    fetched_at = excluded.fetched_at,
                    revision = excluded.revision,
                    metadata_json = excluded.metadata_json
                "#,
                params![
                    doc.id,
                    doc.source_id,
                    doc.kind,
                    doc.title,
                    doc.lang,
                    doc.url,
                    doc.license,
                    doc.attribution,
                    doc.path,
                    doc.mime,
                    doc.hash,
                    doc.fetched_at,
                    doc.revision,
                    doc.metadata_json,
                ],
            )
            .context("belge yazılamadı")?;
        Ok(())
    }

    pub fn document(&self, id: &str) -> Result<Option<DocumentRecord>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, source_id, kind, title, lang, url, license, attribution,
                   path, mime, hash, fetched_at, revision, metadata_json
            FROM documents WHERE id = ?1
            "#,
        )?;
        statement
            .query_row(params![id], row_to_document)
            .optional()
            .context("belge okunamadı")
    }

    pub fn documents(&self) -> Result<Vec<DocumentRecord>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, source_id, kind, title, lang, url, license, attribution,
                   path, mime, hash, fetched_at, revision, metadata_json
            FROM documents ORDER BY kind, title COLLATE NOCASE
            "#,
        )?;
        let rows = statement.query_map([], row_to_document)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("belgeler okunamadı")
    }

    pub fn documents_by_kind(&self, kind: &str) -> Result<Vec<DocumentRecord>> {
        let mut statement = self.conn.prepare(
            r#"
            SELECT id, source_id, kind, title, lang, url, license, attribution,
                   path, mime, hash, fetched_at, revision, metadata_json
            FROM documents WHERE kind = ?1 ORDER BY title COLLATE NOCASE
            "#,
        )?;
        let rows = statement.query_map(params![kind], row_to_document)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("belgeler okunamadı")
    }

    pub fn counts(&self) -> Result<Vec<(String, i64)>> {
        let mut statement = self
            .conn
            .prepare("SELECT kind, COUNT(*) FROM documents GROUP BY kind ORDER BY kind")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("sayımlar okunamadı")
    }

    pub fn total(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .context("belge sayısı okunamadı")
    }

    pub fn enqueue_url(&self, url: &str) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO web_queue(url, status, created_at) VALUES (?1, 'pending', ?2)",
                params![url, now_rfc3339()],
            )
            .context("URL kuyruğa yazılamadı")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn queue_len(&self) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM web_queue WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .context("kuyruk okunamadı")
    }

    pub fn pending_urls(&self, limit: usize) -> Result<Vec<(i64, String)>> {
        let mut statement = self.conn.prepare(
            "SELECT id, url FROM web_queue WHERE status = 'pending' ORDER BY id LIMIT ?1",
        )?;
        let rows =
            statement.query_map(params![limit as i64], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("kuyruk okunamadı")
    }

    pub fn mark_queue(&self, id: i64, status: &str, error: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE web_queue SET status = ?1, started_at = ?2, error = ?3 WHERE id = ?4",
            params![status, now_rfc3339(), error, id],
        )?;
        Ok(())
    }

    pub fn record_run(
        &self,
        source_id: &str,
        started_at: &str,
        status: &str,
        items: i64,
        bytes: i64,
        error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO fetch_runs(source_id, started_at, finished_at, status, items, bytes, error)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                source_id,
                started_at,
                now_rfc3339(),
                status,
                items,
                bytes,
                error
            ],
        )?;
        Ok(())
    }
}

fn row_to_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentRecord> {
    Ok(DocumentRecord {
        id: row.get(0)?,
        source_id: row.get(1)?,
        kind: row.get(2)?,
        title: row.get(3)?,
        lang: row.get(4)?,
        url: row.get(5)?,
        license: row.get(6)?,
        attribution: row.get(7)?,
        path: row.get(8)?,
        mime: row.get(9)?,
        hash: row.get(10)?,
        fetched_at: row.get(11)?,
        revision: row.get(12)?,
        metadata_json: row.get(13)?,
    })
}

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

use crate::db;
use crate::extract::extract_wiki_slugs;
use crate::search::search_fts;
use crate::types::{LinkRow, PageRow, SearchHit};

pub struct BrainEngine {
    db_path: PathBuf,
}

impl BrainEngine {
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        db::open(&db_path)?;
        Ok(Self { db_path })
    }

    pub fn default_home() -> Result<PathBuf> {
        let home = dirs_fallback();
        Ok(home.join(".rs_gbrain").join("brain.db"))
    }

    pub fn open_default() -> Result<Self> {
        let p = Self::default_home()?;
        Self::open(p)
    }

    fn conn(&self) -> Result<Connection> {
        db::open(&self.db_path)
    }

    pub fn put_page(
        &self,
        slug: &str,
        title: &str,
        page_type: &str,
        body: &str,
        source: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO pages (slug, title, page_type, body, source, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(slug) DO UPDATE SET
                title = excluded.title,
                page_type = excluded.page_type,
                body = excluded.body,
                source = excluded.source,
                updated_at = excluded.updated_at
            "#,
            rusqlite::params![slug, title, page_type, body, source, now],
        )?;
        self.reindex_links(&conn, slug, body)?;
        Ok(())
    }

    fn reindex_links(&self, conn: &Connection, from_slug: &str, body: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM links WHERE from_slug = ?1",
            rusqlite::params![from_slug],
        )?;
        let now = Utc::now().to_rfc3339();
        for to in extract_wiki_slugs(body) {
            let to = to.trim_matches('/');
            if to.is_empty() || to == from_slug {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO links (from_slug, to_slug, rel, created_at) VALUES (?1, ?2, 'links_to', ?3)",
                rusqlite::params![from_slug, to, now],
            )?;
        }
        Ok(())
    }

    pub fn get_page(&self, slug: &str) -> Result<Option<PageRow>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT slug, title, page_type, body, source, updated_at FROM pages WHERE slug = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![slug])?;
        if let Some(row) = rows.next()? {
            let updated_at: String = row.get(5)?;
            let dt = chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            return Ok(Some(PageRow {
                slug: row.get(0)?,
                title: row.get(1)?,
                page_type: row.get(2)?,
                body: row.get(3)?,
                source: row.get(4)?,
                updated_at: dt,
            }));
        }
        Ok(None)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        let conn = self.conn()?;
        search_fts(&conn, query, limit)
    }

    pub fn neighbors(&self, slug: &str, limit: usize) -> Result<Vec<LinkRow>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT from_slug, to_slug, rel FROM links
            WHERE from_slug = ?1 OR to_slug = ?1
            LIMIT ?2
            "#,
        )?;
        let rows = stmt.query_map(rusqlite::params![slug, limit as i64], |row| {
            Ok(LinkRow {
                from_slug: row.get(0)?,
                to_slug: row.get(1)?,
                rel: row.get(2)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn stats(&self) -> Result<(usize, usize)> {
        let conn = self.conn()?;
        let pages: i64 = conn.query_row("SELECT COUNT(*) FROM pages", [], |r| r.get(0))?;
        let links: i64 = conn.query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0))?;
        Ok((pages as usize, links as usize))
    }

    pub fn import_markdown_dir(&self, root: &Path) -> Result<usize> {
        let mut n = 0usize;
        if !root.is_dir() {
            anyhow::bail!("not a directory: {}", root.display());
        }
        for path in walkdir_light(root)? {
            if path.extension().and_then(|e: &std::ffi::OsStr| e.to_str()) != Some("md") {
                continue;
            }
            let body = std::fs::read_to_string(&path)?;
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let slug = rel.trim_end_matches(".md").to_string();
            let title = slug.rsplit('/').next().unwrap_or(&slug).to_string();
            let page_type = slug.split('/').next().unwrap_or("note").to_string();
            self.put_page(&slug, &title, &page_type, &body, "import")?;
            n += 1;
        }
        Ok(n)
    }
}

fn walkdir_light(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in
            std::fs::read_dir(&dir).with_context(|| format!("read_dir {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    Ok(out)
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

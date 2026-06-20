use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::db;
use crate::embed::{f32_to_bytes, Embedder, HashEmbedder};
use crate::extract::extract_wiki_slugs;
use crate::hybrid::{hybrid_search, HybridConfig};
use crate::search::search_fts;
use crate::typed_edges::infer_typed_edges;
use crate::types::{
    BrainStats, BriefState, GraphQueryResult, LinkRow, OpenLoop, PageListItem, PageRow, SearchHit,
};

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
        Self::open(Self::default_home()?)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
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
            INSERT INTO pages (slug, title, page_type, body, source, deleted, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
            ON CONFLICT(slug) DO UPDATE SET
                title = excluded.title,
                page_type = excluded.page_type,
                body = excluded.body,
                source = excluded.source,
                deleted = 0,
                updated_at = excluded.updated_at
            "#,
            rusqlite::params![slug, title, page_type, body, source, now],
        )?;
        self.reindex_links(&conn, slug, page_type, body)?;
        let embedder = HashEmbedder;
        self.reindex_page_vectors(&conn, slug, body, &embedder)?;
        Ok(())
    }

    pub fn put_page_with_embedder(
        &self,
        slug: &str,
        title: &str,
        page_type: &str,
        body: &str,
        source: &str,
        embedder: Option<&dyn Embedder>,
    ) -> Result<()> {
        self.put_page(slug, title, page_type, body, source)?;
        if let Some(e) = embedder {
            let conn = self.conn()?;
            self.reindex_page_vectors(&conn, slug, body, e)?;
        }
        Ok(())
    }

    pub fn delete_page(&self, slug: &str) -> Result<bool> {
        let conn = self.conn()?;
        let n = conn.execute(
            "UPDATE pages SET deleted = 1, updated_at = ?2 WHERE slug = ?1 AND deleted = 0",
            rusqlite::params![slug, Utc::now().to_rfc3339()],
        )?;
        Ok(n > 0)
    }

    pub fn add_link(&self, from: &str, to: &str, rel: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO links (from_slug, to_slug, rel, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![from, to, rel, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    fn reindex_links(
        &self,
        conn: &Connection,
        from_slug: &str,
        page_type: &str,
        body: &str,
    ) -> Result<()> {
        conn.execute(
            "DELETE FROM links WHERE from_slug = ?1",
            rusqlite::params![from_slug],
        )?;
        let wiki = extract_wiki_slugs(body);
        let edges = infer_typed_edges(from_slug, page_type, body, &wiki);
        let now = Utc::now().to_rfc3339();
        for edge in edges {
            conn.execute(
                "INSERT OR IGNORE INTO links (from_slug, to_slug, rel, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![from_slug, edge.to_slug, edge.rel, now],
            )?;
        }
        Ok(())
    }

    fn reindex_page_vectors(
        &self,
        conn: &Connection,
        slug: &str,
        body: &str,
        embedder: &dyn Embedder,
    ) -> Result<()> {
        conn.execute("DELETE FROM chunk_vectors WHERE slug = ?1", [slug])?;
        let chunks = chunk_text(body, 800);
        if chunks.is_empty() {
            return Ok(());
        }
        let refs: Vec<&str> = chunks.iter().map(|s| s.as_str()).collect();
        let vectors = embedder.embed(&refs)?;
        let now = Utc::now().to_rfc3339();
        for (i, (text, vec)) in chunks.iter().zip(vectors.iter()).enumerate() {
            let hash = format!("{:x}", md5_hash(text));
            conn.execute(
                "INSERT INTO chunk_vectors (slug, chunk_index, dim, vector, text_hash, updated_at) VALUES (?1,?2,?3,?4,?5,?6)",
                rusqlite::params![slug, i as i64, vec.len() as i64, f32_to_bytes(vec), hash, now],
            )?;
        }
        Ok(())
    }

    pub fn get_page(&self, slug: &str) -> Result<Option<PageRow>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT slug, title, page_type, body, source, updated_at FROM pages WHERE slug = ?1 AND deleted = 0",
        )?;
        let mut rows = stmt.query(rusqlite::params![slug])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row_to_page(row)?));
        }
        Ok(None)
    }

    pub fn list_pages(&self, prefix: Option<&str>, limit: usize) -> Result<Vec<PageListItem>> {
        let conn = self.conn()?;
        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<PageListItem> {
            Ok(PageListItem {
                slug: row.get(0)?,
                title: row.get(1)?,
                page_type: row.get(2)?,
                updated_at: row.get(3)?,
            })
        };
        if let Some(p) = prefix {
            let like = format!("{p}%");
            let mut stmt = conn.prepare(
                "SELECT slug, title, page_type, updated_at FROM pages WHERE deleted = 0 AND slug LIKE ?1 ORDER BY updated_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(rusqlite::params![like, limit as i64], map_row)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        } else {
            let mut stmt = conn.prepare(
                "SELECT slug, title, page_type, updated_at FROM pages WHERE deleted = 0 ORDER BY updated_at DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(rusqlite::params![limit as i64], map_row)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        self.search_with_graph_hint(query, limit)
    }

    /// Hybrid search; if query mentions a known `people/` or `companies/` slug, boost graph neighbors.
    pub fn search_with_graph_hint(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        let anchor = infer_search_anchor(self, query)?;
        self.hybrid_search(query, limit, anchor.as_deref())
    }

    pub fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
        graph_anchor: Option<&str>,
    ) -> Result<Vec<SearchHit>> {
        let embedder = HashEmbedder;
        hybrid_search(
            &self.conn()?,
            &embedder,
            query,
            limit,
            graph_anchor,
            &HybridConfig::default(),
        )
    }

    pub fn search_fts_only(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        search_fts(&self.conn()?, query, limit)
    }

    pub fn neighbors(&self, slug: &str, limit: usize) -> Result<Vec<LinkRow>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT from_slug, to_slug, rel FROM links WHERE from_slug = ?1 OR to_slug = ?1 LIMIT ?2",
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

    pub fn graph_query_filtered(
        &self,
        anchor: &str,
        depth: usize,
        rel_filter: Option<&str>,
    ) -> Result<GraphQueryResult> {
        let g = self.graph_query(anchor, depth)?;
        if let Some(rel) = rel_filter {
            let edges: Vec<LinkRow> = g.edges.into_iter().filter(|e| e.rel == rel).collect();
            let mut nodes: HashSet<String> = HashSet::new();
            nodes.insert(anchor.to_string());
            for e in &edges {
                nodes.insert(e.from_slug.clone());
                nodes.insert(e.to_slug.clone());
            }
            return Ok(GraphQueryResult {
                anchor: g.anchor,
                nodes: nodes.into_iter().collect(),
                edges,
            });
        }
        Ok(g)
    }

    pub fn graph_query(&self, anchor: &str, depth: usize) -> Result<GraphQueryResult> {
        let mut nodes = HashSet::new();
        let mut edges = Vec::new();
        let mut frontier = vec![anchor.to_string()];
        nodes.insert(anchor.to_string());
        for _ in 0..depth.max(1) {
            let mut next = Vec::new();
            for slug in &frontier {
                for link in self.neighbors(slug, 50)? {
                    edges.push(link.clone());
                    if nodes.insert(link.to_slug.clone()) {
                        next.push(link.to_slug.clone());
                    }
                    if nodes.insert(link.from_slug.clone()) {
                        next.push(link.from_slug.clone());
                    }
                }
            }
            frontier = next;
        }
        Ok(GraphQueryResult {
            anchor: anchor.to_string(),
            nodes: nodes.into_iter().collect(),
            edges,
        })
    }

    pub fn add_tag(&self, slug: &str, tag: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO tags (slug, tag, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![slug, tag, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_tags(&self, slug: &str) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT tag FROM tags WHERE slug = ?1")?;
        let rows = stmt.query_map(rusqlite::params![slug], |r| r.get(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn set_brief_loops(&self, lines: &[String]) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM open_loops", [])?;
        let now = Utc::now().to_rfc3339();
        for t in lines {
            conn.execute(
                "INSERT INTO open_loops (text, status, created_at) VALUES (?1, 'open', ?2)",
                rusqlite::params![t, now],
            )?;
        }
        Ok(())
    }

    pub fn set_brief_time_contexts(&self, lines: &[String]) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM time_contexts", [])?;
        let now = Utc::now().to_rfc3339();
        for t in lines {
            conn.execute(
                "INSERT INTO time_contexts (text, created_at) VALUES (?1, ?2)",
                rusqlite::params![t, now],
            )?;
        }
        Ok(())
    }

    pub fn load_brief(&self) -> Result<BriefState> {
        let conn = self.conn()?;
        let loops: Vec<String> = conn
            .prepare("SELECT text FROM open_loops WHERE status = 'open'")?
            .query_map([], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        let contexts: Vec<String> = conn
            .prepare("SELECT text FROM time_contexts")?
            .query_map([], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(BriefState {
            open_loops: loops,
            time_contexts: contexts,
        })
    }

    pub fn list_open_loops(&self) -> Result<Vec<OpenLoop>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT id, text FROM open_loops WHERE status = 'open'")?;
        let rows = stmt.query_map([], |r| {
            Ok(OpenLoop {
                id: r.get(0)?,
                text: r.get(1)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn append_timeline(&self, slug: &str, entry: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO timeline (slug, entry, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![slug, entry, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_timeline(&self, slug: &str, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt =
            conn.prepare("SELECT entry FROM timeline WHERE slug = ?1 ORDER BY id DESC LIMIT ?2")?;
        let rows = stmt.query_map(rusqlite::params![slug, limit as i64], |r| r.get(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn brain_stats(&self) -> Result<BrainStats> {
        let conn = self.conn()?;
        let page_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM pages WHERE deleted = 0", [], |r| {
                r.get(0)
            })?;
        let link_count: i64 = conn.query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0))?;
        let tag_count: i64 = conn.query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))?;
        let open_loop_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM open_loops", [], |r| r.get(0))?;
        let fact_count: i64 = conn.query_row("SELECT COUNT(*) FROM facts", [], |r| r.get(0))?;
        Ok(BrainStats {
            page_count: page_count as usize,
            link_count: link_count as usize,
            tag_count: tag_count as usize,
            open_loop_count: open_loop_count as usize,
            fact_count: fact_count as usize,
        })
    }

    pub fn stats(&self) -> Result<(usize, usize)> {
        let s = self.brain_stats()?;
        Ok((s.page_count, s.link_count))
    }

    pub fn list_pages_without_inbound_links(&self, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT p.slug FROM pages p
            WHERE p.deleted = 0
              AND NOT EXISTS (SELECT 1 FROM links l WHERE l.to_slug = p.slug)
            LIMIT ?1
            "#,
        )?;
        let rows = stmt.query_map([limit as i64], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn reindex_all_vectors(&self, embedder: &dyn Embedder) -> Result<usize> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT slug, body FROM pages WHERE deleted = 0")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut n = 0usize;
        for row in rows.flatten() {
            self.reindex_page_vectors(&conn, &row.0, &row.1, embedder)?;
            n += 1;
        }
        Ok(n)
    }

    pub fn close_stale_open_loops(&self, max_age_days: i64) -> Result<usize> {
        let conn = self.conn()?;
        let cutoff = (Utc::now() - chrono::Duration::days(max_age_days)).to_rfc3339();
        let n = conn.execute(
            "UPDATE open_loops SET status = 'dreamed' WHERE status = 'open' AND created_at < ?1",
            [cutoff],
        )?;
        Ok(n)
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

fn row_to_page(row: &rusqlite::Row<'_>) -> Result<PageRow> {
    let updated_at: String = row.get(5)?;
    let dt = chrono::DateTime::parse_from_rfc3339(&updated_at)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    Ok(PageRow {
        slug: row.get(0)?,
        title: row.get(1)?,
        page_type: row.get(2)?,
        body: row.get(3)?,
        source: row.get(4)?,
        updated_at: dt,
    })
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

fn chunk_text(body: &str, max_len: usize) -> Vec<String> {
    if body.len() <= max_len {
        return vec![body.to_string()];
    }
    body.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| p.chars().take(max_len).collect::<String>())
        .collect()
}

fn md5_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn infer_search_anchor(engine: &BrainEngine, query: &str) -> Result<Option<String>> {
    let q = query.to_ascii_lowercase();
    let conn = engine.conn()?;
    let mut stmt = conn.prepare(
        "SELECT slug FROM pages WHERE deleted = 0 AND (slug LIKE 'people/%' OR slug LIKE 'companies/%') LIMIT 200",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut best: Option<(usize, String)> = None;
    for slug in rows.flatten() {
        let token = slug
            .rsplit('/')
            .next()
            .unwrap_or(&slug)
            .replace('-', " ");
        if token.len() < 3 {
            continue;
        }
        if q.contains(&token) || q.contains(slug.as_str()) {
            let score = token.len();
            if best.as_ref().map(|b| score > b.0).unwrap_or(true) {
                best = Some((score, slug));
            }
        }
    }
    Ok(best.map(|(_, s)| s))
}

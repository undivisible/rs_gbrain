//! Hybrid retrieval: BM25 (FTS) + vector + graph proximity boost.

use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

use crate::embed::{bytes_to_f32, cosine, Embedder};
use crate::search::search_fts;
use crate::types::SearchHit;

pub struct HybridConfig {
    pub fts_weight: f32,
    pub vec_weight: f32,
    pub graph_weight: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            fts_weight: 0.45,
            vec_weight: 0.45,
            graph_weight: 0.10,
        }
    }
}

pub fn hybrid_search(
    conn: &Connection,
    tenant_id: &str,
    embedder: &dyn Embedder,
    query: &str,
    limit: usize,
    graph_anchor: Option<&str>,
    cfg: &HybridConfig,
) -> Result<Vec<SearchHit>> {
    let fts_hits = search_fts(conn, tenant_id, query, limit.saturating_mul(3))?;
    let mut scores: HashMap<String, (f32, SearchHit)> = HashMap::new();

    let max_fts = fts_hits.iter().map(|h| h.score).fold(0.01f32, f32::max);

    for h in fts_hits {
        let norm = h.score / max_fts;
        scores.insert(h.slug.clone(), (norm * cfg.fts_weight, h));
    }

    let q_vec = embedder
        .embed(&[query])?
        .into_iter()
        .next()
        .unwrap_or_default();
    if !q_vec.is_empty() {
        let mut stmt = conn.prepare(
            "SELECT slug, chunk_index, vector FROM chunk_vectors WHERE tenant_id = ?1 AND dim = ?2",
        )?;
        let dim = q_vec.len() as i64;
        let rows = stmt.query_map(rusqlite::params![tenant_id, dim], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        let mut best_vec: HashMap<String, f32> = HashMap::new();
        for r in rows.flatten() {
            let (slug, _idx, bytes) = r;
            let v = bytes_to_f32(&bytes);
            let sim = cosine(&q_vec, &v);
            best_vec
                .entry(slug)
                .and_modify(|m| *m = m.max(sim))
                .or_insert(sim);
        }
        for (slug, sim) in best_vec {
            let entry = scores.entry(slug.clone()).or_insert_with(|| {
                (
                    0.0,
                    SearchHit {
                        slug: slug.clone(),
                        title: slug.clone(),
                        snippet: String::new(),
                        score: 0.0,
                    },
                )
            });
            entry.0 += sim * cfg.vec_weight;
        }
    }

    if let Some(anchor) = graph_anchor {
        let neighbors = graph_slugs(conn, tenant_id, anchor, 2)?;
        for slug in neighbors {
            if slug == anchor {
                continue;
            }
            let entry = scores.entry(slug.clone()).or_insert_with(|| {
                (
                    0.0,
                    SearchHit {
                        slug: slug.clone(),
                        title: slug.clone(),
                        snippet: String::new(),
                        score: 0.0,
                    },
                )
            });
            entry.0 += cfg.graph_weight;
        }
    }

    let mut ranked: Vec<(String, f32, SearchHit)> = scores
        .into_iter()
        .map(|(slug, (s, mut h))| {
            h.score = s;
            if h.snippet.is_empty() {
                if let Ok(sn) = snippet_for_slug(conn, tenant_id, &slug) {
                    h.snippet = sn;
                }
            }
            if h.title == slug {
                if let Ok(t) = title_for_slug(conn, tenant_id, &slug) {
                    h.title = t;
                }
            }
            (slug, s, h)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(ranked.into_iter().take(limit).map(|(_, _, h)| h).collect())
}

fn graph_slugs(
    conn: &Connection,
    tenant_id: &str,
    anchor: &str,
    depth: usize,
) -> Result<Vec<String>> {
    let mut out = vec![anchor.to_string()];
    let mut frontier = vec![anchor.to_string()];
    for _ in 0..depth {
        let mut next = Vec::new();
        for slug in &frontier {
            let mut stmt = conn.prepare(
                "SELECT to_slug FROM links WHERE tenant_id = ?1 AND from_slug = ?2 UNION SELECT from_slug FROM links WHERE tenant_id = ?1 AND to_slug = ?2",
            )?;
            let rows = stmt.query_map(rusqlite::params![tenant_id, slug], |r| {
                r.get::<_, String>(0)
            })?;
            for s in rows.flatten() {
                if !out.contains(&s) {
                    out.push(s.clone());
                    next.push(s);
                }
            }
        }
        frontier = next;
    }
    Ok(out)
}

fn snippet_for_slug(conn: &Connection, tenant_id: &str, slug: &str) -> Result<String> {
    let mut stmt = conn.prepare(
        "SELECT substr(body, 1, 120) FROM pages WHERE tenant_id = ?1 AND slug = ?2 AND deleted = 0",
    )?;
    let mut rows = stmt.query(rusqlite::params![tenant_id, slug])?;
    if let Some(row) = rows.next()? {
        return Ok(row.get(0)?);
    }
    Ok(String::new())
}

fn title_for_slug(conn: &Connection, tenant_id: &str, slug: &str) -> Result<String> {
    let mut stmt =
        conn.prepare("SELECT title FROM pages WHERE tenant_id = ?1 AND slug = ?2 AND deleted = 0")?;
    let mut rows = stmt.query(rusqlite::params![tenant_id, slug])?;
    if let Some(row) = rows.next()? {
        return Ok(row.get(0)?);
    }
    Ok(slug.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::embed::HashEmbedder;
    use tempfile::tempdir;

    #[test]
    fn hybrid_prefers_fts_match() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("b.db");
        let conn = db::open(&path).unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO pages (tenant_id, slug, title, page_type, body, source, deleted, updated_at) VALUES ('default',?1,?2,?3,?4,?5,0,?6)",
            rusqlite::params!["people/alice", "Alice", "person", "graph retrieval expert", "t", now],
        )
        .unwrap();
        let hits = hybrid_search(
            &conn,
            "default",
            &HashEmbedder,
            "graph retrieval",
            5,
            None,
            &HybridConfig::default(),
        )
        .unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].slug, "people/alice");
    }
}

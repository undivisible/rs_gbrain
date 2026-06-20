use anyhow::Result;
use rusqlite::Connection;

use crate::types::SearchHit;

pub fn search_fts(
    conn: &Connection,
    tenant_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let fts_q = query_to_fts(q);
    let mut stmt = conn.prepare(
        r#"
        SELECT p.slug, p.title, snippet(pages_fts, 2, '«', '»', '…', 24) AS snip,
               bm25(pages_fts) AS rank
        FROM pages_fts
        JOIN pages p ON p.rowid = pages_fts.rowid
        WHERE pages_fts MATCH ? AND p.tenant_id = ? AND p.deleted = 0
        ORDER BY rank
        LIMIT ?
        "#,
    )?;
    let rows = stmt.query_map(rusqlite::params![fts_q, tenant_id, limit as i64], |row| {
        let slug: String = row.get(0)?;
        let title: String = row.get(1)?;
        let snippet: String = row.get(2)?;
        let rank: f64 = row.get(3)?;
        Ok(SearchHit {
            slug,
            title,
            snippet,
            score: (-rank as f32).max(0.0),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn query_to_fts(q: &str) -> String {
    q.split_whitespace()
        .map(|w| format!("\"{}\"", w.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_tokenize() {
        assert_eq!(query_to_fts("hello world"), "\"hello\" AND \"world\"");
    }
}

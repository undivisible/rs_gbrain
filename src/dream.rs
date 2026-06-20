//! Nightly dream cycle: consolidate open loops, link orphans, refresh vectors.

use anyhow::Result;
use chrono::Utc;

use crate::embed::{Embedder, HashEmbedder};
use crate::engine::BrainEngine;

#[derive(serde::Serialize)]
pub struct DreamReport {
    pub hypothesis_pages: usize,
    pub links_added: usize,
    pub chunks_indexed: usize,
    pub loops_closed: usize,
}

pub fn run_nightly_cycle(engine: &BrainEngine, embedder: &dyn Embedder) -> Result<DreamReport> {
    let mut report = DreamReport {
        hypothesis_pages: 0,
        links_added: 0,
        chunks_indexed: 0,
        loops_closed: 0,
    };

    let loops = engine.list_open_loops()?;
    for line in loops.iter().take(12) {
        let slug = format!("dream/loop-{}", line.id);
        let body = format!(
            "Dream hypothesis ({}): {}\n\nNext: search brain, file missing pages, link [[inbox/dream]].\n",
            Utc::now().format("%Y-%m-%d"),
            line.text
        );
        engine.put_page_with_embedder(&slug, "Dream", "dream", &body, "dream", Some(embedder))?;
        engine.append_timeline("meta/dream", &format!("hypothesis {slug}"))?;
        report.hypothesis_pages += 1;
    }

    let orphans = engine.list_pages_without_inbound_links(20)?;
    for slug in orphans {
        engine.add_link("meta/dream", &slug, "related_to")?;
        report.links_added += 1;
    }

    report.chunks_indexed = engine.reindex_all_vectors(embedder)?;
    report.loops_closed = engine.close_stale_open_loops(30)?;

    Ok(report)
}

pub fn run_dream_cycle(engine: &BrainEngine) -> Result<usize> {
    let embedder = HashEmbedder;
    Ok(run_nightly_cycle(engine, &embedder)?.hypothesis_pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn nightly_creates_hypothesis() {
        let dir = tempdir().unwrap();
        let e = BrainEngine::open(dir.path().join("b.db")).unwrap();
        e.set_brief_loops(&["Follow up with Alice on term sheet".to_string()])
            .unwrap();
        let r = run_nightly_cycle(&e, &HashEmbedder).unwrap();
        assert!(r.hypothesis_pages >= 1);
        assert!(e.get_page("dream/loop-1").unwrap().is_some());
    }
}

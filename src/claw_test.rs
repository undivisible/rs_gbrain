//! gbrain `claw-test` scripted phases (hermetic temp DB).

use crate::BrainEngine;
use anyhow::Result;
use std::env;

pub struct ClawTestReport {
    pub phases: Vec<&'static str>,
    pub ok: bool,
    pub message: String,
}

pub fn run_scripted() -> Result<ClawTestReport> {
    let tmp = tempfile::tempdir()?;
    let db = tmp.path().join("brain.db");
    env::set_var("RS_GBRAIN_DB", db.to_string_lossy().as_ref());
    let e = BrainEngine::open(&db)?;

    e.put_page(
        "people/alice",
        "Alice",
        "person",
        "Alice CTO at [[companies/acme]].",
        "claw-test",
    )?;
    e.put_page(
        "companies/acme",
        "Acme",
        "company",
        "Acme builds retrieval.",
        "claw-test",
    )?;
    e.import_markdown_dir(tmp.path()).ok();

    let hits = e.search("Alice", 5)?;
    if hits.is_empty() {
        anyhow::bail!("query phase: no hits");
    }
    let _ = crate::query::gather_context(&e, "What about Alice?", 5)?;

    let stats = e.brain_stats()?;
    if stats.page_count < 2 {
        anyhow::bail!("verify: expected >=2 pages");
    }

    Ok(ClawTestReport {
        phases: vec!["setup", "put", "search", "query", "verify"],
        ok: true,
        message: format!(
            "claw-test ok pages={} links={}",
            stats.page_count, stats.link_count
        ),
    })
}

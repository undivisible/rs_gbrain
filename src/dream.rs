//! Dream cycle stub — hypothesis pages from open loops (no LLM).

use anyhow::Result;
use chrono::Utc;

use crate::engine::BrainEngine;

pub fn run_dream_cycle(engine: &BrainEngine) -> Result<usize> {
    let loops = engine.list_open_loops()?;
    let mut n = 0usize;
    for line in loops.iter().take(8) {
        let slug = format!("dream/loop-{}", line.id);
        let body = format!(
            "Hypothesis (dream): explore «{}»\n\nRelated: [[{}]]",
            line.text,
            line.text.split_whitespace().next().unwrap_or("inbox")
        );
        engine.put_page(&slug, "Dream", "dream", &body, "dream")?;
        engine.append_timeline("dream/cycle", &format!("spawned {slug} from loop"))?;
        n += 1;
    }
    let _ = Utc::now();
    Ok(n)
}

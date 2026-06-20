//! `query` / `think` — gather hits + optional LLM synthesis (gbrain-shaped).

use anyhow::Result;

use crate::engine::BrainEngine;
use crate::types::QueryAnswer;

pub fn gather_context(engine: &BrainEngine, question: &str, limit: usize) -> Result<QueryAnswer> {
    gather_context_with_anchor(engine, question, limit, None)
}

pub fn gather_context_with_anchor(
    engine: &BrainEngine,
    question: &str,
    limit: usize,
    anchor: Option<&str>,
) -> Result<QueryAnswer> {
    let hits = if let Some(a) = anchor {
        engine.hybrid_search(question, limit, Some(a))?
    } else {
        engine.search_with_graph_hint(question, limit)?
    };
    let mut citations = hits.clone();
    if citations.is_empty() {
        let words: Vec<&str> = question
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();
        if let Some(w) = words.first() {
            citations = engine.search(w, limit)?;
        }
    }

    let mut answer_parts = Vec::new();
    for h in citations.iter().take(5) {
        if let Ok(Some(p)) = engine.get_page(&h.slug) {
            let summary = p.body.lines().next().unwrap_or(&p.body);
            let line = summary.chars().take(240).collect::<String>();
            answer_parts.push(format!("- **{}** ({}): {}", p.title, p.slug, line));
        }
    }

    let answer = if answer_parts.is_empty() {
        "No matching brain pages. Consider `put` or `import` first.".to_string()
    } else {
        format!(
            "From local brain ({} sources):\n\n{}",
            answer_parts.len(),
            answer_parts.join("\n")
        )
    };

    let mut gaps = Vec::new();
    if citations.is_empty() {
        gaps.push("Brain has no indexed content for this question.".to_string());
    } else {
        gaps.push(
            "Local rs_gbrain: bullets are retrieval context only — host LLM should synthesize prose (unlike upstream gbrain think).".to_string(),
        );
        if citations.len() < 3 {
            gaps.push("Few sources matched; brain may be missing recent email/Slack context.".to_string());
        }
    };

    Ok(QueryAnswer {
        answer,
        citations,
        gaps,
    })
}

pub fn format_query_markdown(q: &QueryAnswer) -> String {
    let mut out = q.answer.clone();
    if !q.gaps.is_empty() {
        out.push_str("\n\n**Gaps:**\n");
        for g in &q.gaps {
            out.push_str(&format!("- {g}\n"));
        }
    }
    if !q.citations.is_empty() {
        out.push_str("\n\n**Sources:**\n");
        for c in &q.citations {
            out.push_str(&format!("- [{}] {}\n", c.score, c.slug));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn query_empty_brain() {
        let dir = tempdir().unwrap();
        let e = BrainEngine::open(dir.path().join("b.db")).unwrap();
        let q = gather_context(&e, "Alice meeting", 5).unwrap();
        assert!(!q.gaps.is_empty());
    }
}

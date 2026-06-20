//! `query` / `think` — gather hits + optional LLM synthesis (gbrain-shaped).

use anyhow::Result;

use crate::engine::BrainEngine;
use crate::types::QueryAnswer;

pub fn gather_context(engine: &BrainEngine, question: &str, limit: usize) -> Result<QueryAnswer> {
    let hits = engine.search(question, limit)?;
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

    let gaps = if citations.is_empty() {
        vec!["Brain has no indexed content for this question.".to_string()]
    } else {
        vec![]
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

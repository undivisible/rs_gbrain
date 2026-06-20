//! Typed graph edges inferred on page write (gbrain-shaped rels).

use regex::Regex;
use std::sync::OnceLock;

pub const REL_LINKS_TO: &str = "links_to";
pub const REL_WORKS_AT: &str = "works_at";
pub const REL_REPORTS_TO: &str = "reports_to";
pub const REL_INVESTED_IN: &str = "invested_in";
pub const REL_ATTENDED: &str = "attended";
pub const REL_KNOWS: &str = "knows";
pub const REL_RELATED_TO: &str = "related_to";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedEdge {
    pub to_slug: String,
    pub rel: String,
}

fn typed_wiki_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([a-zA-Z0-9_./-]+)\|([a-z_]+)\]\]").unwrap())
}

fn reports_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)reports?\s+to\s+\[\[([a-zA-Z0-9_./-]+)\]\]").unwrap())
}

fn works_at_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)works?\s+at\s+\[\[([a-zA-Z0-9_./-]+)\]\]").unwrap())
}

fn invested_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)invested\s+in\s+\[\[([a-zA-Z0-9_./-]+)\]\]").unwrap())
}

fn attended_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)attended\s+\[\[([a-zA-Z0-9_./-]+)\]\]").unwrap())
}

pub fn infer_typed_edges(
    from_slug: &str,
    page_type: &str,
    body: &str,
    wiki_slugs: &[String],
) -> Vec<TypedEdge> {
    let mut edges = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let push = |edges: &mut Vec<TypedEdge>,
                seen: &mut std::collections::HashSet<(String, String)>,
                to: &str,
                rel: &str| {
        let to = to.trim_matches('/');
        if to.is_empty() || to == from_slug {
            return;
        }
        let key = (to.to_string(), rel.to_string());
        if seen.insert(key.clone()) {
            edges.push(TypedEdge {
                to_slug: key.0,
                rel: key.1,
            });
        }
    };

    for cap in typed_wiki_re().captures_iter(body) {
        if let (Some(slug), Some(rel)) = (cap.get(1), cap.get(2)) {
            push(&mut edges, &mut seen, slug.as_str(), rel.as_str());
        }
    }

    for cap in reports_re().captures_iter(body) {
        if let Some(m) = cap.get(1) {
            push(&mut edges, &mut seen, m.as_str(), REL_REPORTS_TO);
        }
    }
    for cap in works_at_re().captures_iter(body) {
        if let Some(m) = cap.get(1) {
            push(&mut edges, &mut seen, m.as_str(), REL_WORKS_AT);
        }
    }
    for cap in invested_re().captures_iter(body) {
        if let Some(m) = cap.get(1) {
            push(&mut edges, &mut seen, m.as_str(), REL_INVESTED_IN);
        }
    }
    for cap in attended_re().captures_iter(body) {
        if let Some(m) = cap.get(1) {
            push(&mut edges, &mut seen, m.as_str(), REL_ATTENDED);
        }
    }

    let pt = page_type.to_ascii_lowercase();
    for slug in wiki_slugs {
        if pt == "person" && slug.starts_with("companies/") {
            push(&mut edges, &mut seen, slug, REL_WORKS_AT);
        } else if pt == "investor" && slug.starts_with("companies/") {
            push(&mut edges, &mut seen, slug, REL_INVESTED_IN);
        } else if pt == "event" && slug.starts_with("people/") {
            push(&mut edges, &mut seen, slug, REL_ATTENDED);
        } else {
            push(&mut edges, &mut seen, slug, REL_LINKS_TO);
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::extract_wiki_slugs;

    #[test]
    fn person_company_works_at() {
        let body = "Alice CTO. See [[companies/acme]].";
        let w = extract_wiki_slugs(body);
        let e = infer_typed_edges("people/alice", "person", body, &w);
        assert!(e
            .iter()
            .any(|x| x.rel == REL_WORKS_AT && x.to_slug == "companies/acme"));
    }

    #[test]
    fn explicit_rel_pipe() {
        let body = "Knows [[people/bob|knows]].";
        let w = extract_wiki_slugs(body);
        let e = infer_typed_edges("people/alice", "person", body, &w);
        assert!(e
            .iter()
            .any(|x| x.rel == "knows" && x.to_slug == "people/bob"));
    }

    #[test]
    fn reports_to_phrase() {
        let body = "Reports to [[people/ceo]].";
        let w = extract_wiki_slugs(body);
        let e = infer_typed_edges("people/alice", "person", body, &w);
        assert!(e.iter().any(|x| x.rel == REL_REPORTS_TO));
    }
}

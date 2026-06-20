//! Link extraction: wikilinks + entity markdown links (gbrain-shaped dirs).

use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

const ENTITY_DIRS: &str =
    "people|companies|meetings|concepts|deal|civic|project|projects|source|media|yc|tech|finance|personal|entities";

fn wiki_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([a-zA-Z0-9_./-]+)(?:\|[^\]]+)?\]\]").unwrap())
}

fn entity_md_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"\[[^\]]+\]\((?:\.\./)*(?:{ENTITY_DIRS})/[^)\s]+?\)"
        ))
        .unwrap()
    })
}

fn entity_slug_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"\[[^\]]+\]\((?:\.\./)*(({ENTITY_DIRS})/[^)\s#]+?)(?:\.md)?\)"
        ))
        .unwrap()
    })
}

pub fn extract_wiki_slugs(body: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for c in wiki_re().captures_iter(body) {
        if let Some(m) = c.get(1) {
            let s = m.as_str().trim_matches('/').to_string();
            if seen.insert(s.clone()) {
                out.push(s);
            }
        }
    }
    if entity_md_re().is_match(body) {
        for c in entity_slug_re().captures_iter(body) {
            if let Some(m) = c.get(1) {
                let s = m
                    .as_str()
                    .trim_end_matches(".md")
                    .trim_matches('/')
                    .to_string();
                if seen.insert(s.clone()) {
                    out.push(s);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiki_links() {
        let s = "See [[people/alice]] and [[companies/acme]].";
        let v = extract_wiki_slugs(s);
        assert_eq!(v.len(), 2);
        assert!(v.contains(&"people/alice".to_string()));
    }

    #[test]
    fn markdown_entity_links() {
        let s = "Met [Alice](../people/alice.md) at [Acme](companies/acme).";
        let v = extract_wiki_slugs(s);
        assert!(v.contains(&"people/alice".to_string()));
        assert!(v.contains(&"companies/acme".to_string()));
    }
}

//! Lightweight link extraction from markdown ([[slug]] and wikilinks).

use regex::Regex;
use std::sync::OnceLock;

fn wiki_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([a-zA-Z0-9_./-]+)\]\]").unwrap())
}

pub fn extract_wiki_slugs(body: &str) -> Vec<String> {
    wiki_re()
        .captures_iter(body)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
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
}

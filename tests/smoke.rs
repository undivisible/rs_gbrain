use rs_gbrain::BrainEngine;
use tempfile::tempdir;

#[test]
fn put_search_graph() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("brain.db");
    let e = BrainEngine::open(&db).unwrap();
    e.put_page(
        "people/alice",
        "Alice",
        "person",
        "CTO at [[companies/acme]].",
        "test",
    )
    .unwrap();
    e.put_page(
        "companies/acme",
        "Acme",
        "company",
        "Series B fintech.",
        "test",
    )
    .unwrap();
    let hits = e.search("Alice CTO", 5).unwrap();
    assert!(!hits.is_empty());
    let links = e.neighbors("people/alice", 10).unwrap();
    assert!(links.iter().any(|l| l.to_slug == "companies/acme"));
}

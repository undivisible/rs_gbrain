use gbrain::typed_edges::REL_WORKS_AT;
use gbrain::BrainEngine;
use tempfile::tempdir;

#[test]
fn typed_edge_on_put_and_graph_query() {
    let dir = tempdir().unwrap();
    let e = BrainEngine::open(dir.path().join("b.db")).unwrap();
    e.put_page(
        "people/alice",
        "Alice",
        "person",
        "CTO at [[companies/acme]].",
        "test",
    )
    .unwrap();
    e.put_page("companies/acme", "Acme", "company", "Builds RAG.", "test")
        .unwrap();
    let g = e
        .graph_query_filtered("people/alice", 2, Some(REL_WORKS_AT))
        .unwrap();
    assert!(g.edges.iter().any(|x| x.to_slug == "companies/acme"));
}

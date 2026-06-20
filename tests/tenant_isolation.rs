use rs_gbrain::BrainEngine;

#[test]
fn tenants_do_not_see_each_others_pages() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("b.db");
    let a = BrainEngine::open_scoped(&db, "alice").unwrap();
    let b = BrainEngine::open_scoped(&db, "bob").unwrap();
    a.put_page("people/x", "X", "person", "tenant-alice-secret-marker", "t")
        .unwrap();
    b.put_page("people/x", "X", "person", "tenant-bob-other", "t")
        .unwrap();
    assert!(a
        .search("tenant-alice-secret-marker", 5)
        .unwrap()
        .iter()
        .any(|h| h.slug == "people/x"));
    assert!(b
        .search_fts_only("tenant-alice-secret-marker", 5)
        .unwrap()
        .is_empty());
}

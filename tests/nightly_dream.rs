use rs_gbrain::{run_nightly_cycle, BrainEngine, HashEmbedder};
use tempfile::tempdir;

#[test]
fn nightly_reindexes_vectors() {
    let dir = tempdir().unwrap();
    let e = BrainEngine::open(dir.path().join("b.db")).unwrap();
    e.put_page("note/a", "A", "note", "semantic retrieval test body", "t")
        .unwrap();
    let r = run_nightly_cycle(&e, &HashEmbedder).unwrap();
    assert!(r.chunks_indexed >= 1);
}

//! Sync workspace markdown brief files into SQLite open_loops / time_contexts.

use anyhow::Result;
use std::path::Path;

use crate::BrainEngine;

pub fn sync_workspace_brief(workspace: &Path, engine: &BrainEngine) -> Result<()> {
    let loops = parse_bullet_file(&workspace.join("memory/open-loops.md"));
    let contexts = parse_bullet_file(&workspace.join("memory/time-contexts.md"));
    if !loops.is_empty() {
        engine.set_brief_loops(&loops)?;
    }
    if !contexts.is_empty() {
        engine.set_brief_time_contexts(&contexts)?;
    }
    Ok(())
}

fn parse_bullet_file(path: &Path) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("- ") && !l.contains("(none yet)"))
        .map(|l| l.trim_start_matches("- ").to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sync_loops_from_workspace() {
        let dir = tempdir().unwrap();
        let mem = dir.path().join("memory");
        std::fs::create_dir_all(&mem).unwrap();
        std::fs::write(
            mem.join("open-loops.md"),
            "# Open loops\n\n- Ship rs_gbrain\n",
        )
        .unwrap();
        let e = BrainEngine::open(dir.path().join("brain.db")).unwrap();
        sync_workspace_brief(dir.path(), &e).unwrap();
        let b = e.load_brief().unwrap();
        assert!(b.open_loops.iter().any(|x| x.contains("rs_gbrain")));
    }
}

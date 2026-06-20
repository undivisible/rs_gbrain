use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS pages (
            slug TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            page_type TEXT NOT NULL DEFAULT 'note',
            body TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'local',
            deleted INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS links (
            from_slug TEXT NOT NULL,
            to_slug TEXT NOT NULL,
            rel TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (from_slug, to_slug, rel)
        );

        CREATE TABLE IF NOT EXISTS tags (
            slug TEXT NOT NULL,
            tag TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (slug, tag)
        );

        CREATE TABLE IF NOT EXISTS open_loops (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'open',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS time_contexts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            until_date TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS facts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            subject_slug TEXT NOT NULL,
            predicate TEXT NOT NULL,
            object_text TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 0.8,
            source TEXT NOT NULL DEFAULT 'local',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS timeline (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL,
            entry TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chunk_vectors (
            slug TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            dim INTEGER NOT NULL,
            vector BLOB NOT NULL,
            text_hash TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (slug, chunk_index)
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
            slug,
            title,
            body,
            content='pages',
            content_rowid='rowid'
        );

        CREATE TRIGGER IF NOT EXISTS pages_ai AFTER INSERT ON pages BEGIN
            INSERT INTO pages_fts(rowid, slug, title, body)
            SELECT new.rowid, new.slug, new.title, new.body WHERE new.deleted = 0;
        END;

        CREATE TRIGGER IF NOT EXISTS pages_ad AFTER DELETE ON pages BEGIN
            INSERT INTO pages_fts(pages_fts, rowid, slug, title, body)
            VALUES ('delete', old.rowid, old.slug, old.title, old.body);
        END;

        CREATE TRIGGER IF NOT EXISTS pages_au AFTER UPDATE ON pages BEGIN
            INSERT INTO pages_fts(pages_fts, rowid, slug, title, body)
            VALUES ('delete', old.rowid, old.slug, old.title, old.body);
            INSERT INTO pages_fts(rowid, slug, title, body)
            SELECT new.rowid, new.slug, new.title, new.body WHERE new.deleted = 0;
        END;
        "#,
    )?;
    let _ = conn.execute(
        "ALTER TABLE pages ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0",
        [],
    );
    rebuild_fts_if_needed(conn)?;
    Ok(())
}

fn rebuild_fts_if_needed(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM pages_fts", [], |r| r.get(0))?;
    let pages: i64 = conn.query_row("SELECT COUNT(*) FROM pages WHERE deleted = 0", [], |r| {
        r.get(0)
    })?;
    if pages > 0 && count == 0 {
        conn.execute("INSERT INTO pages_fts(pages_fts) VALUES('rebuild')", [])?;
    }
    Ok(())
}

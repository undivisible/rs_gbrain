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
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS links (
            from_slug TEXT NOT NULL,
            to_slug TEXT NOT NULL,
            rel TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (from_slug, to_slug, rel)
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
            VALUES (new.rowid, new.slug, new.title, new.body);
        END;

        CREATE TRIGGER IF NOT EXISTS pages_ad AFTER DELETE ON pages BEGIN
            INSERT INTO pages_fts(pages_fts, rowid, slug, title, body)
            VALUES ('delete', old.rowid, old.slug, old.title, old.body);
        END;

        CREATE TRIGGER IF NOT EXISTS pages_au AFTER UPDATE ON pages BEGIN
            INSERT INTO pages_fts(pages_fts, rowid, slug, title, body)
            VALUES ('delete', old.rowid, old.slug, old.title, old.body);
            INSERT INTO pages_fts(rowid, slug, title, body)
            VALUES (new.rowid, new.slug, new.title, new.body);
        END;
        "#,
    )?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM pages_fts", [], |r| r.get(0))?;
    let pages: i64 = conn.query_row("SELECT COUNT(*) FROM pages", [], |r| r.get(0))?;
    if pages > 0 && count == 0 {
        conn.execute("INSERT INTO pages_fts(pages_fts) VALUES('rebuild')", [])?;
    }
    Ok(())
}

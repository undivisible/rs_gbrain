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
    let _ = conn.execute(
        "ALTER TABLE pages ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE links ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE tags ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE chunk_vectors ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default'",
        [],
    );
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS minion_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id TEXT NOT NULL DEFAULT 'default',
            name TEXT NOT NULL,
            payload TEXT,
            status TEXT NOT NULL DEFAULT 'waiting',
            created_at TEXT NOT NULL,
            started_at TEXT,
            finished_at TEXT,
            error TEXT
        );
        "#,
    )?;
    migrate_pages_tenant_pk(conn)?;
    migrate_chunk_vectors_tenant_pk(conn)?;
    rebuild_fts_if_needed(conn)?;
    Ok(())
}

fn migrate_pages_tenant_pk(conn: &Connection) -> Result<()> {
    let sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='pages'",
        [],
        |r| r.get(0),
    )?;
    if sql.contains("(tenant_id, slug)") {
        return Ok(());
    }
    conn.execute_batch(
        r#"
        ALTER TABLE pages RENAME TO pages_old;
        CREATE TABLE pages (
            tenant_id TEXT NOT NULL DEFAULT 'default',
            slug TEXT NOT NULL,
            title TEXT NOT NULL,
            page_type TEXT NOT NULL DEFAULT 'note',
            body TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'local',
            deleted INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (tenant_id, slug)
        );
        INSERT INTO pages (tenant_id, slug, title, page_type, body, source, deleted, updated_at)
        SELECT COALESCE(tenant_id, 'default'), slug, title, page_type, body, source, deleted, updated_at FROM pages_old;
        DROP TABLE pages_old;
        DROP TRIGGER IF EXISTS pages_ai;
        DROP TRIGGER IF EXISTS pages_ad;
        DROP TRIGGER IF EXISTS pages_au;
        CREATE TRIGGER pages_ai AFTER INSERT ON pages BEGIN
            INSERT INTO pages_fts(rowid, slug, title, body)
            SELECT new.rowid, new.slug, new.title, new.body WHERE new.deleted = 0;
        END;
        CREATE TRIGGER pages_ad AFTER DELETE ON pages BEGIN
            INSERT INTO pages_fts(pages_fts, rowid, slug, title, body)
            VALUES ('delete', old.rowid, old.slug, old.title, old.body);
        END;
        CREATE TRIGGER pages_au AFTER UPDATE ON pages BEGIN
            INSERT INTO pages_fts(pages_fts, rowid, slug, title, body)
            VALUES ('delete', old.rowid, old.slug, old.title, old.body);
            INSERT INTO pages_fts(rowid, slug, title, body)
            SELECT new.rowid, new.slug, new.title, new.body WHERE new.deleted = 0;
        END;
        "#,
    )?;
    Ok(())
}

fn migrate_chunk_vectors_tenant_pk(conn: &Connection) -> Result<()> {
    let sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='chunk_vectors'",
        [],
        |r| r.get(0),
    )?;
    if sql.contains("(tenant_id, slug, chunk_index)") {
        return Ok(());
    }
    conn.execute_batch(
        r#"
        ALTER TABLE chunk_vectors RENAME TO chunk_vectors_old;
        CREATE TABLE chunk_vectors (
            tenant_id TEXT NOT NULL DEFAULT 'default',
            slug TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            dim INTEGER NOT NULL,
            vector BLOB NOT NULL,
            text_hash TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (tenant_id, slug, chunk_index)
        );
        INSERT INTO chunk_vectors (tenant_id, slug, chunk_index, dim, vector, text_hash, updated_at)
        SELECT COALESCE(tenant_id, 'default'), slug, chunk_index, dim, vector, text_hash, updated_at FROM chunk_vectors_old;
        DROP TABLE chunk_vectors_old;
        "#,
    )?;
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

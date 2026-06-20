//! Toy job queue: SQLite `minion_jobs`, handlers `dream` + `noop`.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::embed::HashEmbedder;
use crate::engine::BrainEngine;
use crate::run_nightly_cycle;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Waiting,
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinionJob {
    pub id: i64,
    pub tenant_id: String,
    pub name: String,
    pub payload: Option<String>,
    pub status: String,
    pub created_at: String,
    pub error: Option<String>,
}

pub fn enqueue(engine: &BrainEngine, name: &str, payload: Option<&str>) -> Result<i64> {
    let conn = engine.conn_for_jobs()?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO minion_jobs (tenant_id, name, payload, status, created_at) VALUES (?1,?2,?3,'waiting',?4)",
        rusqlite::params![engine.tenant_id(), name, payload, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_jobs(engine: &BrainEngine, limit: usize) -> Result<Vec<MinionJob>> {
    let conn = engine.conn_for_jobs()?;
    let mut stmt = conn.prepare(
        "SELECT id, tenant_id, name, payload, status, created_at, error FROM minion_jobs WHERE tenant_id = ?1 ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![engine.tenant_id(), limit as i64], row_to_job)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn work_once(engine: &BrainEngine) -> Result<Option<MinionJob>> {
    work_batch(engine, 1).map(|v| v.into_iter().next())
}

pub fn work_batch(engine: &BrainEngine, max: usize) -> Result<Vec<MinionJob>> {
    let mut done = Vec::new();
    for _ in 0..max {
        let Some(job) = claim_next(engine)? else {
            break;
        };
        let result = run_handler(engine, &job.name);
        match result {
            Ok(()) => {
                finish_job(engine, job.id, true, None)?;
                done.push(job_with_status(engine, job.id)?);
            }
            Err(e) => {
                finish_job(engine, job.id, false, Some(&e.to_string()))?;
                done.push(job_with_status(engine, job.id)?);
            }
        }
    }
    Ok(done)
}

fn claim_next(engine: &BrainEngine) -> Result<Option<MinionJob>> {
    let conn = engine.conn_for_jobs()?;
    let mut stmt = conn.prepare(
        "SELECT id, tenant_id, name, payload, status, created_at, error FROM minion_jobs WHERE tenant_id = ?1 AND status = 'waiting' ORDER BY id ASC LIMIT 1",
    )?;
    let mut rows = stmt.query(rusqlite::params![engine.tenant_id()])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let job = row_to_job(&row)?;
    let now = Utc::now().to_rfc3339();
    let n = conn.execute(
        "UPDATE minion_jobs SET status = 'active', started_at = ?2 WHERE id = ?1 AND status = 'waiting'",
        rusqlite::params![job.id, now],
    )?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(job))
}

fn finish_job(engine: &BrainEngine, id: i64, ok: bool, err: Option<&str>) -> Result<()> {
    let conn = engine.conn_for_jobs()?;
    let status = if ok { "completed" } else { "failed" };
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE minion_jobs SET status = ?2, finished_at = ?3, error = ?4 WHERE id = ?1",
        rusqlite::params![id, status, now, err],
    )?;
    Ok(())
}

fn job_with_status(engine: &BrainEngine, id: i64) -> Result<MinionJob> {
    let conn = engine.conn_for_jobs()?;
    let mut stmt = conn.prepare(
        "SELECT id, tenant_id, name, payload, status, created_at, error FROM minion_jobs WHERE id = ?1",
    )?;
    let mut rows = stmt.query([id])?;
    let row = rows.next()?.context("job row")?;
    Ok(row_to_job(&row)?)
}

fn run_handler(engine: &BrainEngine, name: &str) -> Result<()> {
    match name {
        "dream" => {
            run_nightly_cycle(engine, &HashEmbedder)?;
            Ok(())
        }
        "noop" => Ok(()),
        other => anyhow::bail!("unknown job name: {other} (toy: dream, noop)"),
    }
}

fn row_to_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<MinionJob> {
    Ok(MinionJob {
        id: row.get(0)?,
        tenant_id: row.get(1)?,
        name: row.get(2)?,
        payload: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        error: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn dream_job_runs() {
        let dir = tempdir().unwrap();
        let e = BrainEngine::open_scoped(dir.path().join("b.db"), "team-a").unwrap();
        let id = enqueue(&e, "noop", None).unwrap();
        assert!(id > 0);
        let ran = work_once(&e).unwrap().expect("job");
        assert_eq!(ran.status, "completed");
    }
}
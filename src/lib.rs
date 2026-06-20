//! Local SQLite brain: pages, chunks, typed links, FTS search.
//! Drop-in for OpenClaw/unthinkclaw — no OAuth, no remote serve.

pub mod db;
pub mod engine;
pub mod extract;
pub mod search;
pub mod types;

pub use engine::BrainEngine;
pub use types::{LinkRow, PageRow, SearchHit};

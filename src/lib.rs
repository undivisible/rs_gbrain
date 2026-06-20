//! Local SQLite brain: pages, FTS, graph links, brief, query, dream.
//! gbrain CLI parity (local subset) — no OAuth.

pub mod claw_test;
pub mod db;
pub mod dream;
pub mod engine;
pub mod extract;
pub mod query;
pub mod search;
pub mod types;

pub use engine::BrainEngine;
pub use query::{format_query_markdown, gather_context};
pub use types::{
    BrainStats, BriefState, GraphQueryResult, LinkRow, PageListItem, PageRow, QueryAnswer,
    SearchHit,
};

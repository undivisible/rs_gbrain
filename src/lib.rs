//! Local hybrid RAG brain: typed edges, FTS + vectors, nightly dream.
//! Plugins: OpenClaw SKILL.md, Hermes manifest — see `plugins/`.

pub mod brief_sync;
pub mod claw_test;
pub mod db;
pub mod dream;
pub mod embed;
pub mod engine;
pub mod extract;
pub mod hybrid;
pub mod local_http;
pub mod mcp;
pub mod plugin;
pub mod query;
pub mod search;
pub mod typed_edges;
pub mod types;

pub use brief_sync::sync_workspace_brief;
pub use dream::{run_dream_cycle, run_nightly_cycle, DreamReport};
pub use embed::{Embedder, HashEmbedder};
pub use engine::BrainEngine;
pub use hybrid::HybridConfig;
pub use plugin::{discover_agent_plugins, AgentPluginKind, AgentPluginSpec};
pub use mcp::run_stdio;
pub use query::{format_query_markdown, gather_context, gather_context_with_anchor};
pub use types::{
    BrainStats, BriefState, GraphQueryResult, LinkRow, PageListItem, PageRow, QueryAnswer,
    SearchHit,
};

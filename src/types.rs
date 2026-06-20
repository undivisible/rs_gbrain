use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRow {
    pub slug: String,
    pub title: String,
    pub page_type: String,
    pub body: String,
    pub source: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageListItem {
    pub slug: String,
    pub title: String,
    pub page_type: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRow {
    pub from_slug: String,
    pub to_slug: String,
    pub rel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub slug: String,
    pub score: f32,
    pub snippet: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainStats {
    pub page_count: usize,
    pub link_count: usize,
    pub tag_count: usize,
    pub open_loop_count: usize,
    pub fact_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnswer {
    pub answer: String,
    pub citations: Vec<SearchHit>,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryResult {
    pub anchor: String,
    pub nodes: Vec<String>,
    pub edges: Vec<LinkRow>,
}

#[derive(Debug, Clone)]
pub struct OpenLoop {
    pub id: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefState {
    pub open_loops: Vec<String>,
    pub time_contexts: Vec<String>,
}

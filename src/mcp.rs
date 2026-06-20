//! MCP stdio server (JSON-RPC): read + write brain tools for Claude/Codex.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::{
    format_query_markdown, gather_context_with_anchor, run_nightly_cycle, BrainEngine, HashEmbedder,
};

const SERVER_NAME: &str = "rs_gbrain";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run_stdio(engine: BrainEngine) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line.context("stdin")?;
        if line.trim().is_empty() {
            continue;
        }
        let req: RpcRequest = serde_json::from_str(&line).context("parse rpc")?;
        let resp = dispatch(&engine, req);
        let out = serde_json::to_string(&resp)?;
        writeln!(stdout, "{out}")?;
        stdout.flush()?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

fn ok(id: Value, result: Value) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn err(id: Value, message: impl Into<String>) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(RpcError {
            code: -32000,
            message: message.into(),
        }),
    }
}

fn dispatch(engine: &BrainEngine, req: RpcRequest) -> RpcResponse {
    let id = req.id.unwrap_or(Value::Null);
    let Some(method) = req.method else {
        return err(id, "missing method");
    };
    match method.as_str() {
        "initialize" => ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
            }),
        ),
        "notifications/initialized" | "initialized" => ok(id, json!({})),
        "ping" => ok(id, json!({})),
        "tools/list" => ok(id, json!({ "tools": tool_definitions() })),
        "tools/call" => match call_tool(engine, req.params) {
            Ok(text) => ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false
                }),
            ),
            Err(e) => ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": e.to_string() }],
                    "isError": true
                }),
            ),
        },
        _ => err(id, format!("unknown method: {method}")),
    }
}

fn call_tool(engine: &BrainEngine, params: Option<Value>) -> Result<String> {
    let params = params.context("missing params")?;
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .context("missing tool name")?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        "get_page" => {
            let slug = arg_str(&args, "slug")?;
            match engine.get_page(&slug)? {
                Some(p) => Ok(serde_json::to_string_pretty(&p)?),
                None => Ok(json!({ "error": "not_found", "slug": slug }).to_string()),
            }
        }
        "put_page" => {
            let slug = arg_str(&args, "slug")?;
            let body = arg_str(&args, "body")?;
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(slug.rsplit('/').next().unwrap_or(&slug));
            let page_type = args
                .get("page_type")
                .or_else(|| args.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("note");
            engine.put_page(&slug, title, page_type, &body, "mcp")?;
            Ok(json!({ "ok": true, "slug": slug }).to_string())
        }
        "delete_page" => {
            let slug = arg_str(&args, "slug")?;
            let deleted = engine.delete_page(&slug)?;
            Ok(json!({ "deleted": deleted, "slug": slug }).to_string())
        }
        "list_pages" => {
            let prefix = args.get("prefix").and_then(|v| v.as_str());
            let limit = arg_usize(&args, "limit").unwrap_or(50);
            let pages = engine.list_pages(prefix, limit)?;
            Ok(serde_json::to_string_pretty(&pages)?)
        }
        "search" => {
            let query = arg_str(&args, "query")?;
            let limit = arg_usize(&args, "limit").unwrap_or(10);
            let anchor = args.get("anchor").and_then(|v| v.as_str());
            let hits = if let Some(a) = anchor {
                engine.hybrid_search(&query, limit, Some(a))?
            } else {
                engine.search_with_graph_hint(&query, limit)?
            };
            Ok(serde_json::to_string_pretty(&hits)?)
        }
        "query" | "think" => {
            let question = args
                .get("question")
                .or_else(|| args.get("query"))
                .and_then(|v| v.as_str())
                .context("question or query required")?;
            let limit = arg_usize(&args, "limit").unwrap_or(8);
            let anchor = args.get("anchor").and_then(|v| v.as_str());
            let q = gather_context_with_anchor(engine, question, limit, anchor)?;
            if args.get("markdown").and_then(|v| v.as_bool()).unwrap_or(false) {
                Ok(format_query_markdown(&q))
            } else {
                Ok(serde_json::to_string_pretty(&q)?)
            }
        }
        "graph_query" => {
            let anchor = arg_str(&args, "anchor")?;
            let depth = arg_usize(&args, "depth").unwrap_or(2);
            let rel = args
                .get("rel")
                .or_else(|| args.get("type"))
                .and_then(|v| v.as_str());
            let g = if let Some(r) = rel {
                engine.graph_query_filtered(&anchor, depth, Some(r))?
            } else {
                engine.graph_query(&anchor, depth)?
            };
            Ok(serde_json::to_string_pretty(&g)?)
        }
        "add_link" => {
            let from = arg_str(&args, "from")?;
            let to = arg_str(&args, "to")?;
            let rel = args
                .get("rel")
                .or_else(|| args.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("related_to");
            engine.add_link(&from, &to, rel)?;
            Ok(json!({ "ok": true }).to_string())
        }
        "add_tag" => {
            let slug = arg_str(&args, "slug")?;
            let tag = arg_str(&args, "tag")?;
            engine.add_tag(&slug, &tag)?;
            Ok(json!({ "ok": true }).to_string())
        }
        "get_tags" => {
            let slug = arg_str(&args, "slug")?;
            let tags = engine.get_tags(&slug)?;
            Ok(serde_json::to_string_pretty(&tags)?)
        }
        "stats" | "get_stats" => Ok(serde_json::to_string_pretty(&engine.brain_stats()?)?),
        "dream" => {
            let r = run_nightly_cycle(engine, &HashEmbedder)?;
            Ok(serde_json::to_string_pretty(&r)?)
        }
        "health" => Ok(
            json!({ "ok": true, "name": SERVER_NAME, "version": SERVER_VERSION }).to_string(),
        ),
        other => anyhow::bail!("unknown tool: {other}"),
    }
}

fn arg_str(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .with_context(|| format!("missing string arg: {key}"))
}

fn arg_usize(args: &Value, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as usize)
}

fn tool_definitions() -> Vec<Value> {
    let tools: &[(&str, &str, Value)] = &[
        (
            "get_page",
            "Read a brain page by slug",
            json!({
                "type": "object",
                "properties": { "slug": { "type": "string" } },
                "required": ["slug"]
            }),
        ),
        (
            "put_page",
            "Write or update a brain page (creates typed graph edges from body)",
            json!({
                "type": "object",
                "properties": {
                    "slug": { "type": "string" },
                    "title": { "type": "string" },
                    "body": { "type": "string" },
                    "page_type": { "type": "string" }
                },
                "required": ["slug", "body"]
            }),
        ),
        (
            "delete_page",
            "Soft-delete a page",
            json!({
                "type": "object",
                "properties": { "slug": { "type": "string" } },
                "required": ["slug"]
            }),
        ),
        (
            "list_pages",
            "List pages, optional slug prefix",
            json!({
                "type": "object",
                "properties": {
                    "prefix": { "type": "string" },
                    "limit": { "type": "integer" }
                }
            }),
        ),
        (
            "search",
            "Hybrid search (FTS + vectors + graph). Optional anchor slug boosts neighbors.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer" },
                    "anchor": { "type": "string" }
                },
                "required": ["query"]
            }),
        ),
        (
            "query",
            "Gather context + citations + gaps (host LLM synthesizes prose)",
            json!({
                "type": "object",
                "properties": {
                    "question": { "type": "string" },
                    "limit": { "type": "integer" },
                    "anchor": { "type": "string" },
                    "markdown": { "type": "boolean" }
                },
                "required": ["question"]
            }),
        ),
        (
            "think",
            "Alias of query — retrieval pack, not upstream gbrain LLM synthesis",
            json!({
                "type": "object",
                "properties": {
                    "question": { "type": "string" },
                    "limit": { "type": "integer" },
                    "anchor": { "type": "string" },
                    "markdown": { "type": "boolean" }
                },
                "required": ["question"]
            }),
        ),
        (
            "graph_query",
            "Traverse typed links from anchor",
            json!({
                "type": "object",
                "properties": {
                    "anchor": { "type": "string" },
                    "depth": { "type": "integer" },
                    "rel": { "type": "string" }
                },
                "required": ["anchor"]
            }),
        ),
        (
            "add_link",
            "Add a typed edge",
            json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string" },
                    "to": { "type": "string" },
                    "rel": { "type": "string" }
                },
                "required": ["from", "to"]
            }),
        ),
        (
            "add_tag",
            "Tag a page",
            json!({
                "type": "object",
                "properties": { "slug": { "type": "string" }, "tag": { "type": "string" } },
                "required": ["slug", "tag"]
            }),
        ),
        (
            "get_tags",
            "List tags on a page",
            json!({
                "type": "object",
                "properties": { "slug": { "type": "string" } },
                "required": ["slug"]
            }),
        ),
        (
            "get_stats",
            "Brain counts",
            json!({ "type": "object", "properties": {} }),
        ),
        (
            "dream",
            "Run nightly dream cycle locally",
            json!({ "type": "object", "properties": {} }),
        ),
        (
            "health",
            "Server health",
            json!({ "type": "object", "properties": {} }),
        ),
    ];
    tools
        .iter()
        .map(|(name, desc, schema)| {
            json!({
                "name": name,
                "description": desc,
                "inputSchema": schema
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn mcp_put_and_get_roundtrip() {
        let dir = tempdir().unwrap();
        let e = BrainEngine::open(dir.path().join("b.db")).unwrap();
        let put = call_tool(
            &e,
            Some(json!({
                "name": "put_page",
                "arguments": {
                    "slug": "people/alice",
                    "body": "Founded [[companies/acme]].",
                    "page_type": "person"
                }
            })),
        )
        .unwrap();
        assert!(put.contains("alice"));
        let got = call_tool(
            &e,
            Some(json!({
                "name": "get_page",
                "arguments": { "slug": "people/alice" }
            })),
        )
        .unwrap();
        assert!(got.contains("acme"));
    }
}
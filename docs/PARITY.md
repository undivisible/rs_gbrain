# Parity vs garrytan/gbrain (local)

## CLI aliases (rs_gbrain)

| gbrain | rs_gbrain | Status |
|--------|-----------|--------|
| `put` | `put` | yes (+ stdin) |
| `get` | `get` | yes |
| `list` | `list` | yes |
| `delete` | `delete` | soft-delete |
| `search` | `search` | FTS |
| `query` | `query` | gather + citations |
| `think` | `think` | same as query locally |
| `graph-query` | `graph-query` | BFS on links |
| `link` | `link` | yes |
| `tag` / `tags` | `tag` / `tags` | yes |
| `get_stats` | `stats` | JSON stats |
| `import` | `import` | markdown tree |
| dream cycle | `dream` | open-loop stub |
| `claw-test` | `claw-test` | scripted phases |
| `init` | `init` | yes |
| `smoke` | `smoke` | yes |

## Implemented (this crate)

- Typed edges on write (`works_at`, `reports_to`, `invested_in`, `attended`, `[[slug|rel]]`)
- `graph-query` + `graph_query_filtered(rel)`
- Hybrid RAG: FTS + chunk vectors (HashEmbedder CI; `fastembed` feature for real models)
- Nightly `dream`: hypotheses, orphan links, vector reindex, stale loop close
- Plugins: `plugins/openclaw/SKILL.md`, `plugins/hermes/plugin.json`

## Local + MCP stdio (this repo)

- SQLite file brain, CLI, plugins for OpenClaw/Hermes/unthinkclaw
- `rs_gbrain serve` — MCP tools (read/write), no OAuth
- Optional `rs_gbrain serve --http` with `--features local-http` (127.0.0.1 JSON)

## MCP tools (rs_gbrain serve)

| Tool | Scope |
|------|--------|
| `get_page`, `list_pages`, `search`, `query`, `think`, `graph_query`, `get_tags`, `get_stats`, `health` | read |
| `put_page`, `delete_page`, `add_link`, `add_tag`, `dream` | write |

## Not in scope (use upstream gbrain)

Remote OAuth MCP, PGLite/Postgres fleet, in-process LLM `think`, minions, skillpacks, company RLS, enrich/takes/LSD, etc.

## unthinkclaw tools

`brain_search`, `brain_query`, `brain_put`, `brain_get` when `rs-gbrain` feature on.
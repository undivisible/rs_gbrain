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

## Not in scope (use upstream gbrain or later)

OAuth, `serve --http`, MCP, PGLite/Postgres fleet, embeddings multimodal, minions, skillpacks, company RLS, autopilot, enrich pipelines, takes calibration, LSD, etc.

## unthinkclaw tools

`brain_search`, `brain_query`, `brain_put`, `brain_get` when `rs-gbrain` feature on.
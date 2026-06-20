# rs_gbrain

Local **hybrid RAG** knowledge brain (MPL-2.0): SQLite, FTS5, typed graph edges, chunk embeddings, nightly dream cycle.

**Agent hosts:** OpenClaw (`SKILL.md`), Hermes (`plugin.json`), unthinkclaw (`brain_*` tools).

## Local-first + MCP stdio

- Brain data: `~/.rs_gbrain/brain.db` (`RS_GBRAIN_DB` / `--db`)
- **MCP (read + write):** `rs_gbrain serve` — stdio JSON-RPC tools (`put_page`, `get_page`, `search`, `query`, `think`, `graph_query`, `dream`, …)
- Claude Code: `claude mcp add rs_gbrain -- rs_gbrain serve`
- Optional loopback JSON (not MCP): `cargo build --features local-http` then `rs_gbrain serve --http --bind 127.0.0.1:8787`
- Embeddings: `HashEmbedder` in CI; `--features fastembed` for on-device models
- **Not** upstream: remote OAuth MCP fleet, Postgres/PGLite, company RLS, minions, LLM synthesis inside `think`

## Features

- Typed edges on write: `works_at`, `reports_to`, `invested_in`, `attended`, `founded`, `advises`, `[[slug|rel]]`
- Hybrid search: BM25 + vectors + graph proximity
- `query` / `think`: gather + citations + gaps (`think` is retrieval pack — host LLM synthesizes prose)
- `dream` nightly: open-loop hypotheses, orphan links, vector reindex
- `sync-brief`: `memory/open-loops.md` + `time-contexts.md` → SQLite brief
- Tests: unit + `graph_typed`, `nightly_dream`, `smoke`, `claw-test`

## Quick start

```bash
cargo test
cargo run -- smoke
cargo run -- claw-test
rs_gbrain put people/alice --title Alice --body "CTO at [[companies/acme]]"
rs_gbrain search "acme" --json
rs_gbrain graph-query people/alice --json
rs_gbrain dream
rs_gbrain sync-brief /path/to/workspace
```

DB: `~/.rs_gbrain/brain.db` (`RS_GBRAIN_DB` / `--db`).

## Plugins

| Host | Path |
|------|------|
| OpenClaw | `plugins/openclaw/SKILL.md` or workspace `plugins/rs_gbrain/SKILL.md` |
| Hermes | `plugins/hermes/plugin.json` |
| unthinkclaw | feature `rs-gbrain`, config `[rs_gbrain]` |

unthinkclaw also scans `plugins/`, `.openclaw/plugins`, `.hermes/plugins` at startup.

## GitHub metadata

```bash
./scripts/github-metadata.sh   # needs gh auth
```

## License

Mozilla Public License 2.0 — see `LICENSE`.
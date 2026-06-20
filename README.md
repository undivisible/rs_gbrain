# rs_gbrain

Local **hybrid RAG** knowledge brain (MPL-2.0): SQLite, FTS5, typed graph edges, chunk embeddings, nightly dream cycle.

**Agent hosts:** OpenClaw (`SKILL.md`), Hermes (`plugin.json`), unthinkclaw (`brain_*` tools).

## Local only (no remote MCP)

This project is **local-first**:

- Brain data stays in `~/.rs_gbrain/brain.db` (or `RS_GBRAIN_DB`)
- **No** gbrain-style remote `serve --http` + OAuth MCP fleet
- Optional **local** JSON API: `cargo build --features local-http` then `rs_gbrain serve --bind 127.0.0.1:8787` — loopback only, not MCP
- Embeddings: `HashEmbedder` in CI; `--features fastembed` for on-device models

Remote MCP / multi-tenant cloud brain is **out of scope** until explicitly added.

## Features

- Typed edges on write: `works_at`, `reports_to`, `invested_in`, `attended`, `[[slug|rel]]`
- Hybrid search: BM25 + vectors + graph proximity
- `query` / `think`: gather + citations + gaps (agent LLM does final synthesis)
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
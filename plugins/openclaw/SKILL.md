---
name: rs_gbrain
description: Hybrid RAG brain (FTS + embeddings + typed graph). Works in OpenClaw, Hermes, and unthinkclaw.
---

# rs_gbrain (OpenClaw)

## Rules

1. Before questions about people, companies, deals, or strategy → run `rs_gbrain search "<query>" --json`.
2. To persist a correction or fact → `rs_gbrain put <slug> --title "..." --body "..."` (use `[[other/slug]]` for links).
3. DB: `~/.rs_gbrain/brain.db` unless `RS_GBRAIN_DB` is set.

## Commands

```bash
rs_gbrain search "Alice Acme" --json
rs_gbrain get people/alice
rs_gbrain put people/alice --file ./brain/people/alice.md
rs_gbrain import ./brain
```

Build from repo: `cargo install --path .` or `cargo build --release` and put `target/release/rs_gbrain` on PATH.
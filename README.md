# rs_gbrain

Local **SQLite** personal knowledge brain — gbrain-shaped, **no OAuth**, no remote serve.

- Pages + FTS search
- `[[wikilink]]` → typed `links_to` edges (graph-lite)
- CLI: `init`, `put`, `get`, `search`, `import`, `smoke`

## Quick start

```bash
cargo build --release
cargo run -- smoke
rs_gbrain put people/alice --title Alice --body "Knows [[companies/acme]]"
rs_gbrain search "acme" --json
```

DB default: `~/.rs_gbrain/brain.db` (override `RS_GBRAIN_DB` or `--db`).

## OpenClaw drop-in

Copy or symlink `plugins/openclaw/` into your OpenClaw workspace skills, or point agents at `SKILL.md`.

## unthinkclaw

```bash
ln -sf ../rs_gbrain /path/to/unthinkclaw/vendor/rs_gbrain
```

Add path dependency in `unthinkclaw/Cargo.toml` when wiring tools (see `docs/integrate-unthinkclaw.md`).

## vs garrytan/gbrain

This is a **weekend-scope** Rust core: SQLite + FTS + links. No dream cycle, synthesis `think`, or Postgres fleet. Grow incrementally.

## CI

`cargo fmt --check`, `clippy -D warnings`, `test`, `smoke` — mirrors gbrain’s “green gate” spirit without Bun.
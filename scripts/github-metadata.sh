#!/usr/bin/env bash
# Set GitHub repo description + topics (requires gh auth).
set -euo pipefail
gh repo edit undivisible/rs_gbrain \
  --description "Local-first hybrid RAG knowledge brain: SQLite, FTS5, typed graph edges, embeddings, dream cycle — OpenClaw & Hermes plugins" \
  --add-topic knowledge-graph --add-topic rag --add-topic sqlite \
  --add-topic embeddings --add-topic openclaw --add-topic hermes \
  --add-topic agent-memory --add-topic fts5 --add-topic gbrain --add-topic rust
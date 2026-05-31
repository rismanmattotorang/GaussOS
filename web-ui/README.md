# GaussOS Web UI

A small Deno server that serves the GaussOS dashboard and proxies the REST API.
Its purpose is simple: **let anyone test the agent memory in a browser** — store
memories, then see exactly *why* each one is retrieved.

## Run

```bash
# 1) Start the GaussOS API (from the repo root)
cargo run --release --features cli-bin --bin gaussos -- server --port 8080

# 2) Start the web UI (this folder)
deno task dev          # or: ./start.sh
#   UI:  http://localhost:3000   (proxies to BACKEND_URL, default http://localhost:8080)
```

Environment: `PORT` (default `3000`), `BACKEND_URL` (default `http://localhost:8080`).

## Pages

- **Dashboard** — live counts (memories, facts, vectors) and the active LLM
  provider, all read from `/api/v1/metrics` and `/api/v1/llm/status`. Plus a
  one-click "Seed sample data" to get started.
- **Retrieval Playground** — the centerpiece. Add a memory (or seed samples),
  run a query, and compare **lexical (BM25)** vs **vector** vs **hybrid (RRF)**
  ranking side by side, each with its full score breakdown
  (`bm25`, `vector`, `recency`, rank). White-box retrieval no other
  agent-memory UI exposes. Backed by `POST /api/v1/retrieval/compare`.
- **Memories** — faceted browser over the store (text, namespace, type, quality)
  via `POST /api/v1/memories/search`.
- **Knowledge Graph** — the bi-temporal entity graph; drag the time slider to
  see it *as it was valid* at any instant, click a node to trace multi-hop
  relevance (Personalized PageRank). Backed by `/api/v1/facts/graph`.
- **Settings** — LLM provider status and a manual forgetting-pass control.

## Honesty

The UI never fabricates data. Every number comes from the backend; if the API
is unreachable the proxy returns `503 backend_unavailable` rather than mock
values, and the dashboard shows `—`.

## Files

- `main.ts` — the server: static files, API proxy (with retry), metrics SSE, and
  the HTML shell.
- `static/app.js` — the client app (navigation, playground, graph viewer, etc.).
- `static/styles.css`, `static/themes.css` — styling and themes.

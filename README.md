<div align="center">

# 🧠 GaussOS

### Long-term memory for AI agents — complete, correct, and fast.

A single Rust binary that unifies temporal knowledge graphs, hybrid retrieval,
and cognitive forgetting — with white-box scoring you can actually inspect.

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-142%20passing-brightgreen.svg)](#building--testing)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Made by Gaussian Technologies](https://img.shields.io/badge/by-Gaussian%20Technologies-8b5cf6.svg)](#about-gaussian-technologies)

[Quick start](#quick-start) · [Concepts](#concepts) · [API](#core-api) · [Architecture](#architecture) · [Docs](#documentation)

</div>

---

## Overview

AI agents are only as good as what they remember. Today that means stitching
together several systems — Zep for temporal graphs, Mem0 for LLM-driven
consolidation, Letta for tiered memory, a vector database for search — in Python,
behind a cluster you have to operate.

**GaussOS unifies them into one engine.** It runs as a single compiled binary
with a REST/streaming API, a web dashboard, and a native terminal UI. No GC
pauses, no Python glue, no external services required.

- **Bi-temporal knowledge graph** — facts are *superseded*, never silently deleted, so you can query the graph *as it was valid* at any past instant.
- **Hybrid retrieval** — BM25 + dense vectors fused with Reciprocal Rank Fusion, diversified by MMR, decayed by recency.
- **In-engine vector index** — a from-scratch HNSW index with scalar/binary quantization; no separate vector DB.
- **Multi-hop graph retrieval** — Personalized PageRank over the entity graph (HippoRAG-style).
- **Cognitive forgetting** — an Ebbinghaus retention curve with salience scoring keeps memory bounded.
- **Provider-agnostic LLM layer** — OpenAI, Anthropic, DeepSeek, Qwen, BytePlus, OpenRouter, or any local OpenAI-compatible model.
- **White-box by design** — every retrieval result carries its full score breakdown. No opaque ranking.

## Quick start

```bash
git clone https://github.com/gaussos/gaussos.git
cd gaussos

# Run the API server — embedded SurrealDB, no external database required
cargo run --release --features cli-bin --bin gaussos -- server --port 8080
```

Prefer Docker? The API and web dashboard come up together:

```bash
docker compose up --build
#  API:  http://localhost:8080
#  UI:   http://localhost:3000   ← open the Retrieval Playground to test it live
```

Store a memory, then retrieve it:

```bash
# Store
curl -X POST http://localhost:8080/api/v1/memories \
  -H 'content-type: application/json' \
  -d '{"payload": {"Text": "Alice prefers dark roast coffee"},
       "tags": ["preference"], "namespace": "users/alice"}'

# Hybrid search — BM25 + vector + RRF + MMR
curl -X POST http://localhost:8080/api/v1/memories/search \
  -H 'content-type: application/json' \
  -d '{"text": "what coffee does alice like?", "namespace": "users/alice"}'
```

## Concepts

| Concept | What it is |
|---|---|
| **MemCube** | The unit of memory: a typed payload (text, semantic, episodic, procedural, parametric, …) with metadata, tags, quality, and timestamps. |
| **Namespace** | A hierarchical scope like `users/alice/work`. Retrieval, access policies, and forgetting all operate per-namespace and its descendants. |
| **Fact** | A `(subject, predicate, object)` triple in the bi-temporal graph. Conflicting facts supersede — old versions are retained for audit and time-travel. |
| **Retrieval** | A query runs through lexical (BM25), vector, and fused hybrid rankers; results return with per-signal scores and ranks. |

## How it compares

| Capability | GaussOS | Zep | Letta | Mem0 |
|---|:--:|:--:|:--:|:--:|
| Bi-temporal knowledge graph (supersede, not delete) | ✅ | ✅ | ❌ | ❌ |
| Hybrid BM25 + vector + **RRF** + **MMR** | ✅ | ✅ | ❌ | 🟡 |
| In-engine **HNSW** + vector quantization | ✅ | ❌ | ❌ | ❌ |
| Multi-hop **Personalized PageRank** retrieval | ✅ | 🟡 | ❌ | 🟡 |
| Cognitive **forgetting curve** + salience scoring | ✅ | ❌ | 🟡 | ❌ |
| Multi-provider LLM (6 vendors + local) | ✅ | 🟡 | ✅ | ✅ |
| Web dashboard **+** native TUI | ✅ | ✅ | ✅ | 🟡 |
| Single compiled binary, runs fully offline | ✅ | ❌ | 🟡 | 🟡 |

<sub>✅ implemented · 🟡 partial · ❌ not offered. Full matrix with code references in **[BENCHMARK.md](BENCHMARK.md)**.</sub>

> **Honesty first.** Every capability above maps to real, tested code. We do not
> publish accuracy leaderboard numbers we haven't reproduced — standing up
> LoCoMo / LongMemEval with reproducible scripts is the top [roadmap](ROADMAP.md) item.

## Core API

| Endpoint | Purpose |
|---|---|
| `POST /api/v1/memories` | Store a memory |
| `POST /api/v1/memories/search` | Hybrid retrieval (BM25 + vector + RRF + MMR) |
| `POST /api/v1/retrieval/compare` | Side-by-side scoring across lexical / vector / hybrid |
| `POST /api/v1/facts` · `GET /api/v1/facts/graph` | Bi-temporal fact graph with point-in-time queries |
| `POST /api/v1/admin/forget` | Run a forgetting pass (Ebbinghaus decay + salience) |
| `GET /api/v1/admin/export` · `POST .../import` | Full snapshot backup / restore |
| `GET /api/v1/llm/status` | Active LLM provider, or honest `llm_not_configured` |
| `GET /metrics/prometheus` | Prometheus metrics |

## LLM providers

The agent layer speaks two wire protocols — the **Anthropic Messages API** and the
**OpenAI-compatible Chat Completions API** — and ships presets for the major
vendors. Selection is by environment variable; no code changes, no rebuilds.

| Provider | `LLM_PROVIDER` | Key env |
|---|---|---|
| OpenAI (GPT) | `openai` | `OPENAI_API_KEY` |
| Anthropic (Claude) | `anthropic` | `ANTHROPIC_API_KEY` |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` |
| Qwen (DashScope) | `qwen` | `DASHSCOPE_API_KEY` |
| BytePlus (ModelArk) | `byteplus` | `BYTEPLUS_API_KEY` |
| OpenRouter | `openrouter` | `OPENROUTER_API_KEY` |
| Any OpenAI-compatible (Ollama, vLLM, LM Studio) | `custom` | `LLM_BASE_URL` + `LLM_API_KEY` |

```bash
export LLM_PROVIDER=openai OPENAI_API_KEY=sk-...                                      # hosted
export LLM_PROVIDER=custom LLM_BASE_URL=http://localhost:11434/v1 LLM_MODEL=llama3.1  # local
```

Leave `LLM_PROVIDER` unset and GaussOS auto-selects the first provider whose key
is present. With no key, it reports `llm_not_configured` instead of fabricating a
response. See [`.env.example`](.env.example) and [`src/agents/llm.rs`](src/agents/llm.rs).

## Architecture

```
   API & streaming  (axum REST · SSE · WebSocket)   +   Web dashboard · native TUI
 ──────────────────────────────────────────────────────────────────────────────────
   Memory engine
     • MemCubes — 7 payload types (semantic, episodic, procedural, …)
     • Hybrid retrieval — BM25 + dense vectors → RRF → MMR → temporal decay
     • In-engine HNSW ANN index + scalar/binary vector quantization
     • Bi-temporal knowledge graph (valid-time + transaction-time)
     • Multi-hop Personalized PageRank · community detection
     • LLM extract→update ingestion · forgetting curve · salience scoring
 ──────────────────────────────────────────────────────────────────────────────────
   Storage   embedded SurrealDB (in-memory or RocksDB on disk) · pluggable backends
   Ops       Prometheus · OpenTelemetry/OTLP traces · audit log · RLS · AES-GCM
```

## Operations & security

- **Persistence** — embedded SurrealDB runs in-memory by default; set `GAUSSOS_SURREAL_PATH` to persist to disk via RocksDB.
- **Observability** — a Prometheus endpoint plus an OTLP trace exporter that activates on `OTEL_EXPORTER_OTLP_ENDPOINT`. Grafana dashboard and Prometheus config under [`deploy/`](deploy/).
- **Security** — tamper-evident audit log (hash chain), per-namespace row-level security (default-deny), and feature-gated AES-256-GCM field encryption with key rotation (`--features encryption`).
- **Distributed** — consistent-hash sharding with virtual nodes ([`src/database/cluster.rs`](src/database/cluster.rs)).

## Deployment

- **Docker** — multi-stage `Dockerfile` (slim, non-root, healthchecked) and `docker compose` with an opt-in `observability` profile (Prometheus + Grafana).
- **Kubernetes** — a Helm chart at [`deploy/helm/gaussos/`](deploy/helm/gaussos/): Deployment, Service, PVC for the RocksDB store, HPA, and a ServiceMonitor.

## SDKs

Thin clients so Mem0 / Zep / Letta users can migrate with minimal friction:

- **Python** — [`sdk/python/gaussos.py`](sdk/python/gaussos.py) (standard library only)
- **TypeScript** — [`sdk/typescript/gaussos.ts`](sdk/typescript/gaussos.ts) (fetch-based)

## Documentation

| Document | Contents |
|---|---|
| [AGENT_MEMORY.md](AGENT_MEMORY.md) | The memory engine, explained in depth |
| [BENCHMARK.md](BENCHMARK.md) | Full capability comparison vs the field |
| [ROADMAP.md](ROADMAP.md) | What's shipped and what's next |
| [RESEARCH.md](RESEARCH.md) | The research GaussOS draws on |

## Building & testing

```bash
cargo test --lib                        # 142 tests
cargo test --lib --features encryption  # + field-encryption tests
cargo run --release --features tui-bin --bin gaussos-tui   # terminal cockpit
```

## License

Released under the [MIT License](LICENSE).

## About Gaussian Technologies

GaussOS is built by **Gaussian Technologies** 🇮🇩, an Indonesian deep-tech startup
building world-class AI infrastructure from Indonesia for the world. We believe
agent memory should be **open, fast, correct, and white-box** — every retrieval
decision inspectable, every fact auditable, every byte safe.

<div align="center"><sub>Built with care in Indonesia, on the shoulders of the Rust community.</sub></div>

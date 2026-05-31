# 🧠 GaussOS

**Long-term memory for AI agents — complete, correct, and fast. One Rust binary.**

*Built by [Gaussian Technologies](#about-gaussian-technologies), an Indonesian deep-tech startup.* 🇮🇩

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-142%20passing-brightgreen.svg)](#)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## What is GaussOS?

AI agents are only as good as what they remember. Today you have to choose:
Zep for temporal graphs, Mem0 for LLM-driven consolidation, Letta for tiered
memory, a separate vector DB for search — then glue them together in Python and
operate a cluster.

GaussOS brings those ideas together into **a single, offline-capable, type-safe
engine** with a REST/streaming API, a web dashboard, and a native terminal UI.
No GC pauses, no Python glue, no required external services.

It pairs a **bi-temporal knowledge graph** (facts are *superseded*, never
silently deleted) with **hybrid retrieval** (BM25 + dense vectors fused by RRF,
diversified by MMR), an **in-engine HNSW index** with vector quantization,
**multi-hop graph retrieval** (Personalized PageRank), a **cognitive forgetting
curve**, and a **pluggable multi-provider LLM layer**.

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

<sub>✅ implemented · 🟡 partial · ❌ not offered. Full matrix with code
references in **[BENCHMARK.md](BENCHMARK.md)**.</sub>

> **Honesty first.** Every capability claim above maps to real, tested code.
> We don't publish accuracy leaderboard numbers we haven't reproduced —
> standing up LoCoMo/LongMemEval with reproducible scripts is the top
> [roadmap](ROADMAP.md) item.

## Quick start

```bash
git clone https://github.com/gaussos/gaussos.git
cd gaussos

# Run the API server (embedded SurrealDB — no external database needed)
cargo run --release --features cli-bin --bin gaussos -- server --port 8080
```

Or with Docker — API + web dashboard in one command:

```bash
docker compose up --build
#  API:  http://localhost:8080
#  UI:   http://localhost:3000
```

Then store and retrieve a memory over the REST API:

```bash
# Store
curl -X POST http://localhost:8080/api/v1/memories \
  -H 'content-type: application/json' \
  -d '{"payload": {"Text": "Alice prefers dark roast coffee"},
       "tags": ["preference"], "namespace": "users/alice"}'

# Hybrid search (BM25 + vector + RRF + MMR)
curl -X POST http://localhost:8080/api/v1/memories/search \
  -H 'content-type: application/json' \
  -d '{"text": "what coffee does alice like?", "namespace": "users/alice"}'
```

## Core API

| Endpoint | What it does |
|---|---|
| `POST /api/v1/memories` | Store a memory |
| `POST /api/v1/memories/search` | Hybrid retrieval (BM25 + vector + RRF + MMR) |
| `POST /api/v1/retrieval/compare` | Side-by-side scoring across retrieval strategies |
| `POST /api/v1/facts` · `GET /api/v1/facts/graph` | Bi-temporal fact graph (point-in-time queries) |
| `POST /api/v1/admin/forget` | Run a forgetting pass (Ebbinghaus decay + salience) |
| `GET  /api/v1/admin/export` · `POST .../import` | Full snapshot backup / restore |
| `GET  /api/v1/llm/status` | Active LLM provider, or honest `llm_not_configured` |
| `GET  /metrics/prometheus` | Prometheus metrics |

Every retrieval result carries an **inspectable score breakdown** — no black-box
ranking.

## LLM providers

The agent layer is **provider-agnostic**: it speaks the Anthropic Messages API
and the OpenAI-compatible Chat Completions API, and ships presets for the major
vendors. Selection is by environment variable — no code changes, no rebuilds.

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
export LLM_PROVIDER=openai OPENAI_API_KEY=sk-...      # GPT
export LLM_PROVIDER=custom LLM_BASE_URL=http://localhost:11434/v1 LLM_MODEL=llama3.1  # local
```

Leave `LLM_PROVIDER` unset and GaussOS auto-selects the first provider whose key
is present. With no key, it returns an honest `llm_not_configured` status rather
than fabricating a response. See [`.env.example`](.env.example) and
[`src/agents/llm.rs`](src/agents/llm.rs).

## Architecture

```
   API / streaming (axum REST · SSE · WebSocket)   +   Web dashboard · native TUI
 ────────────────────────────────────────────────────────────────────────────────
   Memory engine
     • MemCubes (7 payload types: semantic, episodic, procedural, …)
     • Hybrid retrieval: BM25 + dense vectors → RRF → MMR → temporal decay
     • In-engine HNSW ANN index + scalar/binary vector quantization
     • Bi-temporal knowledge graph (valid-time + transaction-time)
     • Multi-hop Personalized PageRank · community detection
     • LLM extract→update ingestion · forgetting curve · salience scoring
 ────────────────────────────────────────────────────────────────────────────────
   Storage: embedded SurrealDB (in-memory or RocksDB on disk) · pluggable backends
   Ops:     Prometheus metrics · OpenTelemetry/OTLP traces · audit log · RLS · AES-GCM
```

## Operations & security

- **Persistence** — embedded SurrealDB runs in-memory by default; set
  `GAUSSOS_SURREAL_PATH` to persist to disk (RocksDB).
- **Observability** — Prometheus endpoint plus an OTLP trace exporter that
  activates on `OTEL_EXPORTER_OTLP_ENDPOINT`. Grafana dashboard and Prometheus
  config under [`deploy/`](deploy/).
- **Security** — tamper-evident audit log (hash chain), per-namespace row-level
  security (default-deny), and feature-gated AES-256-GCM field encryption with
  key rotation (`--features encryption`).
- **Deploy** — multi-stage `Dockerfile`, `docker compose` (with an opt-in
  `observability` profile), and a Helm chart at
  [`deploy/helm/gaussos/`](deploy/helm/gaussos/).
- **Distributed** — consistent-hash sharding with virtual nodes
  ([`src/database/cluster.rs`](src/database/cluster.rs)).

## SDKs

Thin clients so Mem0/Zep/Letta users can migrate easily:

- **Python** — [`sdk/python/gaussos.py`](sdk/python/gaussos.py) (stdlib only)
- **TypeScript** — [`sdk/typescript/gaussos.ts`](sdk/typescript/gaussos.ts) (fetch-based)

## Documentation

- 🧠 [AGENT_MEMORY.md](AGENT_MEMORY.md) — the memory engine, explained
- 📊 [BENCHMARK.md](BENCHMARK.md) — full capability comparison vs the field
- 🗺️ [ROADMAP.md](ROADMAP.md) — what's done and what's next
- 🔬 [RESEARCH.md](RESEARCH.md) — the research GaussOS draws on

## Building & testing

```bash
cargo test --lib                        # 142 tests
cargo test --lib --features encryption  # + field-encryption tests
cargo run --release --features tui-bin --bin gaussos-tui   # terminal cockpit
```

## License

MIT — see [LICENSE](LICENSE).

## About Gaussian Technologies

GaussOS is built by **Gaussian Technologies**, an Indonesian deep-tech startup
on a mission to build world-class AI infrastructure from Indonesia for the
world. We believe agent memory should be **open, fast, correct, and white-box** —
every retrieval decision inspectable, every fact auditable, every byte safe.

*Built with ❤️ in Indonesia, on the shoulders of the Rust community.*

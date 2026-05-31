# GaussOS — Capability Benchmark vs TencentDB, Zep, Letta, Mem0

*By Gaussian Technologies — an Indonesian deep‑tech startup.*

This document benchmarks **GaussOS** against the leading agent‑memory systems on
**capabilities, features, algorithms, and engineering substrate**. It is an
honest, evidence‑based comparison: every "✅" for GaussOS maps to code in this
repository (file references given), and the **Maturity & honesty** section
states plainly what is *not* yet proven (e.g. published accuracy scores).

> Scope note. The public agent‑memory leaderboards (LoCoMo, LongMemEval) are
> dominated by **vendor self‑reported, LLM‑as‑judge numbers that competing
> vendors cannot reproduce for each other** (documented disputes between Mem0
> and Zep; the independent *MemoryAgentBench*, arXiv 2507.05257, shows all
> systems still fail multi‑hop conflict resolution). We therefore do **not**
> publish head‑to‑head accuracy numbers we have not independently run. This is a
> *capability/feature* benchmark; running LoCoMo/LongMemEval is a tracked
> roadmap item (see [`ROADMAP.md`](ROADMAP.md)).

---

## 1. Systems compared

| System | Language | Storage | Core idea |
|---|---|---|---|
| **GaussOS** | **Rust** (compiled, no GC) | Embedded SurrealDB / Postgres / in‑memory, pluggable | Unified memory engine: bi‑temporal graph + hybrid/ANN/graph retrieval + forgetting, in one binary |
| TencentDB‑Agent‑Memory | Python | SQLite + sqlite‑vec | Layered (L0→L3) memory + Mermaid symbolic canvas + RRF hybrid |
| Zep / Graphiti | Python / TypeScript | Neo4j | Bi‑temporal knowledge graph + hybrid search + community summaries |
| Letta / MemGPT | Python | Postgres / vector DB | Self‑editing memory blocks, tiered context, sleep‑time compute |
| Mem0 | Python / TypeScript | Vector + optional graph | LLM extract → ADD/UPDATE/DELETE/NOOP consolidation |

---

## 2. Feature matrix

Legend: ✅ implemented · 🟡 partial / basic · 🧭 on roadmap · ❌ not offered · n/a not applicable

### Retrieval & ranking
| Capability | GaussOS | TencentDB | Zep | Letta | Mem0 |
|---|:--:|:--:|:--:|:--:|:--:|
| Lexical BM25 | ✅ `memory/retrieval.rs` | ✅ | ✅ | 🟡 | 🟡 |
| Dense vector search | ✅ `database/query.rs` (5 metrics) | ✅ | ✅ | ✅ | ✅ |
| Reciprocal Rank Fusion (hybrid) | ✅ | ✅ | ✅ | ❌ | 🟡 |
| MMR diversity re‑rank | ✅ | 🟡 | 🟡 | ❌ | ❌ |
| ANN index (HNSW) in‑engine | ✅ `memory/ann/hnsw.rs` (from scratch) | 🟡 (sqlite‑vec) | via Neo4j | via vector DB | via vector DB |
| Vector quantization (scalar + binary) | ✅ `memory/ann/quantization.rs` | ❌ | ❌ | ❌ | ❌ |
| Multi‑hop graph retrieval (Personalized PageRank) | ✅ `memory/graph_retrieval.rs` (HippoRAG‑style) | ❌ | 🟡 (graph traversal) | ❌ | 🟡 (graph) |
| Cross‑encoder reranking | 🧭 | ❌ | ✅ | ❌ | 🟡 |

### Memory model & cognition
| Capability | GaussOS | TencentDB | Zep | Letta | Mem0 |
|---|:--:|:--:|:--:|:--:|:--:|
| Bi‑temporal facts (valid + transaction time) | ✅ `memory/temporal.rs` (4 timestamps) | ❌ | ✅ | ❌ | ❌ |
| Conflict = supersede, not delete (audit history) | ✅ | ❌ | ✅ | 🟡 | 🟡 (overwrite) |
| Point‑in‑time recall | ✅ | ❌ | ✅ | ❌ | ❌ |
| Forgetting curve (Ebbinghaus + reinforcement) | ✅ `memory/decay.rs` | ❌ | ❌ | 🟡 (sleep‑time) | ❌ |
| Generative‑Agents scorer (recency·importance·relevance) | ✅ `memory/scoring.rs` | ❌ | ❌ | ❌ | ❌ |
| Hierarchical layering (L0→L3 progressive disclosure) | ✅ `memory/hierarchy.rs` | ✅ | 🟡 (3‑tier graph) | 🟡 (tiers) | ❌ |
| Episodic / semantic / procedural types (CoALA) | ✅ `core::MemoryPayload` | 🟡 | 🟡 | ✅ | 🟡 |
| LLM extract→update ingestion loop | 🟡 (schema + multi‑provider client) | ✅ | ✅ | ✅ | ✅ |
| Self‑editing memory blocks | 🧭 | ❌ | ❌ | ✅ | ❌ |
| Community summaries (Leiden/label‑prop) | 🧭 | ❌ | ✅ | ❌ | ❌ |
| Symbolic task canvas (Mermaid) | 🧭 | ✅ | ❌ | ❌ | ❌ |
| Autonomous sleep‑time consolidation agent | 🟡 (forgetting/consolidation pass) | ❌ | ❌ | ✅ | ❌ |

### LLM & integration
| Capability | GaussOS | TencentDB | Zep | Letta | Mem0 |
|---|:--:|:--:|:--:|:--:|:--:|
| Multi‑provider LLM out of the box | ✅ OpenAI · DeepSeek · Qwen · BytePlus · OpenRouter · Anthropic · custom (`agents/llm.rs`) | 🟡 | 🟡 | ✅ | ✅ |
| OpenAI‑compatible + Anthropic protocols | ✅ | 🟡 | 🟡 | ✅ | ✅ |
| Local model support (Ollama/vLLM via `custom`) | ✅ | 🟡 | 🟡 | ✅ | 🟡 |

### Interfaces & operations
| Capability | GaussOS | TencentDB | Zep | Letta | Mem0 |
|---|:--:|:--:|:--:|:--:|:--:|
| REST API | ✅ | ✅ | ✅ | ✅ | ✅ |
| SSE streaming + WebSocket | ✅ | 🟡 | 🟡 | 🟡 | 🟡 |
| GraphQL / gRPC (optional) | ✅ (feature‑gated) | ❌ | ❌ | ❌ | ❌ |
| Web dashboard (live metrics) | ✅ `web-ui/` (real SSE) | 🟡 | ✅ (cloud) | ✅ | 🟡 |
| Terminal UI (TUI) | ✅ `src/tui/` (ratatui) | ❌ | ❌ | ❌ | ❌ |
| AuthN/Z (JWT, OAuth2, API keys, MFA, RBAC) | ✅ `src/auth/` | 🟡 | ✅ (cloud) | 🟡 | 🟡 |
| Real host metrics (sysinfo) | ✅ | n/a | n/a | n/a | n/a |

### Engineering substrate
| Property | GaussOS | TencentDB | Zep | Letta | Mem0 |
|---|:--:|:--:|:--:|:--:|:--:|
| Implementation language | **Rust** | Python | Py/TS | Python | Py/TS |
| Memory safety w/o GC | ✅ | ❌ | ❌ | ❌ | ❌ |
| Lock‑free hot paths (DashMap/atomics) | ✅ | ❌ | ❌ | ❌ | ❌ |
| SIMD vector ops | ✅ `performance/simd.rs` | ❌ | ❌ | ❌ | ❌ |
| Runs fully offline, single binary | ✅ (embedded SurrealDB) | ✅ (SQLite) | ❌ (Neo4j) | 🟡 | 🟡 |
| Unit/integration test suite | ✅ 87 lib + integration tests | 🟡 | ✅ | ✅ | ✅ |

---

## 3. Where GaussOS is already ahead

1. **Breadth in one engine.** No single competitor combines a **bi‑temporal
   knowledge graph (Zep) + RRF hybrid retrieval & L0→L3 layering (TencentDB) +
   HippoRAG‑style PPR multi‑hop + a cognitive forgetting curve + an in‑engine
   HNSW index with quantization**. GaussOS does, behind one API.
2. **Compiled‑Rust substrate.** Memory safety without GC, lock‑free hot paths,
   and SIMD — a fundamentally faster, more predictable foundation than the
   Python/TS competitors.
3. **Targets the unsolved frontier.** *MemoryAgentBench* shows every system fails
   multi‑hop conflict resolution; GaussOS pairs **bi‑temporal supersession**
   (correct conflict handling) with **PPR multi‑hop retrieval** — exactly the
   two levers that frontier needs.
4. **Truly pluggable LLMs.** OpenAI, DeepSeek, Qwen, BytePlus, OpenRouter,
   Anthropic, or any OpenAI‑compatible/local endpoint — switched by env var.
5. **Offline, single binary.** Embedded SurrealDB means no Neo4j/cluster to run.
6. **Both a Web dashboard and a native TUI** — operators get a GUI *and* a
   keyboard‑driven terminal cockpit.

## 4. Maturity & honesty (where we must still win)

- **No published accuracy benchmarks yet.** We have not run LoCoMo/LongMemEval
  end‑to‑end with a live LLM. Until we do, we make **capability**, not accuracy,
  claims. (Roadmap item #1.)
- **LLM extract→update ingestion** is schema‑based today; the Mem0/Zep‑style
  LLM‑driven extraction loop using our multi‑provider client is partially wired
  (Roadmap item #2).
- **Community summaries, self‑editing memory blocks, symbolic canvas, and a
  fully autonomous sleep‑time agent** are designed but not yet implemented
  (Roadmap items #3–#6).
- **Web UI/TUI polish** is good but pre‑1.0; see the UX roadmap.

The plan to close every gap and extend the lead is in [`ROADMAP.md`](ROADMAP.md).
